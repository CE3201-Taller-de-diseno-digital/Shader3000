use super::Context;
use crate::{
    arch::{Emitter, Register},
    ir::Local,
};
use std::io;

pub struct Allocations<'a, E: Emitter<'a>>(Vec<Slot<E::Register>>);

struct Slot<R: Register> {
    reg: R,
    entry: Option<Entry>,
}

struct Entry {
    local: Local,
    dirty: bool,
}

impl<'a, E: Emitter<'a>> Context<'a, E> {
    pub fn spill(&self, regs: &mut Allocations<'a, E>) -> io::Result<()> {
        for slot in regs.0.iter_mut() {
            let reg = slot.reg;

            if let Some(entry) = &mut slot.entry {
                if entry.dirty {
                    E::reg_to_local(self, reg, entry.local)?;
                    entry.dirty = false;
                }
            }
        }

        Ok(())
    }

    pub fn clear(&self, regs: &mut Allocations<'a, E>) -> io::Result<()> {
        self.spill(regs)?;
        for slot in regs.0.iter_mut() {
            slot.entry = None;
        }

        Ok(())
    }

    pub fn read(&self, regs: &mut Allocations<'a, E>, local: Local) -> io::Result<E::Register> {
        for slot in regs.0.iter() {
            match &slot.entry {
                Some(entry) if entry.local == local => return Ok(slot.reg),
                _ => (),
            }
        }

        let entry = Some(Entry {
            local,
            dirty: false,
        });

        let slot = self.take_slot(regs, &[], entry)?;
        E::local_to_reg(self, local, slot.reg)?;

        Ok(slot.reg)
    }

    pub fn write(&self, regs: &mut Allocations<'a, E>, local: Local) -> io::Result<E::Register> {
        for slot in regs.0.iter_mut() {
            let reg = slot.reg;

            match &mut slot.entry {
                Some(entry) if entry.local == local => {
                    entry.dirty = true;
                    return Ok(reg);
                }

                _ => (),
            }
        }

        let entry = Some(Entry { local, dirty: true });

        self.take_slot(regs, &[], entry).map(|slot| slot.reg)
    }

    pub fn scratch(
        &self,
        regs: &mut Allocations<'a, E>,
        locked: &[E::Register],
    ) -> io::Result<E::Register> {
        self.take_slot(regs, locked, None).map(|slot| slot.reg)
    }

    pub fn read_into(
        &self,
        regs: &mut Allocations<'a, E>,
        reg: E::Register,
        local: Local,
    ) -> io::Result<()> {
        let slot = regs
            .0
            .iter_mut()
            .find(|slot| slot.reg == reg)
            .expect("register not in file");

        let overwrite = match &mut slot.entry {
            Some(entry) if entry.local == local => false,
            Some(entry) => {
                E::reg_to_local(self, reg, entry.local)?;
                *entry = Entry {
                    local,
                    dirty: false,
                };

                true
            }

            None => true,
        };

        if overwrite {
            E::local_to_reg(self, local, reg)?;
        }

        Ok(())
    }

    pub fn assert_dirty(&self, regs: &mut Allocations<'a, E>, reg: E::Register, local: Local) {
        regs.0
            .iter()
            .filter_map(|slot| match &slot.entry {
                Some(entry) if entry.local == local => Some(local),
                _ => None,
            })
            .next()
            .ok_or(())
            .expect_err("assert_dirty() on loaded local");

        let slot = regs
            .0
            .iter_mut()
            .find(|slot| slot.reg == reg)
            .expect("bad register");
        assert!(slot.entry.is_none(), "assert_dirty() on occupied register");

        slot.entry = Some(Entry { local, dirty: true });
    }

    fn take_slot<'b>(
        &self,
        regs: &'b mut Allocations<'a, E>,
        locked: &[E::Register],
        entry: Option<Entry>,
    ) -> io::Result<&'b mut Slot<E::Register>>
    where
        'a: 'b,
    {
        let slot = self.find_slot(regs, locked)?;
        slot.entry = entry;

        Ok(slot)
    }

    fn find_slot<'b>(
        &self,
        regs: &'b mut Allocations<'a, E>,
        locked: &[E::Register],
    ) -> io::Result<&'b mut Slot<E::Register>> {
        // La estructura de esta función es un poco obtusa debido a
        // limitaciones actuales de rustc/borrowck

        if regs
            .0
            .iter_mut()
            .find(|slot| slot.entry.is_none())
            .is_some()
        {
            Ok(regs.0.iter_mut().find(|slot| slot.entry.is_none()).unwrap())
        } else {
            // Todos los registros están ocupados, se hace spill de alguno
            let slot = regs
                .0
                .iter_mut()
                .filter(|slot| locked.iter().find(|locked| **locked == slot.reg).is_none())
                .next()
                .expect("register file exhaustion");

            E::reg_to_local(self, slot.reg, slot.entry.as_ref().unwrap().local)?;
            Ok(slot)
        }
    }
}

impl<'a, E: Emitter<'a>> Default for Allocations<'a, E> {
    fn default() -> Self {
        let allocations = E::Register::FILE
            .iter()
            .copied()
            .map(|reg| Slot { reg, entry: None })
            .collect::<Vec<_>>();

        Allocations(allocations)
    }
}
