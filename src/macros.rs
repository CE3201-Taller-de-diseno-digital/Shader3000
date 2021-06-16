//! Macros internas utilizadas en el compilador.
//!
//! Este módulo existe debido a limitaciones del sistema de macros
//! declarativas. Habría sido posible resolver esto mismo con el uso
//! de `#![feature(pub_macro_rules)]`, pero esa característic fue
//! eliminada en marzo de 2021. Existen interdependencias inherentes
//! entre los módulos `compiler::codegen` y `compiler::arch`. En el
//! caso específico de las macros que estos módulos esperan exportar
//! a el otro, no existe forma de lograr ello más que exponer las
//! macros involucradas a nivel de todo el crate.

/// Selecciona una sustitución apropiada de arquitectura objetivo en
/// una expresión según corresponda.
///
/// Conceptualmente, esta macro permite lo que aparenta ser un parámetro
/// de tipo estático escogido en tiempo de ejecución. La intención es
/// seleccionar una implementación apropiada para una arquitectura que
/// no será conocida hasta la ejecución de la expresión. El metaparámetro
/// `$type` nombra a un parámetro de tipo que será sustituido en la
/// expresión `$expr` con la implementación adecuada del trait `arch::Emitter`.
///
/// # Ejemplo
/// ```
/// println!("{}", dispatch_arch!(T: Arch::Xtensa, T::VALUE_SIZE));
/// ```
macro_rules! dispatch_arch {
    ($type:ident: $arch:expr => $expr:expr) => {{
        use crate::arch::{Arch, Xtensa, X86_64};

        match $arch {
            Arch::X86_64 => {
                type $type<'target> = X86_64<'target>;
                $expr
            }

            Arch::Xtensa => {
                type $type<'target> = Xtensa<'target>;
                $expr
            }
        }
    }};
}

/// Emite una línea de código ensamblador.
///
/// El propósito de esta macro es desacoplar la presentación
/// tabular e indentada de los mnemónicos de opcode de los
/// muchos puntos desde donde se emite código ensamblable.
///
/// # Ejemplo
/// Sea `cx` una expresión que refiera a un `codegen::Context`.
/// ```
/// emit!(cx, "ret")?; // Instrucción sin operandos
/// emit!(cx, "mov", "%{}, %{}", "rax", "rsi")?; // Instrucción con operandos
/// ```
macro_rules! emit {
    ($context:expr, $opcode:expr) => {
        writeln!($context, "\t{}", $opcode)
    };

    ($context:expr, $opcode:expr, $($format:tt)*) => {{
        write!($context, "\t{:8}", $opcode)?;
        writeln!($context, $($format)*)
    }};
}

macro_rules! emit_label {
    ($context:expr, $label:expr) => {
        writeln!($context, "\t.L{}.{}:", $context.function().name, $label.0)
    };
}

/// Genera el símbolo que corresponde a una etiqueta dentro de una función.
macro_rules! format_label {
    ($context:expr, $label:expr) => {
        format!(".L{}.{}", $context.function().name, $label.0)
    };
}
