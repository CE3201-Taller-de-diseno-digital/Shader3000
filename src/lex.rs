//! Análisis léxico.
//!
//! # Tokenization
//! Esta es la primera fase del compilador. Descompone un [`InputStream`]
//! (flujo de caracteres) en unidades léxicas denominadas tokens. Los espacios
//! en blanco y los comentarios se descartan durante esta operación. Cada
//! token emitido esta asociado a una ubicación en el código fuente original,
//! lo cual permite rastrear errores en tanto los mismos como constructos
//! más elevados de fases posteriores.
//!
//! # Contenido de un token
//! Este lexer no produce lexemas para casos donde no son necesarios o terminan
//! siendo más complicados. Por ejemplo, operadores, puntuación y palabras clave
//! se identifican por el hecho de lo que son y no incluyen lexemas. Por su parte,
//! los identificadores sí incluyen su lexema original. Las constantes literales
//! se resuelven a sus valores en vez de preservarsus lexemas.
//!
//! # Reglas importantes del lenguaje
//! - Los identificadores tienen un límite de longitud.
//! - Los identificadores pueden incluir `'@'` y `'?'`.
//! - Los identificadores deben empezar con una letra minúscula.
//! - Con excepción de la regla anterior, el lenguaje es case-insensitive,
//!   por lo cual tanto `procedure` como `PROCEDURE` y `ProcEDure`
//!   resultan en la palabra clave [`Keyword::Procedure`].
//!
//! # Errores
//! El lexer es capaz de recuperarse parcialmente de condiciones de error.
//! Esto ocurre en suficiente grado como para reportar más de un error por
//! ejecución, pero no lo suficiente como para permitir el avance a las
//! demás fases de la compilación.

use crate::source::{InputStream, Located, Location};
use std::{
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

use thiserror::Error;

// Case-insensitive
pub use unicase::Ascii as NoCase;

/// Límite de longitud de identificadores.
const MAX_ID_LENGTH: usize = 10;

/// Literal entero máximo.
const INT_MAX: i32 = i32::MAX;

/// Error de escaneo.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LexerError {
    /// Error de E/S originado por el [`InputStream`].
    #[error("I/O error")]
    Input(#[from] std::io::Error),

    /// Carácter desconocido o inesperado en el flujo de entrada.
    #[error("Bad character {0:?} in input stream")]
    BadChar(char),

    /// Se esperaba un carácter específico en esta posición.
    #[error("Expected {0:?}")]
    Expected(char),

    /// No se reconoce una secuencia de escape en un literal de cadena.
    #[error("Bad escape sequence")]
    BadEscape,

    /// No se cerró un terminal de cadena.
    #[error("Unterminated string literal")]
    UnterminatedString,

    /// Un identificador excede el límite de longitud.
    #[error("Identifier exceeds {MAX_ID_LENGTH} characters")]
    IdTooLong,

    /// Una constante entera se encuentra fuera de rango.
    #[error("Integer literal overflow, valid range is [0, {INT_MAX}]")]
    IntOverflow,
}

/// Un identificador.
///
/// Los identificadores cumplen ciertas reglas de contenido y longitud.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier(Rc<NoCase<String>>);

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for Identifier {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(fmt)
    }
}

/// Un literal de cadana.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLiteral(Rc<NoCase<String>>);

impl AsRef<NoCase<String>> for StrLiteral {
    fn as_ref(&self) -> &NoCase<String> {
        &self.0
    }
}

/// Objeto resultante del análisis léxico.
///
/// Un token contiene suficiente información para describir completamente
/// a una entidad léxica en el programa fuente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Identificador.
    Id(Identifier),

    /// Palabra clave.
    Keyword(Keyword),

    /// Literal de cadena.
    StrLiteral(StrLiteral),

    /// Literal de entero.
    IntLiteral(i32),

    /// `=`
    Assign,

    /// `,`
    Comma,

    /// `.`
    Period,

    /// `+`
    Plus,

    /// `-`
    Minus,

    /// `*`
    Times,

    /// `**`
    Pow,

    /// `/`
    Div,

    /// `//`
    IntegerDiv,

    /// `%`
    Mod,

    /// `:`
    Colon,

    /// `;`
    Semicolon,

    /// `==`
    Equal,

    /// `<>`
    NotEqual,

    /// `<`
    Less,

    /// `<=`
    LessOrEqual,

    /// `>`
    Greater,

    /// `>=`
    GreaterOrEqual,

    /// `(`
    OpenParen,

    /// `[`
    OpenSquare,

    /// `{`
    OpenCurly,

    /// `)`
    CloseParen,

    /// `]`
    CloseSquare,

    /// `}`
    CloseCurly,
}

impl Display for Token {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;

