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
pub struct Program {
    pub globals: Vec<Rc<Global>>,
    pub code: Vec<Rc<Function>>,
}

/// Un objeto invocable con un símbolo conocido. Las funciones
/// pueden ser generadas o ser implementadas externamente y
/// encontradas durante la fase de enlazado.
pub struct Function {
    pub name: String,
    pub body: FunctionBody,
    pub parameters: u32,
}

/// El cuerpo de una función puede ser desconocido (externo)
/// o conformarse por una secuencia de instrucciones de IR.
pub enum FunctionBody {
    External,
    Generated(Vec<Instruction>),
}

/// Las etiquetas están constituidas por identificadores arbitrarios,
/// no necesariamente secuenciales pero sí únicos.
#[derive(Copy, Clone)]
pub struct Label(pub u32);

/// Las locales se identifican por índices secuenciales.
#[derive(Copy, Clone)]
pub struct Local(pub u32);

/// Una variable global se identifica únicamente por su símbolo.
pub struct Global(pub String);

/// Una instrucción de representación intermedia.
pub enum Instruction {
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
    LoadGlobal(Rc<Global>, Local),

    /// Copiar los contenidos de una local a una variable global.
    StoreGlobal(Local, Rc<Global>),

    /// Llamar a una función, copiando los argumentos de las locales
    /// indicadas para ese efecto. Opcionalmente, el valor de retorno
    /// de la función se escribe a una local. Los contenidos de locales
    /// se preservan tras llamadas a funciones arbitrarias.
    Call {
        target: Rc<Function>,
        arguments: Vec<Local>,
        output: Option<Local>,
    },
}
