use crate::{lex::Identifier, source::Located};

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
