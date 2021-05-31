//! Análisis sintáctico.

use std::{iter::Peekable, marker::PhantomData};
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
    If {
        condition: Located<Expr>,
        body: Vec<Statement>,
    },

    For {
        variable: Located<Identifier>,
        iterable: Located<Expr>,
        step: Option<Located<Expr>>,
        body: Vec<Statement>,
    },

    UserCall {
        procedure: Located<Identifier>,
        args: Vec<Located<Expr>>,
    },

    Assignment {
        targets: Vec<Located<Target>>,
        values: Vec<Located<Expr>>,
    },

    MethodCall {
        target: Located<Target>,
        method: Located<Identifier>,
        args: Vec<Located<Expr>>,
    },
}

#[derive(Debug)]
pub enum Expr {
    True(Located<()>),
    False(Located<()>),
    Integer(Located<i32>),
    Read(Located<Target>),
    Negate(Box<Located<Expr>>),
    Binary(Box<Located<Expr>>, BinOp, Box<Located<Expr>>),
}

#[derive(Copy, Clone, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Pow,
    Div,
    Mod,
    IntegerDiv,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(Debug)]
pub struct Target {
    variable: Located<Identifier>,
    indices: Vec<Located<Index>>,
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

    #[error("Expected any of `if`, `for`, `call`, assignment or method call")]
    ExpectedStatement,

    #[error("Expected any of `int`, `bool`, `list`")]
    ExpectedType,

    #[error("Expected an expression")]
    ExpectedExpr,

    #[error("Missing type annotation for procedure parameter")]
    MissingParameterType,

    #[error("Abrupt end of program")]
    UnexpectedEof,
}

pub trait TokenStream<'a> = Iterator<Item = &'a Located<Token>> + Clone;

pub fn parse<'a>(tokens: impl TokenStream<'a>) -> Result<Ast, Located<ParserError>> {
    let mut parser = Parser {
        tokens: tokens.peekable(),
        last_known: Location::default(),
        lifetime_hack: PhantomData,
    };

    parser.program().map_err(Failure::coerce)
}

#[derive(Clone)]
struct Parser<'a, I: TokenStream<'a>> {
    tokens: Peekable<I>,
    last_known: Location,
    lifetime_hack: PhantomData<&'a ()>,
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

impl<'a, I: TokenStream<'a>> Parser<'a, I> {
    fn program(&mut self) -> Parse<Ast> {
        let mut procedures = Vec::new();
        while self.tokens.peek().is_some() {
            procedures.push(self.procedure()?);
        }

        Ok(Ast(procedures))
    }

    fn procedure(&mut self) -> Parse<Procedure> {
        self.keyword(Keyword::Procedure)?;
        let name = self.id()?;

        self.expect(Token::OpenParen)?;
        let parameters = self.comma_separated(Parser::parameter, true)?;
        self.expect(Token::CloseParen)?;

        let statements = self.statement_block()?;

        Ok(Procedure {
            name,
            parameters,
            statements,
        })
    }

    fn parameter(&mut self) -> Parse<Parameter> {
        let name = self.id().map_err(Failure::weak)?;

        self.expect(Token::Colon)?;
        let of = self.typ()?;

        Ok(Parameter { name, of })
    }

    fn statement_block(&mut self) -> Parse<Vec<Statement>> {
        self.expect(Token::OpenCurly)?;

        let mut statements = Vec::new();
        loop {
            match self.attempt(Parser::statement) {
                Ok(statement) => statements.push(statement),
                Err(Failure::Weak(error)) => {
                    self.expect(Token::CloseCurly)
                        .map_err(|_| Failure::Strict(error))?;

                    break Ok(statements);
                }

                Err(error) => break Err(error),
            }
        }
    }

