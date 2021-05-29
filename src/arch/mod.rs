//! Detalles específicos para cada arquitectura objetivo.
//!
//! Este módulo expone interfaces de generación de código
//! y de parámetros de arquitectura que son implementadas
//! por sus propios submódulos. En general, debe utilizarse
//! la macro `dispatch_arch!()` para acceder a estas
//! implementaciones.

use crate::{
    codegen::Context,
    ir::{Function, Global, Instruction, Local},
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

    /// Construir a partir de un contexto de emisión y un listado de
    /// instrucciones en representación intermedia.
    ///
    /// AdeMás de construirse, el prólogo de la función debe emitirse
    /// aquí, ajustando la pila y creando un stack frame.
    fn new(cx: Context<'a, Self>, instructions: &[Instruction]) -> io::Result<Self>;

    /// Emite el epílogo de la función, terminando su listado de código.
    fn epilogue(self) -> io::Result<()>;

    /// Obtiene el contexto de emisión.
    ///
    /// Implicado aquí que todo `Emitter` debe guardar as-is el [`Context`]
    /// que se le otorga en [`Emitter::new()`].
    fn cx(&mut self) -> &mut Context<'a, Self>;

    /// Saltar incondicionalmente a una etiqueta.
    fn jump_unconditional(&mut self, label: &str) -> io::Result<()>;

    /// Saltar a una etiqueta si una local tiene valor cero.
    fn jump_if_false(&mut self, local: Local, label: &str) -> io::Result<()>;

    /// Copiar una constante a una local.
    fn load_const(&mut self, value: i32, local: Local) -> io::Result<()>;

    /// Copiar los contenidos de una variable global a una local.
    fn load_global(&mut self, global: &Global, local: Local) -> io::Result<()>;

    /// Copiar los contenidos de una local a una vriable global.
    fn store_global(&mut self, local: Local, global: &Global) -> io::Result<()>;

    /// Invocar a una función.
    ///
    /// El `Emitter` debe disponer los argumentos en los registros
    /// o ubicaciones correctas, llamar propiamente a la función,
    /// y opcionalmente asociar su valor de retorno con una local.
    fn call(
        &mut self,
        target: &Function,
        arguments: &[Local],
        output: Option<Local>,
    ) -> io::Result<()>;
}

/// Registro de procesador.
pub trait Register: Copy {}
