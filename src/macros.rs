macro_rules! dispatch_arch {
    ($type:ident: $arch:expr => $expr:expr) => {{
        use crate::arch::{Arch, Target, Xtensa, X86_64};

        match $arch {
            Arch::X86_64 => {
                type $type = X86_64;
                $expr
            }

            Arch::Xtensa => {
                type $type = Xtensa;
                $expr
            }
        }
    }};
}

macro_rules! emit {
    ($self:expr, $opcode:expr) => {
        writeln!($self.output, "\t{}", $opcode)
    };

    ($self:expr, $opcode:expr, $($format:tt)*) => {{
        write!($self.output, "\t{:8}", $opcode)?;
        writeln!($self.output, $($format)*)
    }};
}
