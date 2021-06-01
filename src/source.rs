//! Rastreo de ubicaciones originales en código fuente.
//!
//! Los distintos objetos internos que el compilador construye
//! deben llevar cuenta de posiciones o rangos de ubicaciones en
//! el código fuente original, lo cual permite determinar un punto
//! exacto o aproximado en donde ocurre un error de abstracción
//! arbitraria.

use std::{
    cell::RefCell,
    fmt::{self, Debug, Display, Formatter},
    io::{self, BufRead},
    iter,
    ops::Range,
    rc::Rc,
};

/// Ancho de los divisores de tabulador.
const TAB_STOP: u32 = 4;

/// Un flujo de entrada, carácter por carácter.
pub trait InputStream = Iterator<Item = Result<(char, Location), io::Error>>;

/// Un objeto cualquiera con una posición original asociada.
#[derive(Debug, Clone)]
pub struct Located<T> {
    location: Location,
    value: T,
}

impl<T> Located<T> {
    /// Obtiene el valor.
    pub fn val(&self) -> &T {
        &self.value
    }

    /// Obtiene la ubicación.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// Descarta la ubicación y toma ownership del valor.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Descompone y toma ownership de las dos partes.
    pub fn split(self) -> (Location, T) {
        (self.location, self.value)
    }

    /// Construye a partir de un valor y una ubicación.
    pub fn at(value: T, location: Location) -> Self {
        Located { value, location }
    }

    /// Transforma el valor con la misma ubicación.
    pub fn map<U, F>(self, map: F) -> Located<U>
    where
        F: FnOnce(T) -> U,
    {
        Located {
            value: map(self.value),
            location: self.location,
        }
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
    from: Rc<Source>,
    position: Range<Position>,
}

impl Location {
    /// Unifica un rango de ubicaciones. Se asume el mismo origen.
    pub fn span(from: Location, to: &Location) -> Self {
        Location {
            from: from.from,
            position: from.position.start..to.position.end,
        }
    }

    /// Obtiene la posición de inicio.
    pub fn start(&self) -> Position {
        self.position.start
    }

    /// Obtiene la posición de fin.
    pub fn end(&self) -> Position {
        self.position.end
    }
}

impl Display for Location {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:", self.from.name)?;

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

    /// Ajusta la posición a la siguiente columna de tabulador.
    pub fn tab(self) -> Position {
        let column = 1 + ((self.column - 1) / TAB_STOP + 1) * TAB_STOP;
        Position {
            line: self.line,
            column,
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
/// no trivial para realizar la misma operación. La ubicación que se
/// encuentra en la tupla de retorno es la posición que le corresponderá
/// al primer caracter en la salida. Cada carácter emitido incluye a la
/// ubicación del siguiente.
pub fn consume<R, S>(reader: R, name: S) -> (Location, impl InputStream)
where
    R: BufRead,
    S: Into<String>,
{
    let source = Rc::new(Source {
        name: name.into(),
        lines: Default::default(),
    });

    let start = Location {
        from: Rc::clone(&source),
        position: Position::default()..Position::default().advance(),
    };

    let chars = reader
        .lines()
        .enumerate()
        .map(move |(line_index, line)| {
            let source = Rc::clone(&source);

            Fallible::new(line.map(move |line| {
                let line_chars: Vec<_> = line.chars().collect();
                source.lines.borrow_mut().push(line);

                let mut column = 1;
                line_chars
                    .into_iter()
                    .chain(iter::once('\n'))
                    .map(move |c| {
                        let here = Position {
                            line: line_index as u32 + 1,
                            column,
                        };

                        let next = match c {
                            '\n' => here.newline(),
                            '\t' => here.tab(),
                            _ => here.advance(),
                        };

                        column = next.column;
                        let location = Location {
                            from: Rc::clone(&source),
                            position: next..next.advance(),
                        };

                        (c, location)
                    })
            }))
        })
        .flatten()
        .fuse();

    (start, chars)
}

/// Nombre de origen e histórico interior de líneas.
struct Source {
    name: String,
    lines: RefCell<Vec<String>>,
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