    fn statement(&mut self) -> Parse<Statement> {
        match self.lookahead(|s| s.next().map(Located::into_inner))? {
            Token::Keyword(Keyword::If) => self.if_statement(),
            Token::Keyword(Keyword::For) => self.for_statement(),
            Token::Keyword(Keyword::Call) => self.user_call(),

            Token::Id(_) => {
                let targets = self.comma_separated(Parser::target, false)?;
                match self.lookahead(|s| s.expect(Token::Assign).map_err(Failure::weak)) {
                    Err(Failure::Weak(_)) if targets.len() == 1 => {
                        self.method_call(targets.into_iter().next().unwrap())
                    }

                    result => {
                        result?;
                        self.assignment(targets)
                    }
                }
            }

            _ => {
                self.next()?;
                self.fail(ParserError::ExpectedStatement)
                    .map_err(Failure::weak)
            }
        }
    }

    fn if_statement(&mut self) -> Parse<Statement> {
        self.keyword(Keyword::If)?;
        let condition = self.expr().map_err(Failure::strict)?;
        let body = self.statement_block()?;

        Ok(Statement::If { condition, body })
    }

    fn for_statement(&mut self) -> Parse<Statement> {
        self.keyword(Keyword::For)?;
        let variable = self.id()?;

        self.keyword(Keyword::In)?;
        let iterable = self.expr().map_err(Failure::strict)?;

        let step = match self.attempt(|s| s.keyword(Keyword::Step).map_err(Failure::weak)) {
            Err(Failure::Weak(_)) => None,
            result => {
                result?;
                Some(self.expr().map_err(Failure::strict)?)
            }
        };

        let body = self.statement_block()?;

        Ok(Statement::For {
            variable,
            iterable,
            step,
            body,
        })
    }

    fn user_call(&mut self) -> Parse<Statement> {
        self.keyword(Keyword::Call)?;
        let (procedure, args) = self.id_call()?;
        self.expect(Token::Semicolon)?;

        Ok(Statement::UserCall { procedure, args })
    }

    fn method_call(&mut self, target: Located<Target>) -> Parse<Statement> {
        self.expect(Token::Period)?;
        let (method, args) = self.id_call()?;
        self.expect(Token::Semicolon)?;

        Ok(Statement::MethodCall {
            target,
            method,
            args,
        })
    }

    fn assignment(&mut self, targets: Vec<Located<Target>>) -> Parse<Statement> {
        self.expect(Token::Assign)?;
        let values = self.comma_separated(Parser::expr, false)?;
        self.expect(Token::Semicolon)?;

        Ok(Statement::Assignment { targets, values })
    }

    fn id_call(&mut self) -> Parse<(Located<Identifier>, Vec<Located<Expr>>)> {
        let id = self.id()?;
        let args = match self.attempt(|s| s.expect(Token::OpenParen).map_err(Failure::weak)) {
            Err(Failure::Weak(_)) => Vec::new(),

            result => {
                result?;

                let args = self.comma_separated(Parser::expr, true)?;
                self.expect(Token::CloseParen)?;
                args
            }
        };

        Ok((id, args))
    }

    fn target(&mut self) -> Parse<Located<Target>> {
        let variable = self.id()?;

        let mut indices = Vec::new();
        loop {
            match self.attempt(Parser::index) {
                Err(Failure::Weak(_)) => break,
                index => indices.push(index?),
            }
        }

        let id_location = variable.location().clone();
        let location = match indices.last() {
            Some(last) => Location::span(id_location, last.location()),
            None => id_location,
        };

        Ok(Located::at(Target { variable, indices }, location))
    }

    fn index(&mut self) -> Parse<Located<Index>> {
        self.expect(Token::OpenSquare).map_err(Failure::weak)?;
        let start = self.last_known.clone();

        let first = self.selector()?;
        let index = match self.attempt(|s| s.expect(Token::Comma).map_err(Failure::weak)) {
            Err(Failure::Weak(_)) => Index::Direct(first),

            result => {
                result?;
                let second = self.selector()?;

                Index::Indirect(first, second)
            }
        };

        self.expect(Token::CloseSquare)?;
        let end = &self.last_known;

        Ok(Located::at(index, Location::span(start, end)))
    }