        match self {
            Id(id) => write!(fmt, "identifier `{}`", id.0),
            Keyword(keyword) => write!(fmt, "keyword `{}`", keyword),
            StrLiteral(string) => write!(fmt, "literal \"{}\"", string.0),
            IntLiteral(integer) => write!(fmt, "literal `{}`", integer),
            Assign => fmt.write_str("`=`"),
            Comma => fmt.write_str("`,`"),
            Period => fmt.write_str("`.`"),
            Plus => fmt.write_str("`+`"),
            Minus => fmt.write_str("`-`"),
            Times => fmt.write_str("`*`"),
            Pow => fmt.write_str("`**`"),
            Div => fmt.write_str("`/`"),
            IntegerDiv => fmt.write_str("`//`"),
            Mod => fmt.write_str("`%`"),
            Colon => fmt.write_str("`:`"),
            Semicolon => fmt.write_str("`;`"),
            Equal => fmt.write_str("`==`"),
            NotEqual => fmt.write_str("`<>`"),
            Less => fmt.write_str("`<`"),
            LessOrEqual => fmt.write_str("`<=`"),
            Greater => fmt.write_str("`>`"),
            GreaterOrEqual => fmt.write_str("`>=`"),
            OpenParen => fmt.write_str("`(`"),
            OpenSquare => fmt.write_str("`[`"),
            OpenCurly => fmt.write_str("`{`"),
            CloseParen => fmt.write_str("`)`"),
            CloseSquare => fmt.write_str("]`"),
            CloseCurly => fmt.write_str("`}`"),
        }
    }
}

/// Una palabra clave.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Keyword {
    True,
    False,
    Type,
    List,
    Bool,
    Mat,
    Int,
    If,
    For,
    In,
    Step,
    Len,
    Call,
    Procedure,
    Blink,
    Delay,
    PrintLed,
    PrintLedX,
}

impl Display for Keyword {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Keyword::*;
        let string = match self {
            True => "true",
            False => "false",
            Type => "type",
            List => "list",
            Bool => "bool",
            Mat => "mat",
            Int => "int",
            If => "if",
            For => "for",
            In => "in",
            Len => "len",
            Step => "step",
            Call => "call",
            Procedure => "procedure",
            Blink => "blink",
            Delay => "delay",
            PrintLed => "PrintLed",
            PrintLedX => "PrintLedX",
        };

        fmt.write_str(string)
    }
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        use Keyword::*;

        const KEYWORDS: &'static [(NoCase<&'static str>, Keyword)] = &[
            (NoCase::new("true"), True),
            (NoCase::new("false"), False),
            (NoCase::new("type"), Type),
            (NoCase::new("list"), List),
            (NoCase::new("bool"), Bool),
            (NoCase::new("mat"), Mat),
            (NoCase::new("int"), Int),
            (NoCase::new("if"), If),
            (NoCase::new("for"), For),
            (NoCase::new("in"), In),
            (NoCase::new("len"), Len),
            (NoCase::new("step"), Step),
            (NoCase::new("call"), Call),
            (NoCase::new("procedure"), Procedure),
            (NoCase::new("Blink"), Blink),
            (NoCase::new("Delay"), Delay),
            (NoCase::new("PrintLed"), PrintLed),
            (NoCase::new("PrintLedX"), PrintLedX),
        ];

        KEYWORDS
            .iter()
            .find(|&&(name, _)| name == NoCase::new(string))
            .map(|&(_, keyword)| keyword)
            .ok_or(())
    }
}

/// Máquina de estados para análisis sintáctico.
///
/// Un lexer puede encontrarse en uno de diversos estados. La
/// salida del lexer, así como su siguiente estado, se define
/// a partir de tanto su estado actual como el siguiente carácter
/// encontrado en el flujo de entrada.
pub struct Lexer<S: Iterator> {
    source: std::iter::Peekable<S>,
    state: State,
    start: Location,
    next: Location,
}

/// Posibles estados del lexer.
enum State {
    /// Estado que ocurre antes de encontrar el inicio de un token.
    Start,

    /// Estado de error.
    Error,

    /// Estado de completitud; siempre emite el token incluido,
    /// consume la entrada actual y pasa a [`State::Start`].
    Complete(Token),

    /// Se encontró `#`.
    ///
    /// Debería seguir otro `#` para entrar en un comentario.
    Hash,

    /// Se encontró `*`.
    ///
    /// Puede resultar en [`Token::Times`] o [`Token::Pow`].
    Star,

    /// Se encontró `/`.
    ///
    /// Puede resultar en [`Token::Div`] o [`Token::IntegerDiv`].
    Slash,

    /// Se encontró `=`.
    ///
    /// Puede resultar en [`Token::Assign`] o [`Token::Equal`].
    AssignOrEqual,

