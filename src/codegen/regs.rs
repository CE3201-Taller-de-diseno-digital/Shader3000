use super::Context;
use crate::{
    arch::{Emitter, Register},
    ir::Local,
};
use std::io;

pub struct Allocations<'a, E: Emitter<'a>> {
    slots: Vec<Slot<E::Register>>,
    next_id: usize,
}

struct Slot<R: Register> {
    reg: R,
    entry: Option<Entry>,
}

struct Entry {
    local: Local,
    dirty: bool,
    sequence: usize,
}

impl<'a, E: Emitter<'a>> Context<'a, E> {
    pub fn spill(&self, regs: &mut Allocations<'a, E>) -> io::Result<()> {
        for slot in regs.slots.iter_mut() {
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
        for slot in regs.slots.iter_mut() {
            slot.entry = None;
        }

        Ok(())
    }

    pub fn read(&self, regs: &mut Allocations<'a, E>, local: Local) -> io::Result<E::Register> {
        if let Some((reg, _)) = regs.find_local(local) {
            return Ok(reg);
        }

        let entry = Some(Entry {
            local,
            dirty: false,
            sequence: regs.next_sequence(),
        });

        let slot = self.take_slot(regs, &[], entry)?;
        E::local_to_reg(self, local, slot.reg)?;

        Ok(slot.reg)
    }

    pub fn write(&self, regs: &mut Allocations<'a, E>, local: Local) -> io::Result<E::Register> {
        if let Some((reg, entry)) = regs.find_local(local) {
            entry.as_mut().unwrap().dirty = true;
            return Ok(reg);
        }

        let entry = Some(Entry {
            local,
            dirty: true,
            sequence: regs.next_sequence(),
        });

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
        let move_from = match regs.find_local(local) {
            Some((old_reg, old_entry)) if old_reg != reg => {
                *old_entry = None;
                Some(old_reg)
            }

            _ => None,
        };

        let sequence = regs.next_sequence();
        let slot = regs
            .slots
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
                    sequence,
                };

                true
            }

            None => true,
        };

        match (overwrite, move_from) {
            (true, Some(old_reg)) => E::reg_to_reg(self, old_reg, reg),
            (true, None) => E::local_to_reg(self, local, reg),
            _ => Ok(()),
        }
    }

    pub fn assert_dirty(&self, regs: &mut Allocations<'a, E>, reg: E::Register, local: Local) {
        regs.slots
            .iter()
            .filter_map(|slot| match &slot.entry {
                Some(entry) if entry.local == local => Some(local),
                _ => None,
            })
            .next()
            .ok_or(())
            .expect_err("assert_dirty() on loaded local");

        let sequence = regs.next_sequence();
        let slot = regs
            .slots
            .iter_mut()
            .find(|slot| slot.reg == reg)
            .expect("bad register");

        assert!(slot.entry.is_none(), "assert_dirty() on occupied register");
        slot.entry = Some(Entry {
            local,
            dirty: true,
            sequence,
        });
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
            .slots
            .iter_mut()
            .find(|slot| slot.entry.is_none())
            .is_some()
        {
            Ok(regs
                .slots
                .iter_mut()
                .find(|slot| slot.entry.is_none())
                .unwrap())
        } else {
            // Todos los registros están ocupados, se hace spill de alguno
            let slot = regs
                .slots
                .iter_mut()
                .filter(|slot| locked.iter().find(|locked| **locked == slot.reg).is_none())
                .min_by_key(|slot| slot.entry.as_ref().unwrap().sequence)
                .expect("register file exhaustion");

            E::reg_to_local(self, slot.reg, slot.entry.as_ref().unwrap().local)?;
            Ok(slot)
        }
    }
}

impl<'a, E: Emitter<'a>> Allocations<'a, E> {
    fn find_local(&mut self, local: Local) -> Option<(E::Register, &mut Option<Entry>)> {
        for slot in self.slots.iter_mut() {
            let reg = slot.reg;

            match &mut slot.entry {
                Some(entry) if entry.local == local => return Some((reg, &mut slot.entry)),
                _ => continue,
            }
        }

        None
    }

    fn next_sequence(&mut self) -> usize {
        let next = self.next_id;
        self.next_id += 1;

        next
    }
}

impl<'a, E: Emitter<'a>> Default for Allocations<'a, E> {
    fn default() -> Self {
        let slots = E::Register::FILE
            .iter()
            .copied()
            .map(|reg| Slot { reg, entry: None })
            .collect::<Vec<_>>();

        Allocations { slots, next_id: 0 }
    }
}
