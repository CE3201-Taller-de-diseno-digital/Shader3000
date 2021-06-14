//! Representación intermedia de código.
//!
//! La representación intermedia es lineal y recuerda ligeramente
//! a lenguajes ensambladores. Idealmente, deben ser simple
//! traducir un programa semánticamente analizado a representación
//! intermedia, y a su vez debe ser simple traducir código IR al
//! lenguaje ensamblador objetivo.
//!
//! # Locales
//! Toda función generada dispone de un número de "locales", cada
//! una de las cuales de identifica por índice. Las locales son celdas
//! de memoria no tipadas cuyo tamaño es constante pero depende de la
//! arquitectura objetivo. Las primeras locales corresponden uno a uno
//! a los parámetros de la función.
//!
//! # Etiquetas
//! El control de flujo se realiza a través de etiquetas y saltos. Las
//! etiquetas, al igual que las locales, existen por el hecho de ser
//! identificadas numéricamente y no se declaran de alguna otra manera.
//! Todas las instrucciones de IR involucran a locales, etiquetas o
//! ambas.
//!
//! # Símbolos
//! Para este punto del proceso de compilación, tanto variables
//! globales como funciones externas han sido reducidas a símbolos
//! ensamblables.

use std::rc::Rc;

/// Un programa en representación intermedia.
#[derive(Debug)]
pub struct Program {
    pub globals: Vec<Global>,
    pub code: Vec<GeneratedFunction>,
}

#[derive(Clone, Debug)]
pub enum Function {
    External(&'static str),
    Generated(Rc<String>),
}

impl Function {
    pub fn name(&self) -> &str {
        match self {
            Function::External(name) => name,
            Function::Generated(name) => &name,
        }
    }
}

#[derive(Debug)]
pub struct GeneratedFunction {
    pub name: Rc<String>,
    pub body: Vec<Instruction>,
    pub parameters: u32,
}

/// Las etiquetas están constituidas por identificadores arbitrarios,
/// no necesariamente secuenciales pero sí únicos.
#[derive(Copy, Clone, Debug, Default)]
pub struct Label(pub u32);

/// Las locales se identifican por índices secuenciales.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Local(pub u32);

/// Una variable global se identifica únicamente por su símbolo.
#[derive(Clone, Debug)]
pub struct Global(Rc<String>);

impl AsRef<str> for Global {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl<S: Into<String>> From<S> for Global {
    fn from(string: S) -> Self {
        Global(Rc::new(string.into()))
    }
}

/// Una instrucción de representación intermedia.
#[derive(Debug)]
pub enum Instruction {
    /// Copia contenidos de una local a otra.
    Move(Local, Local),

    /// Establecer la ubicación de una etiqueta al punto donde ocurre
    /// esta instrucción la secuencia del programa.
    SetLabel(Label),

    /// Saltar incondicionalmente a una etiqueta.
    Jump(Label),

    /// Saltar a una etiqueta si y solo si el valor de una local es cero.
    /// De lo contrario, no se realiza ninguna acción.
    JumpIfFalse(Local, Label),

    /// Sobreescribir los contenidos de una local con una constante.
    LoadConst(i32, Local),

    /// Copiar los contenidos de una variable global a una local.
    LoadGlobal(Global, Local),

    /// Copiar los contenidos de una local a una variable global.
    StoreGlobal(Local, Global),

    /// Llamar a una función, copiando los argumentos de las locales
    /// indicadas para ese efecto. Opcionalmente, el valor de retorno
    /// de la función se escribe a una local. Los contenidos de locales
    /// se preservan tras llamadas a funciones arbitrarias.
    Call {
        target: Function,
        arguments: Vec<Local>,
        output: Option<Local>,
    },
}