    /// Se encontró `<`.
    ///
    /// Puede resultar en [`Token::Less`], [`Token::LessOrEqual`]
    /// o [`Token::NotEqual`].
    LeftAngle,

    /// Se encontró `>`.
    ///
    /// Puede resultar en [`Token::Greater`] o [`Token::GreaterOrEqual`].
    RightAngle,

    /// Comentario de línea.
    ///
    /// Este estado vuelve a [`State::Start`] al encontrar `'\n'`.
    Comment,

    /// Constante entera.
    ///
    /// Este estado incluirá dígitos en el token mientras que
    /// el siguiente carácter sea un dígito.
    Integer(i32),

    /// Literal de cadena.
    StringChars(String),

    /// Término que puede ser un identificador o una palabra clave.
    Word(String),
}

impl<S: InputStream> Lexer<S> {
    /// Crea un lexer en estado inicial a partir de un flujo.
    pub fn new(start: Location, source: S) -> Self {
        let next = start.clone();
        Lexer {
            source: source.peekable(),
            state: State::Start,
            start,
            next,
        }
    }

    /// Reduce la entrada a sea una secuencia conocida de tokens
    /// infalibles o una secuencia de errores.
    ///
    /// En caso de que ocurra al menos un error, el lexer dejará
    /// de buscar tokens exitosos y comenzará a acumular solamente
    /// errores. El propósito de esta función es permitir la
    /// recolección de múltiples errores léxicos en una misma ejecución
    /// del compilador.
    pub fn try_exhaustive(mut self) -> Result<Vec<Located<Token>>, Vec<Located<LexerError>>> {
        let mut tokens = Vec::new();

        while let Some(result) = self.next() {
            match result {
                Ok(token) => tokens.push(token),
                Err(error) => {
                    drop(tokens);

                    let mut errors = vec![error];
                    errors.extend(self.filter_map(Result::err));

                    return Err(errors);
                }
            }
        }

        Ok(tokens)
    }

