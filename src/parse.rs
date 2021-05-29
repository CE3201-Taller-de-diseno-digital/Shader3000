use std::iter::Peekable;

use crate::{
    lex::{Identifier, Keyword, Lexer, LexerError, Token},
    source::{InputStream, Located, Location, Position},
};
use thiserror::Error;

pub struct Ast(Vec<Procedure>);

pub struct Procedure {
    name: Located<Identifier>,
    parameters: Vec<Parameter>,
    statements: Vec<Statement>,
}

pub struct Parameter {
    name: Located<Identifier>,
    of: Located<Type>,
}

pub enum Type {
    Int,
    Bool,
    List(Box<Located<Type>>),
}

pub enum Statement {
    Expr(Expr),

    Assignment {
        target: Located<Identifier>,
        indices: Vec<Index>,
        value: Expr,
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

pub enum Index {
    Direct(Selector),
    Indirect(Selector, Selector),
}

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
    #[error("Expected  \",\" or \")\" ")]
    MissingSeparationToken,
    #[error("No Identifier defined for procedure")]
    MissingId,
    #[error("Missing type anotation for procedure parameter")]
    MissingParameterType,
    #[error("Token found is not allowed in parameter block")]
    InvalidParameterToken,
    #[error("abrupt end of program")]
    IncompleteBlock,
}
//#[derive(Clone)]
//struct Parser<I: Iterator<Item = Located<Token>> + Clone> {
//    tokens: Peekable<I>,
//    last_pos: Option<Location>,
//}
//
//impl<I: Iterator<Item = Located<Token>> + Clone> Parser<I> {
//    pub fn new(input: I)->Self{
//        Parser{
//            tokens: input.peekable(),
//            last_pos: None
//        }
//    }
//    pub fn expect_token(
//        expected: Token,
//        tokens: &Peekable<impl Iterator<Item = Located<Token>>>,
//        last_pos: Location,
//    ) -> Result<(Located<Token>, Location), Located<ParserError>> {
//        match tokens.next() {
//            Some(token) => {
//                let found = token.as_ref().clone();
//                match found {
//                    expected => Ok((token, token.location().clone())),
//                    _ => Err(Located::from_location(
//                        ParserError::UnexpectedToken(expected, found),
//                        token.location(),
//                    )),
//                }
//            }
//            None => Err(Located::from_location(
//                ParserError::MissingToken(expected),
//                &last_pos,
//            )),
//        }
//    }
//    pub fn try_token(
//        expected: Token,
//        tokens: &Peekable<impl Iterator<Item = Located<Token>>>,
//    ) -> bool {
//        match tokens.peek() {
//            Some(token) => {
//                let found = token.as_ref().clone();
//                match found {
//                    expected => true,
//                    _ => false,
//                }
//            }
//            None => false,
//        }
//    }
//
//    pub fn parse_procedure(
//        tokens: &Peekable<impl Iterator<Item = Located<Token>>>,
//        last_pos: Location,
//    ) -> Result<Procedure, Located<ParserError>> {
//        let name = parse_id(tokens, last_pos)?;
//        let (_, mut last_pos) = expect_token(Token::OpenParen, tokens, name.1)?;
//        let name = name.0;
//        let parameters: Vec<Parameter> = Vec::new();
//        let statements: Vec<Statement> = Vec::new();
//        //parse parameters
//        loop {
//            let (parameter, feedback_pos) = parse_paramater(tokens, last_pos)?;
//            last_pos = feedback_pos;
//            parameters.push(parameter);
//            let decision_token = tokens.next();
//            match decision_token {
//                Some(token) => match token.as_ref() {
//                    Token::CloseParen => break,
//                    Token::Comma => continue,
//                    _ => {
//                        return Err(Located::from_location(
//                            ParserError::MissingSeparationToken,
//                            token.location(),
//                        ))
//                    }
//                },
//                None => {
//                    return Err(Located::from_location(
//                        ParserError::IncompleteBlock,
//                        &last_pos,
//                    ))
//                }
//            }
//        }
//        //parse statements
//
//        //assemble procedure
//        Ok(Procedure {
//            name,
//            parameters,
//            statements,
//        })
//    }
//    pub fn parse_paramater(
//        tokens: &Peekable<impl Iterator<Item = Located<Token>>>,
//        last_pos: Location,
//    ) -> Result<(Parameter, Location), Located<ParserError>> {
//        let (name, last_pos) = parse_id(tokens, last_pos)?;
//        let (_, last_post) = expect_token(Token::Colon, tokens, last_pos)?;
//        let (of, last_pos) = parse_type(tokens, last_pos)?;
//        Ok((Parameter { name, of }, last_pos))
//    }
//
//    pub fn parse_id(
//        tokens: &Peekable<impl Iterator<Item = Located<Token>>>,
//        last_pos: Location,
//    ) -> Result<(Located<Identifier>, Location), Located<ParserError>> {
//        match tokens.next() {
//            Some(token) => {
//                let found = token.as_ref().clone();
//                match found {
//                    Token::Id(identifier) => Ok((
//                        Located::from_one(identifier, token),
//                        token.location().clone(),
//                    )),
//                    _ => Err(Located::from_location(
//                        ParserError::MissingId,
//                        token.location(),
//                    )),
//                }
//            }
//            None => Err(Located::from_location(
//                ParserError::IncompleteBlock,
//                &last_pos,
//            )),
//        }
//    }
//    pub fn parse_type(
//        tokens: &Peekable<impl Iterator<Item = Located<Token>>>,
//        last_pos: Location,
//    ) -> Result<(Located<Type>, Location), Located<ParserError>> {
//        //int, bool or List:bool/int/List
//        if let Some(token) = tokens.next() {
//            match token.as_ref() {
//                Token::Keyword(Keyword::Int) => Ok(Located::from_one(identifier.clone(), token)),
//                _ => Err(Located::from_one(ParserError::MissingId, token)),
//            }
//        } else {
//            Err(Located::from_one(ParserError::MissingId, last_token))
//        }
//    }
//
//    //pub fn parse_expression(
//    //    tokens: &Peekable<impl Iterator<Item=Located<Token>>>,last_pos: Location
//    //) -> Result<Expr, Located<ParserError>> {
//    //}
//    //pub fn parse_statement(
//    //    tokens: &Peekable<impl Iterator<Item=Located<Token>>>,last_pos: Location
//    //) -> Result<Statement, Located<ParserError>> {
//    //}
//    //pub fn parse_index(
//    //    tokens: &Peekable<impl Iterator<Item=Located<Token>>>,last_pos: Location
//    //) -> Result<Index, Located<ParserError>> {
//    //}
//    //pub fn parse_selector(
//    //    tokens: &Peekable<impl Iterator<Item=Located<Token>>>,last_pos: Location
//    //) -> Result<Selector, Located<ParserError>> {
//    //}
//
//    //tokens se obtiene con instancia de Lexer usando try_exhaustive
//    pub fn parse(
//        tokens: impl Iterator<Item = Located<Token>>,
//    ) -> Result<Ast, Located<ParserError>> {
//        let mut tokens = tokens.peekable();
//        let mut procedures: Vec<Procedure> = Vec::new();
//
//        while let Some(last_token) = tokens.next() {
//            let token = last_token.as_ref();
//            //check for mandatory token keyword
//            match token {
//                Token::Keyword(Keyword::Procedure) => {
//                    procedures.push(parse_procedure(&tokens, last_token.location().clone())?)
//                }
//                _ => {
//                    let error = ParserError::UnexpectedToken(
//                        Token::Keyword(Keyword::Procedure),
//                        token.clone(),
//                    );
//                    return Err(Located::from_one(error, last_token));
//                }
//            }
//        }
//        Ok(Ast(procedures))
//    }
//}

