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

use crate::source::{InputStream, Located, Position, SourceName};
use std::str::FromStr;
use thiserror::Error;

/// Límite de longitud de identificadores.
const MAX_ID_LENGTH: usize = 10;

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
    #[error("identifier exceeds {MAX_ID_LENGTH} characters")]
    IdTooLong,

    /// Se trató de comenzar un identificador con una letra mayúscula.
    #[error("identifiers must begin with a lowercase letter")]
    UppercaseId,
}

/// Un identificador.
///
/// Los identificadores cumplen ciertas reglas de contenido y longitud.
#[derive(Debug, Clone)]
pub struct Identifier(String);

/// Objeto resultante del análisis léxico.
///
/// Un token contiene suficiente información para describir completamente
/// a una entidad léxica en el programa fuente.
#[derive(Debug, Clone)]
pub enum Token {
    /// Identificador.
    Id(Identifier),

    /// Palabra clave.
    Keyword(Keyword),

    /// Literal de cadena.
    StrLiteral(String),

    /// Literal de entero.
    IntLiteral(i32),

    /// `=`
    Assign,

    /// `,`
    Comma,

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

/// Una palabra clave.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Keyword {
    True,
    False,
    Type,
    List,
    Bool,
    Int,
    If,
    For,
    In,
    Step,
    Del,
    Procedure,
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        // "CI" es "Case Insensitive"
        use unicase::Ascii as CI;
        use Keyword::*;

        const KEYWORDS: &'static [(CI<&'static str>, Keyword)] = &[
            (CI::new("true"), True),
            (CI::new("false"), False),
            (CI::new("type"), Type),
            (CI::new("list"), List),
            (CI::new("bool"), Bool),
            (CI::new("int"), Int),
            (CI::new("if"), If),
            (CI::new("for"), For),
            (CI::new("in"), In),
            (CI::new("step"), Step),
            (CI::new("del"), Del),
            (CI::new("procedure"), Procedure),
        ];

        KEYWORDS
            .iter()
            .find(|&&(name, _)| name == CI::new(string))
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
    from: SourceName,
    state: State,
    start: Position,
    next: Position,
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
    /// Crea un lexer en estado inicial a partir de un flujo y su origen.
    pub fn new(source: S, from: SourceName) -> Self {
        Lexer {
            from,
            source: source.peekable(),
            state: State::Start,
            start: Default::default(),
            next: Default::default(),
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
    pub fn try_exhaustive(
        mut self,
    ) -> Result<impl Iterator<Item = Located<Token>>, impl Iterator<Item = Located<LexerError>>>
    {
        let mut tokens = Vec::new();

        while let Some(result) = self.next() {
            match result {
                Ok(token) => tokens.push(token),
                Err(error) => {
                    drop(tokens);

                    let mut errors = vec![error];
                    errors.extend(self.filter_map(Result::err));

                    return Err(errors.into_iter());
                }
            }
        }

        Ok(tokens.into_iter())
    }

    /// Intenta construir un siguiente token.
    fn lex(&mut self) -> Result<Option<Token>, LexerError> {
        use {State::*, Token::*};

        let token = loop {
            // Se espera un siguiente carácter, fallando si hay error de E/S
            let next_char = match self.source.peek() {
                None => None,
                Some(Ok(c)) => Some(*c),
                Some(Err(_)) => break Err(self.source.next().unwrap().unwrap_err().into()),
            };

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
                    *accumulated = *accumulated * 10 + digit.to_digit(10).unwrap() as i32;
                }

                // Si sigue algo que no es un dígito, la constante a terminado
                (Integer(integer), _) => break Ok(IntLiteral(*integer)),

                // Fin de literales de cadena
                (StringChars(string), Some('"')) => {
                    self.state = Complete(StrLiteral(std::mem::take(string)))
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
                    } else if word.chars().nth(0).unwrap().is_ascii_uppercase() {
                        self.start = self.next;
                        break Err(LexerError::UppercaseId);
                    } else {
                        break Ok(Id(Identifier(std::mem::take(word))));
                    }
                }
            }

            // Si no hubo `continue`, aquí se consume el carácter que
            // se observó con lookahead anteriormente
            self.source.next();

            // Cambios en la posición del cursor
            match next_char {
                Some('\n') => self.next = self.next.newline(),
                Some(_) => self.next = self.next.advance(),
                None => (),
            }
        };

        token.map(Some)
    }
}

impl<S: InputStream> Iterator for Lexer<S> {
    type Item = Result<Located<Token>, Located<LexerError>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lex() {
            Ok(None) => None,
            Ok(Some(token)) => {
                let range = self.start..self.next;
                let next = Located::at(token, self.from.clone(), range);

                self.start = self.next;
                self.state = State::Start;

                Some(Ok(next))
            }

            Err(error) => {
                self.state = State::Error;

                let range = self.next..self.next.advance();
                Some(Err(Located::at(error, self.from.clone(), range)))
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