    /// Intenta construir un siguiente token.
    fn lex(&mut self) -> Result<Option<(Token, Location)>, LexerError> {
        use {State::*, Token::*};

        let mut last_accepted = self.start.clone();
        let token = loop {
            // Se espera un siguiente carácter, fallando si hay error de E/S
            let next_char = match self.source.peek() {
                None => None,
                Some(Ok((c, _))) => Some(*c),
                Some(Err(_)) => break Err(self.source.next().unwrap().err().unwrap().into()),
            };

            // La posición de origen se mueve junto a la posición
            // siguiente siempre que no se haya encontrado una
            // frontera de token
            if let Start = self.state {
                self.start = self.next.clone();
            }

            // Switch table principal, determina cambios de estado
            // y de salida del lexer a partir de combinaciones del
            // estado actual y el siguiente carácter
            match (&mut self.state, next_char) {
                // Condiciones de error: se descarta la línea donde
                // ocurrió el error. Al llegar al final de la línea
                // el lexer se recupera y reinicia.
                (Error, None) => return Ok(None),
                (Error, Some('\n')) => self.state = Start,
                (Error, Some(_)) => (),

                // Tokens triviales
                (Start, None) => return Ok(None),
                (Start, Some(',')) => self.state = Complete(Comma),
                (Start, Some('.')) => self.state = Complete(Period),
                (Start, Some('+')) => self.state = Complete(Plus),
                (Start, Some('-')) => self.state = Complete(Minus),
                (Start, Some('%')) => self.state = Complete(Mod),
                (Start, Some(':')) => self.state = Complete(Colon),
                (Start, Some(';')) => self.state = Complete(Semicolon),
                (Start, Some('(')) => self.state = Complete(OpenParen),
                (Start, Some('[')) => self.state = Complete(OpenSquare),
                (Start, Some('{')) => self.state = Complete(OpenCurly),
                (Start, Some(')')) => self.state = Complete(CloseParen),
                (Start, Some(']')) => self.state = Complete(CloseSquare),
                (Start, Some('}')) => self.state = Complete(CloseCurly),
                (Start, Some('#')) => self.state = Hash,
                (Start, Some('*')) => self.state = Star,
                (Start, Some('/')) => self.state = Slash,
                (Start, Some('=')) => self.state = AssignOrEqual,
                (Start, Some('<')) => self.state = LeftAngle,
                (Start, Some('>')) => self.state = RightAngle,

                // Cadenas, identificadores y palabras clave
                (Start, Some('"')) => self.state = StringChars(String::new()),
                (Start, Some(c)) if c.is_ascii_alphabetic() => self.state = Word(c.to_string()),

                // Inicio de una constante numérica. No se consume
                // el entero, ya que esta lógica ya está implementada
                // en el respectivo caso para un estado de constante
                // entera para el cual el siguiente carácter es un
                // dígito. Por tanto, la constante es inicialmente cero.
                (Start, Some(c)) if c.is_ascii_digit() => {
                    self.state = Integer(0);
                    continue;
                }

                // Espacios en blanco y caracteres inesperados
                (Start, Some(c)) if c.is_ascii_whitespace() => (),
                (Start, Some(c)) => break Err(LexerError::BadChar(c)),

                // Emisión retardada de tokens cualesquiera
                (Complete(value), _) => break Ok(std::mem::replace(value, Plus)),

                // `#` siempre debería iniciar un comentario de la forma `##`
                (Hash, Some('#')) => self.state = Comment,
                (Hash, _) => break Err(LexerError::Expected('#')),

                // Multiplicación `*` y potencia `**`
                (Star, Some('*')) => self.state = Complete(Pow),
                (Star, _) => break Ok(Times),

                // División `/` y división entera `//`
                (Slash, Some('/')) => self.state = Complete(IntegerDiv),
                (Slash, _) => break Ok(Div),

                // Asignación `=` e igualdad `==`
                (AssignOrEqual, Some('=')) => self.state = Complete(Equal),
                (AssignOrEqual, _) => break Ok(Assign),

                // Comparaciones `<` y `<=` y desigualdad `<>`
                (LeftAngle, Some('=')) => self.state = Complete(LessOrEqual),
                (LeftAngle, Some('>')) => self.state = Complete(NotEqual),
                (LeftAngle, _) => break Ok(Less),

                // Comparaciones `>` y `>=`
                (RightAngle, Some('=')) => self.state = Complete(GreaterOrEqual),
                (RightAngle, _) => break Ok(Greater),

                // Los comentarios descartan la línea donde ocurren
                (Comment, Some('\n')) => self.state = Start,
                (Comment, Some(_)) => (),
                (Comment, None) => self.state = Start,

                // Acumulación dígito por dígito de constantes enteras
                (Integer(accumulated), Some(digit)) if digit.is_ascii_digit() => {
                    let digit = digit.to_digit(10).unwrap() as i32;

                    match accumulated
                        .checked_mul(10)
                        .and_then(|n| n.checked_add(digit))
                    {
                        Some(result) => *accumulated = result,
                        None => break Err(LexerError::IntOverflow),
                    }
                }

                // Si sigue algo que no es un dígito, la constante a terminado
                (Integer(integer), _) => break Ok(IntLiteral(*integer)),

                // Fin de literales de cadena
                (StringChars(string), Some('"')) => {
                    let literal = Rc::new(NoCase::new(std::mem::take(string)));
                    self.state = Complete(Token::StrLiteral(self::StrLiteral(literal)));
                }

                // Casos entre comillas para literales de cadena
                (StringChars(_), Some('\\')) => break Err(LexerError::BadEscape),
                (StringChars(string), Some(c)) if is_string_char(c) => string.push(c),
                (StringChars(_), _) => break Err(LexerError::UnterminatedString),

                // Extensión de términos
                (Word(word), Some(c)) if is_word_char(c) => {
                    if word.len() == MAX_ID_LENGTH {
                        break Err(LexerError::IdTooLong);
                    }

                    word.push(c);
                }

                // Si sigue algo que no puede formar parte del término, ha terminado
                (Word(word), _) => {
                    if let Ok(keyword) = self::Keyword::from_str(&word) {
                        break Ok(Keyword(keyword));
                    } else {
                        break Ok(Id(Identifier(Rc::new(NoCase::new(std::mem::take(word))))));
                    }
                }
            }

            // Si no hubo `continue`, aquí se consume el carácter que
            // se observó con lookahead anteriormente
            if let Some(Ok((_, next_position))) = self.source.next() {
                last_accepted = std::mem::replace(&mut self.next, next_position);
            }
        };

        token.map(|token| Some((token, last_accepted)))
    }
}

impl<S: InputStream> Iterator for Lexer<S> {
    type Item = Result<Located<Token>, Located<LexerError>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lex() {
            Ok(None) => None,
            Ok(Some((token, last_accepted))) => {
                self.state = State::Start;

                let location = Location::span(self.start.clone(), &last_accepted);
                Some(Ok(Located::at(token, location)))
            }

            Err(error) => {
                self.state = State::Error;
                Some(Err(Located::at(error, self.next.clone())))
            }
        }
    }
}

/// Determina si un carácter puede pertenecer al interior de
/// un literal de cadena.
fn is_string_char(c: char) -> bool {
    c == '_' || (!c.is_control() && !c.is_whitespace())
}

/// Determina si un carácter puede pertenecer a un término.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c.is_ascii_digit() || matches!(c, '@' | '_' | '?')
}
