use thiserror::Error;

use std::{
    collections::HashMap,
    fmt::{self, Display},
    rc::Rc,
};

use crate::{
    ir::{self, Function, Global, Instruction, Local},
    lex::Identifier,
    parse,
    source::Located,
};

struct SymbolTable<'a> {
    outer: Option<&'a mut SymbolTable<'a>>,
    symbols: HashMap<Identifier, Named>,
}

impl SymbolTable<'_> {
    fn lookup(&self, id: &Located<Identifier>) -> Semantic<&Named> {
        self.symbols.get(id).ok_or_else(|| {
            Located::at(
                SemanticError::Undefined(id.as_ref().clone()),
                id.location().clone(),
            )
        })
    }
}

enum Named {
    Var(Variable),
    Proc(Procedure),
}

#[derive(Clone)]
struct Variable {
    access: Access,
    typ: Type,
}

struct Procedure {
    symbol: Rc<String>,
    parameters: Vec<Type>,
}

#[derive(Clone)]
enum Access {
    Global(Global),
    Local(Local),
}

#[derive(Copy, Clone, Debug)]
pub enum Type {
    Int,
    Bool,
    List,
    Mat,
}

impl Display for Type {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => fmt.write_str("int"),
            Type::Bool => fmt.write_str("bool"),
            Type::List => fmt.write_str("list"),
            Type::Mat => fmt.write_str("mat"),
        }
    }
}

#[derive(Copy, Clone)]
enum Ownership {
    Owned,
    Borrowed,
}

trait Sink<'a> {
    fn symbols(&self) -> &SymbolTable<'a>;

    fn push(&mut self, instruction: Instruction);

    fn ephemeral<F, R>(&mut self, callback: F) -> Semantic<R>
    where
        F: FnOnce(&mut Self, Local) -> Semantic<(Type, Ownership, R)>;
}

struct TypeCheck<'a>(&'a mut SymbolTable<'static>);

impl Sink<'static> for TypeCheck<'_> {
    fn symbols(&self) -> &SymbolTable<'static> {
        &self.0
    }

    fn push(&mut self, _instruction: Instruction) {}

    fn ephemeral<F, R>(&mut self, callback: F) -> Semantic<R>
    where
        F: FnOnce(&mut Self, Local) -> Semantic<(Type, Ownership, R)>,
    {
        let (_, _, result) = callback(self, Local(42))?;
        Ok(result)
    }
}

pub type Semantic<T> = Result<T, Located<SemanticError>>;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum SemanticError {
    #[error("Entrypoint not found, define a parameterless`procedure main()`")]
    NoMain,

    #[error("Mismatch in number of targets and values")]
    UnbalancedAssignment,

    #[error("Type mismatch: expected `{0}`, found `{1}`")]
    ExpectedType(Type, Type),

    #[error("Type mismatch: expected `list` or `mat`, found `{0}`")]
    ExpectedListOrMat(Type),

    #[error("Expected variable, found procedure family `{0}`")]
    ExpectedVar(Identifier),

    #[error("Symbol `{0}` is undefined")]
    Undefined(Identifier),
}

impl parse::Ast {
    pub fn resolve(self) -> Semantic<ir::Program> {
        let scope = scan_global_scope(&self)?;

        Ok(ir::Program {
            code: vec![],
            globals: vec![],
        })
    }
}

fn scan_global_scope(ast: &parse::Ast) -> Semantic<SymbolTable<'static>> {
    let main = ast
        .iter()
        .find(|proc| {
            let id = proc.name().as_ref();
            unicase::eq_ascii(id.as_ref(), "main") && proc.parameters().is_empty()
        })
        .ok_or_else(|| Located::at(SemanticError::NoMain, ast.eof().clone()))?;

    let mut globals = SymbolTable {
        outer: None,
        symbols: HashMap::new(),
    };

    let mut statements = main.statements().iter();
    while let Some(parse::Statement::Assignment { targets, values }) = statements.next() {
        for (target, value) in break_assignment(targets, values)? {
            // Inicialmente solo se consideran definiciones y no asignaciones
            let id = target.var().as_ref();
            if globals.symbols.get(id).is_none() && target.indices().is_empty() {
                // Esto solo verifica e infiere tipos, todavía no se genera IR
                let (typ, _) = eval(value, Local(42), &mut TypeCheck(&mut globals))?;

                let var = Variable {
                    access: Access::Global(Global::from(mangle(id, &[]))),
                    typ,
                };

                globals.symbols.insert(id.clone(), Named::Var(var));
            }
        }
    }

    Ok(globals)
}

