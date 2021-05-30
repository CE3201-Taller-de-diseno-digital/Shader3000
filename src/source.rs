use std::{
    fmt::{self, Debug, Display, Formatter},
    io, iter,
    ops::Range,
    rc::Rc,
};

pub trait InputStream = Iterator<Item = Result<char, io::Error>>;

#[derive(Debug, Clone)]
pub struct Located<T> {
    location: Location,
    value: T,
}

impl<T> Located<T> {
    pub fn at(value: T, from: SourceName, position: Range<Position>) -> Self {
        Located {
            value,
            location: Location::new(from, position),
        }
    }
    pub fn from_location(value: T, location: &Location) -> Self {
        Located {
            value: value,
            location: location.clone(),
        }
    }
    pub fn from_one<U>(value: T, src: Located<U>) -> Located<T> {
        Located::at(
            value,
            src.location.from.clone(),
            src.location.position.clone(),
        )
    }
    pub fn from_two<U, V>(value: T, src_start: Located<U>, src_end: Located<V>) -> Located<T> {
        Located::at(
            value,
            src_start.location.from.clone(),
            src_start.location.position.start..src_end.location.position.end,
        )
    }
    pub fn val(&self) -> &T {
        &self.value
    }
    pub fn location(&self) -> &Location {
        &self.location
    }
}
impl<T> AsRef<T> for Located<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}
#[derive(Clone)]
pub struct Location {
    from: SourceName,
    position: Range<Position>,
}

impl Location {
    pub fn new(from: SourceName, position: Range<Position>) -> Self {
        Location { from, position }
    }
}

impl Display for Location {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:", self.from)?;

        let Range { start, end } = self.position;
        if end == start.advance() {
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

#[derive(Clone)]
pub struct SourceName(Rc<String>);

impl SourceName {
    pub fn from<N: Into<String>>(name: N) -> Self {
        SourceName(Rc::new(name.into()))
    }
}

impl Display for SourceName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Position {
    line: u32,
    column: u32,
}

impl Position {
    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn column(&self) -> u32 {
        self.column
    }

    pub fn advance(self) -> Position {
        Position {
            line: self.line,
            column: self.column + 1,
        }
    }

    pub fn back(self) -> Position {
        Position {
            line: self.line,
            column: self.column - 1,
        }
    }

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

pub fn consume<R: io::BufRead>(reader: R) -> impl InputStream {
    let line_chars = |line: String| line.chars().collect::<Vec<char>>().into_iter();
    reader
        .lines()
        .map(move |line| Fallible::new(line.map(line_chars)))
        .flatten()
        .fuse()
}

struct Fallible<I, E>(Result<I, iter::Once<E>>);

impl<I, E> Fallible<I, E> {
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