    fn selector(&mut self) -> Parse<Selector> {
        let start = self.optional(Parser::expr)?;
        let colon = self.attempt(|s| s.expect(Token::Colon).map_err(Failure::weak));

        match (start, colon) {
            (Some(start), Err(Failure::Weak(_))) => Ok(Selector::Single(start)),

            (start, result) => {
                result.map_err(Failure::strict)?;
                let end = self.optional(Parser::expr)?;

                Ok(Selector::Range { start, end })
            }
        }
    }

    fn typ(&mut self) -> Parse<Located<Type>> {
        let (location, token) = self.next()?.split();
        let typ = match token {
            Token::Keyword(Keyword::Int) => Type::Int,
            Token::Keyword(Keyword::Bool) => Type::Bool,
            Token::Keyword(Keyword::List) => Type::List,

            _ => self.fail(ParserError::ExpectedType)?,
        };

        Ok(Located::at(typ, location))
    }

    fn expr(&mut self) -> Parse<Located<Expr>> {
        //TODO
        let (location, token) = self.next()?.split();
        match token {
            Token::IntLiteral(integer) => Ok(Located::at(
                Expr::Integer(Located::at(integer, location.clone())),
                location.clone(),
            )),

            _ => self.fail(ParserError::ExpectedExpr).map_err(Failure::weak),
        }
    }

    fn optional<T, F>(&mut self, rule: F) -> Parse<Option<T>>
    where
        F: FnOnce(&mut Self) -> Parse<T>,
    {
        match self.attempt(rule) {
            Err(Failure::Weak(_)) => Ok(None),
            result => Ok(Some(result?)),
        }
    }

    fn attempt<T, F>(&mut self, rule: F) -> Parse<T>
    where
        F: FnOnce(&mut Self) -> Parse<T>,
    {
        let mut fork = self.clone();

        let result = rule(&mut fork);
        if result.is_ok() {
            *self = fork;
        }

        result
    }

    fn lookahead<T, F>(&mut self, rule: F) -> Parse<T>
    where
        F: FnOnce(&mut Self) -> Parse<T>,
    {
        rule(&mut self.clone())
    }

    fn comma_separated<T, F>(&mut self, mut rule: F, allow_empty: bool) -> Parse<Vec<T>>
    where
        F: FnMut(&mut Self) -> Parse<T>,
    {
        let mut items = match self.attempt(|s| rule(s)) {
            Err(Failure::Weak(_)) if allow_empty => return Ok(Vec::new()),
            item => vec![item.map_err(Failure::strict)?],
        };

        loop {
            match self.attempt(|s| s.expect(Token::Comma).map_err(Failure::weak)) {
                Err(Failure::Weak(_)) => break Ok(items),
                result => {
                    result?;
                    items.push(rule(self).map_err(Failure::strict)?);
                }
            }
        }
    }

    fn id(&mut self) -> Parse<Located<Identifier>> {
        let (location, token) = self.next()?.split();
        match token {
            Token::Id(id) => Ok(Located::at(id, location)),
            _ => self.fail(ParserError::ExpectedId),
        }
    }

    fn keyword(&mut self, keyword: Keyword) -> Parse<()> {
        self.expect(Token::Keyword(keyword))
    }

    fn expect(&mut self, token: Token) -> Parse<()> {
        match self.next().map(Located::into_inner) {
            Ok(found) if found == token => Ok(()),
            Ok(found) => self.fail(ParserError::UnexpectedToken(token, found)),
            Err(_) => self.fail(ParserError::MissingToken(token)),
        }
    }

    fn next(&mut self) -> Parse<Located<Token>> {
        match self.tokens.next() {
            Some(token) => {
                self.last_known = token.location().clone();
                Ok(token.clone())
            }

            None => self.fail(ParserError::UnexpectedEof),
        }
    }

    fn fail<T>(&self, error: ParserError) -> Parse<T> {
        Err(Failure::Strict(Located::at(error, self.last_known.clone())))
    }
}
