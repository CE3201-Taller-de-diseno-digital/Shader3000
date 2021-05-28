use crate::source::{InputStream, Located, Position, SourceName};
use std::str::FromStr;
use thiserror::Error;

pub const MAX_ID_LENGTH: usize = 10;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum LexerError {
    #[error("I/O error")]
    Input(#[from] std::io::Error),

    #[error("Bad character {0:?} in input stream")]
    BadChar(char),

    #[error("Expected {0:?}")]
    Expected(char),

    #[error("Bad escape sequence")]
    BadEscape,

    #[error("Unterminated string literal")]
    UnterminatedString,

    #[error("identifier exceeds {MAX_ID_LENGTH} characters")]
    IdTooLong,

    #[error("identifiers must begin with a lowercase letter")]
    UppercaseId,
}

#[derive(Debug)]
pub struct Identifier(String);

#[derive(Debug)]
pub enum Token {
    Id(Identifier),
    Keyword(Keyword),
    StrLiteral(String),
    IntLiteral(i32),
    Assign,
    Comma,
    Plus,
    Minus,
    Times,
    Pow,
    Div,
    IntegerDiv,
    Mod,
    Colon,
    Semicolon,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    OpenParen,
    OpenSquare,
    OpenCurly,
    CloseParen,
    CloseSquare,
    CloseCurly,
}

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

pub struct Lexer<S: Iterator> {
    source: std::iter::Peekable<S>,
    from: SourceName,
    state: State,
    start: Position,
    next: Position,
}

enum State {
    Start,
    Error,
    Complete(Token),
    Hash,
    Star,
    Slash,
    AssignOrEqual,
    LeftAngle,
    RightAngle,
    Comment,
    Integer(i32),
    StringChars(String),
    Word(String),
}

impl<S: InputStream> Lexer<S> {
    pub fn new(source: S, from: SourceName) -> Self {
        Lexer {
            from,
            source: source.peekable(),
            state: State::Start,
            start: Default::default(),
            next: Default::default(),
        }
    }

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

    fn lex(&mut self) -> Result<Option<Token>, LexerError> {
        use {State::*, Token::*};

        let token = loop {
            let next_char = match self.source.peek() {
                None => None,
                Some(Ok(c)) => Some(*c),
                Some(Err(_)) => break Err(self.source.next().unwrap().unwrap_err().into()),
            };

            match (&mut self.state, next_char) {
                (Error, None) => return Ok(None),
                (Error, Some('\n')) => self.state = Start,
                (Error, Some(_)) => (),

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
                (Start, Some('"')) => self.state = StringChars(String::new()),
                (Start, Some(c)) if c.is_ascii_alphabetic() => self.state = Word(c.to_string()),

                (Start, Some(c)) if c.is_ascii_digit() => {
                    self.state = Integer(0);
                    continue;
                }

                (Start, Some(c)) if c.is_ascii_whitespace() => (),
                (Start, Some(c)) => break Err(LexerError::BadChar(c)),

                (Complete(value), _) => break Ok(std::mem::replace(value, Plus)),

                (Hash, Some('#')) => self.state = Comment,
                (Hash, _) => break Err(LexerError::Expected('#')),

                (Star, Some('*')) => self.state = Complete(Pow),
                (Star, _) => break Ok(Times),

                (Slash, Some('/')) => self.state = Complete(IntegerDiv),
                (Slash, _) => break Ok(Div),

                (AssignOrEqual, Some('=')) => self.state = Complete(Equal),
                (AssignOrEqual, _) => break Ok(Assign),

                (LeftAngle, Some('=')) => self.state = Complete(LessOrEqual),
                (LeftAngle, Some('>')) => self.state = Complete(NotEqual),
                (LeftAngle, _) => break Ok(Less),

                (RightAngle, Some('=')) => self.state = Complete(GreaterOrEqual),
                (RightAngle, _) => break Ok(Greater),

                (Comment, Some('\n')) => self.state = Start,
                (Comment, Some(_)) => (),
                (Comment, None) => self.state = Start,

                (Integer(accumulated), Some(digit)) if digit.is_ascii_digit() => {
                    *accumulated = *accumulated * 10 + digit.to_digit(10).unwrap() as i32;
                }

                (Integer(integer), _) => break Ok(IntLiteral(*integer)),

                (StringChars(string), Some('"')) => {
                    self.state = Complete(StrLiteral(std::mem::take(string)))
                }

                (StringChars(_), Some('\\')) => break Err(LexerError::BadEscape),
                (StringChars(string), Some(c)) if is_string_char(c) => string.push(c),
                (StringChars(_), _) => break Err(LexerError::UnterminatedString),

                (Word(word), Some(c)) if is_word_char(c) => {
                    if word.len() == MAX_ID_LENGTH {
                        break Err(LexerError::IdTooLong);
                    }

                    word.push(c);
                }

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

            self.source.next();
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

fn is_string_char(c: char) -> bool {
    c == '_' || (!c.is_control() && !c.is_whitespace())
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c.is_ascii_digit() || matches!(c, '@' | '_' | '?')
}
