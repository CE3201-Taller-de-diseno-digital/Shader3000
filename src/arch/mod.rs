//! Detalles específicos para cada arquitectura objetivo.
//!
//! Este módulo expone interfaces de generación de código
//! y de parámetros de arquitectura que son implementadas
//! por sus propios submódulos. En general, debe utilizarse
//! la macro `dispatch_arch!()` para acceder a estas
//! implementaciones.

use crate::{
    codegen::{regs::Allocations, Context},
    ir::{BinOp, Function, Global, Instruction, Local},
};

use std::io;

/// Arquitectura de procesador (ISA).
#[derive(Copy, Clone)]
pub enum Arch {
    X86_64,
    Xtensa,
}

mod x86_64;
mod xtensa;

pub use x86_64::Emitter as X86_64;
pub use xtensa::Emitter as Xtensa;

/// Emisión de código ensamblador para una función.
///
/// Los tipos que implementa ese trait traducen operaciones
/// primitivas del lenguaje intermedio a código máquina para
/// la arquitectura objetivo.
pub trait Emitter<'a>: Sized {
    /// Tamaño natural de un valor no tipado, en bytes.
    const VALUE_SIZE: u32;

    /// TIpo de registro.
    type Register: Register;

    /// Estado que se preserva durante las subfases de llamada.
    type CallInfo;

    /// Estado de cada marco de llamada.
    type FrameInfo: Default;

    /// Construir a partir de un contexto de emisión y un listado de
    /// instrucciones en representación intermedia.
    ///
    /// AdeMás de construirse, el prólogo de la función debe emitirse
    /// aquí, ajustando la pila y creando un stack frame.
    fn new(cx: Context<'a, Self>, instructions: &[Instruction]) -> io::Result<Self>;

    /// Emite el epílogo de la función, terminando su listado de código.
    fn epilogue(self) -> io::Result<()>;

    /// Obtiene el contexto de emisión y el estado de reservación
    /// de registros.
    ///
    /// Implicado aquí que todo `Emitter` debe guardar as-is el [`Context`]
    /// que se le otorga en [`Emitter::new()`].
    fn cx_regs(&mut self) -> (&mut Context<'a, Self>, &mut Allocations<'a, Self>);

    /// Saltar incondicionalmente a una etiqueta.
    fn jump_unconditional(&mut self, label: &str) -> io::Result<()>;

    /// Saltar a una etiqueta si un registro contiene cero.
    fn jump_if_false(&mut self, reg: Self::Register, label: &str) -> io::Result<()>;

    /// Copiar una constante a un registro.
    fn load_const(&mut self, value: i32, reg: Self::Register) -> io::Result<()>;

    /// Copiar los contenidos de una variable global a un registro.
    fn load_global(&mut self, global: &Global, reg: Self::Register) -> io::Result<()>;

    /// Copiar los contenidos de un registro a una vriable global.
    fn store_global(&mut self, reg: Self::Register, global: &Global) -> io::Result<()>;

    /// Niega un booleano.
    fn not(&mut self, reg: Self::Register) -> io::Result<()>;

    /// Calcula el complemento a dos de un entero.
    fn negate(&mut self, reg: Self::Register) -> io::Result<()>;

    /// Realiza una operación binaria (aritmética o lógica).
    fn binary(&mut self, lhs: Self::Register, op: BinOp, rhs: Self::Register) -> io::Result<()>;

    /// Copia los argumentos de una llamada a sus posiciones
    /// definidas por la convención de llamada.
    ///
    /// El estado retornado por esta función le permite a [`Emitter::call()`]
    /// llevar cuenta de información que se calculó durante la colocaeción
    /// de argumentos, generalmente con el propósito de limpiar argumentos
    /// colocados.
    fn prepare_args(&mut self, arguments: &[Local]) -> io::Result<Self::CallInfo>;

    /// Invocar a una función.
    ///
    /// Se asume para este punto que se han colocado los argumentos
    /// en los lugares correctos y que no quedan registros dirty.
    fn call(&mut self, target: &Function, call_info: Self::CallInfo) -> io::Result<()>;

    /// Copia los contenidos de un registro a una local.
    fn reg_to_local(cx: &Context<'a, Self>, reg: Self::Register, local: Local) -> io::Result<()>;

    /// Copia los contenidos de una local a un registro.
    fn local_to_reg(cx: &Context<'a, Self>, local: Local, reg: Self::Register) -> io::Result<()>;

    /// Copia los contenidos de un registro a otro.
    fn reg_to_reg(
        cx: &Context<'a, Self>,
        source: Self::Register,
        target: Self::Register,
    ) -> io::Result<()>;

    /// Véase [Context::spill()].
    fn spill(&mut self) -> io::Result<()> {
        let (cx, regs) = self.cx_regs();
        cx.spill(regs)
    }

    /// Véase [Context::clear()].
    fn clear(&mut self) -> io::Result<()> {
        let (cx, regs) = self.cx_regs();
        cx.clear(regs)
    }

    /// Véase [Context::write()].
    fn read(&mut self, local: Local) -> io::Result<Self::Register> {
        let (cx, regs) = self.cx_regs();
        cx.read(regs, local)
    }

    /// Véase [Context::write()].
    fn write(&mut self, local: Local) -> io::Result<Self::Register> {
        let (cx, regs) = self.cx_regs();
        cx.write(regs, local)
    }

    /// Véase [Context::assert_dirty()]
    fn assert_dirty(&mut self, reg: Self::Register, local: Local) {
        let (cx, regs) = self.cx_regs();
        cx.assert_dirty(regs, reg, local);
    }
}

/// Registro de procesador.
pub trait Register: 'static + Copy + PartialEq + Eq {
    /// Registro en el que se encuentran valores de retorno.
    const RETURN: Self;

    /// Registros disponsibles para reservación.
    const FILE: &'static [Self];
}