fn break_assignment<'a>(
    targets: &'a [Located<parse::Target>],
    values: &'a [Located<parse::Expr>],
) -> Semantic<impl Iterator<Item = (&'a Located<parse::Target>, &'a Located<parse::Expr>)>> {
    let error_location = if targets.len() > values.len() {
        targets[values.len()].location()
    } else if targets.len() < values.len() {
        values[targets.len()].location()
    } else {
        return Ok(targets.iter().zip(values.iter()));
    };

    Err(Located::at(
        SemanticError::UnbalancedAssignment,
        error_location.clone(),
    ))
}

fn eval<'a, S: Sink<'a>>(
    expr: &Located<parse::Expr>,
    into: Local,
    sink: &mut S,
) -> Semantic<(Type, Ownership)> {
    use parse::Expr::*;
    use Ownership::Owned;

    match expr.as_ref() {
        True => {
            sink.push(Instruction::LoadConst(1, into));
            Ok((Type::Bool, Owned))
        }

        False => {
            sink.push(Instruction::LoadConst(0, into));
            Ok((Type::Bool, Owned))
        }

        Integer(constant) => {
            sink.push(Instruction::LoadConst(*constant, into));
            Ok((Type::Int, Owned))
        }

        Read(target) => read(target, into, sink),

        Len(expr) => sink.ephemeral(|sink, arg| {
            let (arg_type, arg_ownership) = eval(expr, arg, sink)?;
            let target = match arg_type {
                Type::List => Function::External("builtin_len_list"),

                _ => {
                    return Err(Located::at(
                        SemanticError::ExpectedType(Type::List, arg_type),
                        expr.location().clone(),
                    ))
                }
            };

            sink.push(Instruction::Call {
                target,
                arguments: vec![arg],
                output: Some(into),
            });

            Ok((arg_type, arg_ownership, (Type::Int, Owned)))
        }),

        _ => todo!(),
    }
}

fn read<'a, S: Sink<'a>>(
    target: &parse::Target,
    into: Local,
    sink: &mut S,
) -> Semantic<(Type, Ownership)> {
    let var = target.var();
    let var = match sink.symbols().lookup(var)? {
        Named::Var(var) => var,
        Named::Proc(_) => {
            return Err(Located::at(
                SemanticError::ExpectedVar(var.as_ref().clone()),
                var.location().clone(),
            ))
        }
    };

    let var = var.clone();

    match &var.access {
        Access::Local(local) => sink.push(Instruction::Move(*local, into)),
        Access::Global(global) => sink.push(Instruction::LoadGlobal(global.clone(), into)),
    }

    if !target.indices().is_empty() {
        todo!()
    }

    Ok((var.typ, Ownership::Borrowed))
}

fn mangle(name: &Identifier, types: &[Type]) -> String {
    let name = name.as_ref();

    let mut mangled = String::from("user_");
    mangled.reserve(name.len() + types.len());

    for c in name.chars().map(char::to_lowercase).flatten() {
        match c {
            '@' => mangled.push_str("$a$"),
            '?' => mangled.push_str("$q$"),
            _ => mangled.push(c),
        }
    }

    if !types.is_empty() {
        mangled.push_str("$$");
        mangled.extend(types.iter().map(|typ| match typ {
            Type::Int => 'i',
            Type::Mat => 'm',
            Type::Bool => 'b',
            Type::List => 'l',
        }));
    }

    mangled
}
