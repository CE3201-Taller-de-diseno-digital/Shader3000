//! Rastreo de ubicaciones originales en código fuente.
//!
//! Los distintos objetos internos que el compilador construye
//! deben llevar cuenta de posiciones o rangos de ubicaciones en
//! el código fuente original, lo cual permite determinar un punto
//! exacto o aproximado en donde ocurre un error de abstracción
//! arbitraria.

use std::{
    fmt::{self, Debug, Display, Formatter},
    io, iter,
    ops::Range,
    rc::Rc,
};

/// Un flujo de entrada es cualquier flujo de caracteres.
pub trait InputStream = Iterator<Item = Result<char, io::Error>>;

/// Un objeto cualquiera con una posición original asociada.
#[derive(Debug, Clone)]
pub struct Located<T> {
    location: Location,
    value: T,
}

impl<T> Located<T> {
    /// Construye un `Located` a partir de un valor, un identificador
    /// de origen y un rango de posiciones.
    pub fn at(value: T, from: SourceName, position: Range<Position>) -> Self {
        Located {
            value,
            location: Location::new(from, position),
        }
    }

    /// Construye a partir de un valor y una ubicación.
    pub fn from_location(value: T, location: &Location) -> Self {
        Located {
            value: value,
            location: location.clone(),
        }
    }

    /// Transporta una ubicación a otro objeto.
    pub fn from_one<U>(value: T, src: Located<U>) -> Located<T> {
        Located::at(
            value,
            src.location.from.clone(),
            src.location.position.clone(),
        )
    }

    /// Construye, tomando para inicio y fin el inicio y fin respectivo
    /// de otros objetos ubicados cualesquiera.
    pub fn from_two<U, V>(value: T, src_start: Located<U>, src_end: Located<V>) -> Located<T> {
        Located::at(
            value,
            src_start.location.from.clone(),
            src_start.location.position.start..src_end.location.position.end,
        )
    }

    /// Obtiene el valor.
    pub fn val(&self) -> &T {
        &self.value
    }

    /// Obtiene la ubicación.
    pub fn location(&self) -> &Location {
        &self.location
    }
}

impl<T> AsRef<T> for Located<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

/// Una ubicación está conformada por un origen y un rango de posiciones.
#[derive(Clone)]
pub struct Location {
    from: SourceName,
    position: Range<Position>,
}

impl Location {
    /// Construye una ubicación.
    pub fn new(from: SourceName, position: Range<Position>) -> Self {
        Location { from, position }
    }
}

impl Display for Location {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:", self.from)?;

        let Range { start, end } = self.position;
        if end == start.advance() {
            // Solo se señala una columna en específico
            write!(formatter, "{}", start)
        } else {
            write!(formatter, "[{}-{}]", start, end.back())
        }
    }
}

impl Debug for Location {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        <Self as Display>::fmt(self, formatter)
    }
}

/// Un identificador de flujo origen.
///
/// La cadena es arbitraria y no se interpreta de ninguna forma.
/// Su propósito es añadir información a mensajes de error.
#[derive(Clone)]
pub struct SourceName(Rc<String>);

impl SourceName {
    /// Construye un origen.
    pub fn from<N: Into<String>>(name: N) -> Self {
        SourceName(Rc::new(name.into()))
    }
}

impl Display for SourceName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

/// Una posición línea-columna en un archivo.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Position {
    line: u32,
    column: u32,
}

impl Position {
    /// Obtiene el número de línea.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Obtiene el número de columna.
    pub fn column(&self) -> u32 {
        self.column
    }

    /// Incrementa el número de columna.
    pub fn advance(self) -> Position {
        Position {
            line: self.line,
            column: self.column + 1,
        }
    }

    /// Decrementa el número de columna.
    pub fn back(self) -> Position {
        Position {
            line: self.line,
            column: self.column - 1,
        }
    }

    /// Incrementa el número de línea y retorna a la columna 1.
    pub fn newline(self) -> Position {
        Position {
            line: self.line + 1,
            column: 1,
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Position { line: 1, column: 1 }
    }
}

impl Display for Position {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// Transforma un flujo de entrada estándar en uno que itera por carácter.
///
/// Esta función existe debido a que `std` no ofrece algún mecanismo
/// no trivial para realizar la misma operación.
pub fn consume<R: io::BufRead>(reader: R) -> impl InputStream {
    let line_chars = |line: String| line.chars().collect::<Vec<char>>().into_iter();
    reader
        .lines()
        .map(move |line| Fallible::new(line.map(line_chars)))
        .flatten()
        .fuse()
}

/// Un iterador que emite un solo error o encapsula las salidas de
/// otro iterador en `Ok`, pero nunca ambas.
struct Fallible<I, E>(Result<I, iter::Once<E>>);

impl<I, E> Fallible<I, E> {
    /// Crea un iterador a partir de un `Result`.
    pub fn new(result: Result<I, E>) -> Self {
        Fallible(result.map_err(iter::once))
    }
}

impl<I: Iterator, E> Iterator for Fallible<I, E> {
    type Item = Result<I::Item, E>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            Ok(ok) => ok.next().map(Ok),
            Err(error) => error.next().map(Err),
        }
    }
}
