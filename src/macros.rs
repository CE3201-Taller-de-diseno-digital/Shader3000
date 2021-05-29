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

macro_rules! emit {
    ($context:expr, $opcode:expr) => {
        writeln!($context.output(), "\t{}", $opcode)
    };

    ($context:expr, $opcode:expr, $($format:tt)*) => {{
        write!($context.output(), "\t{:8}", $opcode)?;
        writeln!($context.output(), $($format)*)
    }};
}
