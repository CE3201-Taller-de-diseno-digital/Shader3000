//! Análisis sintáctico.

use std::iter::Peekable;
use thiserror::Error;

use crate::{
    lex::{Identifier, Keyword, Token},
    source::{Located, Location},
};

#[derive(Debug)]
pub struct Ast(Vec<Procedure>);

#[derive(Debug)]
pub struct Procedure {
    name: Located<Identifier>,
    parameters: Vec<Parameter>,
    statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct Parameter {
    name: Located<Identifier>,
    of: Located<Type>,
}

#[derive(Debug)]
pub enum Type {
    Int,
    Bool,
    List,
}

#[derive(Debug)]
pub enum Statement {
    Expr(Expr),

    Assignment {
        targets: Vec<Target>,
        values: Vec<Expr>,
    },

    If {
        condition: Located<Expr>,
        body: Vec<Statement>,
    },

    For {
        variable: Located<Identifier>,
        iterable: Located<Expr>,
        step: Option<Located<Expr>>,
    },
}

#[derive(Debug)]
pub enum Expr {
    True,
    False,
    Integer(i32),
    Read(Identifier),

    Index {
        base: Box<Expr>,
        index: Box<Index>,
    },

    UserCall {
        target: Located<Identifier>,
        arguments: Vec<Located<Expr>>,
    },
}

#[derive(Debug)]
pub struct Target {
    variable: Located<Identifier>,
    indices: Vec<Index>,
}

#[derive(Debug)]
pub enum Index {
    Direct(Selector),
    Indirect(Selector, Selector),
}

#[derive(Debug)]
pub enum Selector {
    Single(Located<Expr>),
    Range {
        start: Option<Located<Expr>>,
        end: Option<Located<Expr>>,
    },
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("No parameters were especified for procedure")]
    MissingProcedureParameters,

    #[error("Missing operand in sequence")]
    MissingOperand,

    #[error("Expected token {0:?}, found {1:?} instead")]
    UnexpectedToken(Token, Token),

    #[error("Expected token {0:?}, none was found instead")]
    MissingToken(Token),

    #[error("Expected  \",\" or \")\"")]
    MissingSeparationToken,

    #[error("Expected identifier")]
    ExpectedId,

    #[error("Expected any of `int`, `bool`, `list`")]
    ExpectedType,

    #[error("Missing type annotation for procedure parameter")]
    MissingParameterType,

    #[error("Abrupt end of program")]
    UnexpectedEof,
}

pub trait TokenStream = Iterator<Item = Located<Token>>;

pub fn parse(tokens: impl TokenStream) -> Result<Ast, Located<ParserError>> {
    let mut parser = Parser {
        tokens: tokens.peekable(),
        last_known: Location::default(),
    };

    parser.program().map_err(Failure::coerce)
}

struct Parser<I: TokenStream> {
    tokens: Peekable<I>,
    last_known: Location,
}

enum Failure {
    Weak(Located<ParserError>),
    Strict(Located<ParserError>),
}

impl Failure {
    fn weak(self) -> Self {
        Failure::Weak(self.coerce())
    }

    fn strict(self) -> Self {
        Failure::Strict(self.coerce())
    }

    fn coerce(self) -> Located<ParserError> {
        match self {
            Failure::Weak(error) => error,
            Failure::Strict(error) => error,
        }
    }
}

type Parse<T> = Result<T, Failure>;

impl<I: TokenStream> Parser<I> {
    fn program(&mut self) -> Parse<Ast> {
        let mut procedures = Vec::new();
        while self.tokens.peek().is_some() {
            procedures.push(self.procedure()?);
        }

        Ok(Ast(procedures))
    }

    fn procedure(&mut self) -> Parse<Procedure> {
        self.keyword(Keyword::Procedure)?;
        let name = self.identifier()?;

        self.expect(Token::OpenParen)?;
        let parameters = self.comma_separated(Parser::parameter)?;
        self.expect(Token::CloseParen)?;

        let statements = self.statement_block()?;

        Ok(Procedure {
            name,
            parameters,
            statements,
        })
    }

    fn parameter(&mut self) -> Parse<Parameter> {
        let name = self.identifier().map_err(Failure::weak)?;

        self.expect(Token::Colon)?;
        let of = self.typ()?;

        Ok(Parameter { name, of })
    }

    fn statement_block(&mut self) -> Parse<Vec<Statement>> {
        self.expect(Token::OpenCurly)?;

        let mut statements = Vec::new();
        loop {
            match self.statement() {
                Ok(statement) => statements.push(statement),
                Err(Failure::Weak(_)) => {
                    self.expect(Token::CloseCurly)?;
                    break Ok(statements);
                }

                Err(error) => break Err(error),
            }
        }
    }

    fn statement(&mut self) -> Parse<Statement> {
        self.expect(Token::Semicolon).map_err(Failure::weak)?;
        Ok(Statement::Expr(Expr::True))
    }

    fn typ(&mut self) -> Parse<Located<Type>> {
        let typ = match self.peek()? {
            Token::Keyword(Keyword::Int) => Type::Int,
            Token::Keyword(Keyword::Bool) => Type::Bool,
            Token::Keyword(Keyword::List) => Type::List,

            _ => return self.fail(ParserError::ExpectedType),
        };

        Ok(self.next()?.map(|_| typ))
    }

    fn comma_separated<T, F>(&mut self, mut rule: F) -> Parse<Vec<T>>
    where
        F: FnMut(&mut Self) -> Parse<T>,
    {
        let mut items = match rule(self) {
            Err(Failure::Weak(_)) => return Ok(Vec::new()),
            item => vec![item?],
        };

        loop {
            match self.expect(Token::Comma).map_err(Failure::weak) {
                Err(Failure::Weak(_)) => break Ok(items),
                result => {
                    result?;
                    items.push(rule(self).map_err(Failure::strict)?);
                }
            }
        }
    }

    fn identifier(&mut self) -> Parse<Located<Identifier>> {
        match self.peek()? {
            Token::Id(id) => {
                let id = id.clone();
                Ok(self.next()?.map(|_| id))
            }

            _ => self.fail(ParserError::ExpectedId),
        }
    }

    fn keyword(&mut self, keyword: Keyword) -> Parse<()> {
        self.expect(Token::Keyword(keyword))
    }

    fn expect(&mut self, token: Token) -> Parse<()> {
        match self.peek() {
            Ok(found) if *found == token => {
                self.next()?;
                Ok(())
            }

            Ok(found) => {
                let found = found.clone();
                self.fail(ParserError::UnexpectedToken(token, found))
            }

            Err(_) => self.fail(ParserError::MissingToken(token)),
        }
    }

    fn peek(&mut self) -> Parse<&Token> {
        match self.tokens.peek() {
            Some(token) => {
                self.last_known = token.location().clone();
                Ok(token.as_ref())
            }

            None => todo!(),
        }
    }

    fn next(&mut self) -> Parse<Located<Token>> {
        match self.tokens.next() {
            Some(token) => {
                self.last_known = token.location().clone();
                Ok(token)
            }

            None => self.fail(ParserError::UnexpectedEof),
        }
    }

    fn fail<T>(&self, error: ParserError) -> Parse<T> {
        Err(Failure::Strict(Located::at(error, self.last_known.clone())))
    }
}
