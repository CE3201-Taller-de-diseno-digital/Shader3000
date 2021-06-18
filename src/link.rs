//! Construcción de ejecutables.
//!
//! Una vez que se ha emitido código ensamblador, este debe ser
//! ensamblado y enlazado contra `libruntime` para producir in binario
//! ejecutable.

use std::{
    fs,
    io::BufWriter,
    path::Path,
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    str::FromStr,
};

use crate::arch::Arch;
use bitflags::bitflags;
use thiserror::Error;

bitflags! {
    /// Opciones a aplicar durante el enlazado.
    pub struct LinkOptions: u32 {
        /// Remover símbolos de depuración del ejecutable final.
        ///
        /// Esta operación reduce significativamente el tamaño del
        /// ejecutable en muchos casos. Es buena práctica utilizarla
        /// para distribuir binarios release.
        const STRIP = 0x01;
    }
}

/// Un error de ensamblado o enlazado.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LinkerError {
    /// Ocurrió un evento de error de E/S durante la invocación
    /// de comandos externos.
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    /// El enlazador inició su ejecución, pero falló en enlazar.
    #[error("Linker exited with status code {0:?}")]
    Failed(ExitStatus),
}

/// Plataforma objetivo.
///
/// Las plataformas objetivo se diferencian de las arquitecturas
/// objetivo ([`Arch`]) en que para una plataforma se define un
/// entorno de ejecución particular sobre una arquitectura (ISA)
/// específica. Las diferentes configuraciones de `libruntime`
/// toman esto en cuenta, por lo cual no es suficiente discriminar
/// el sistema objetivo a partir de únicamente su ISA de procesador.
#[derive(Copy, Clone)]
pub enum Platform {
    /// Plataforma sobre la que core el compilador ("hosted").
    Native,

    /// Espressif ESP8266.
    Esp8266,
}

impl Platform {
    /// Obtiene la ISA asociada a esta plataforma.
    pub fn arch(self) -> Arch {
        #[cfg(not(target_arch = "x86_64"))]
        error!("Native target is not x86-64");

        match self {
            Platform::Native => Arch::X86_64,
            Platform::Esp8266 => Arch::Xtensa,
        }
    }
}

impl FromStr for Platform {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "native" => Ok(Platform::Native),
            "esp8266" => Ok(Platform::Esp8266),
            _ => Err(()),
        }
    }
}

/// Instancia del enlazador para un ejecutable definido.
///
/// Las operaciones de ensamblado y enlazado se delegan a ejecutables
/// externos del paquete `binutils`.
pub struct Linker {
    child: Child,
    stdin: BufWriter<ChildStdin>,
}

impl Linker {
    /// Inicia una instancia del enlazador.
    ///
    /// El enlazador tratará de emitir un ejecutable y escribirlo a
    /// la ruta indicada por `output`.
    pub fn spawn<O>(platform: Platform, output: &O, opts: LinkOptions) -> Result<Self, LinkerError>
    where
        O: AsRef<Path>,
    {
        let params = platform.link_params();

        let mut library_path = fs::read_link("/proc/self/exe").expect("Failed to read symlink");
        library_path.pop(); // "<...>/compiler" => "<...>"
        library_path.push("lib");
        library_path.push(params.name);

        // Para ensamblar el código máquina generador por codegen,
        // se hace pipe del mismo al stdin del linker.
        let mut command = Command::new(params.command);
        command
            .args(params.extra_args)
            // Ruta de búsqueda de bibliotecas en lib/{platform}
            .arg("-L")
            .arg(&library_path)
            .arg("-o")
            .arg(output.as_ref())
            // Se descarta código muerto, se asume entrada en asm y se enlaza
            // contra la biblioteca de soporte libruntime
            .args(&["-Wl,--gc-sections", "-xassembler", "-", "-lruntime"])
            .stdin(Stdio::piped());

        if opts.contains(LinkOptions::STRIP) {
            command.arg("-s");
        }

        let mut child = command.spawn().map_err(LinkerError::Io)?;
        let stdin = BufWriter::new(child.stdin.take().unwrap());

        Ok(Linker { child, stdin })
    }

    /// Obtiene la entrada estándar del rproceso que espera recibir ensamblador.
    ///
    /// Luego de crear una instancia con [`.spawn()`], se debe escribir código
    /// ensamblador en la forma exacta en que fue emitido por las fases de
    /// generación de código.
    pub fn stdin(&mut self) -> &mut BufWriter<ChildStdin> {
        &mut self.stdin
    }

    /// Indica el fin del flujo de código y finaliza el enlazado.
    pub fn finish(mut self) -> Result<(), LinkerError> {
        drop(self.stdin);

        let status = self.child.wait().map_err(LinkerError::Io)?;
        if status.success() {
            Ok(())
        } else {
            Err(LinkerError::Failed(status))
        }
    }
}

/// Información acerca del enlazador requerido para cada plataforma.
struct Parameters {
    /// Nombre de la plataforma.
    ///
    /// Esto se utiliza para examinar el subdirectorio apropiado dentro
    /// de `lib/`.
    name: &'static str,

    /// Comando de enlazado.
    command: &'static str,

    /// Argumentos adicionales al comando de enlazado que se necesitan
    /// para esta plataforma.
    extra_args: &'static [&'static str],
}

impl Platform {
    /// Enumera los detalles del comano de enlazado por plataforma.
    fn link_params(self) -> Parameters {
        match self {
            Platform::Native => Parameters {
                name: "native",
                command: "gcc",

                // rustc usa libpthread para hilos, libdl para enlazado
                // lazy en tiempo de ejecución y libm para floats
                extra_args: &["-pthread", "-ldl", "-lm"],
            },

            Platform::Esp8266 => Parameters {
                name: "esp8266",
                command: "xtensa-lx106-elf-gcc",

                // Esta es una plataforma #![no_std], por lo cual -nostartfiles
                // evita enlazar objetos de bootstrap que asumen un entorno
                // hosted. El linker script link.x dispone las secciones del
                // ejecutable en la distribución de rangos de flash y RAM particulares
                // al ESP8266.
                extra_args: &["-nostartfiles", "-Wl,-Tlink.x"],
            },
        }
    }
}
