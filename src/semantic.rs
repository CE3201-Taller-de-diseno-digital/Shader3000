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
    Procs {
        variants: HashMap<Vec<Type>, Rc<String>>,
    },
}

#[derive(Clone)]
struct Variable {
    access: Access,
    typ: Type,
}

#[derive(Clone)]
enum Access {
    Global(Global),
    Local(Local),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

trait Sink {
    fn push(&mut self, instruction: Instruction);

    fn alloc_local(&mut self) -> Local;

    fn free_local(&mut self, local: Local);
}

struct TypeCheck;

impl Sink for TypeCheck {
    fn push(&mut self, _instruction: Instruction) {}

    fn alloc_local(&mut self) -> Local {
        Local(42)
    }

    fn free_local(&mut self, _local: Local) {}
}

pub type Semantic<T> = Result<T, Located<SemanticError>>;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum SemanticError {
    #[error("Entrypoint not found, define a parameterless `procedure main()`")]
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

    #[error("This definition for `{0}` is in conflict with a global variable")]
    NameClash(Identifier),

    #[error("Redefinition of procedure `{0}` with the same parameter types")]
    SignatureClash(Identifier),
}

impl parse::Ast {
    pub fn resolve(self) -> Semantic<ir::Program> {
        let mut global_scope = self.scan_global_scope()?;
        let mut context = Context {
            scope: &mut global_scope,
            sink: &mut TypeCheck,
        };

        let code = self
            .iter()
            .map(|procedure| context.scan_proc(procedure))
            .collect::<Result<Vec<_>, _>>()?;

        let globals = global_scope
            .symbols
            .into_iter()
            .filter_map(|(_, named)| match named {
                Named::Var(Variable {
                    access: Access::Global(global),
                    ..
                }) => Some(global),

                _ => None,
            })
            .collect();

        Ok(ir::Program { code, globals })
    }

    fn scan_global_scope(&self) -> Semantic<SymbolTable<'static>> {
        let main = self
            .iter()
            .find(|proc| {
                let id = proc.name().as_ref();
                unicase::eq_ascii(id.as_ref(), "main") && proc.parameters().is_empty()
            })
            .ok_or_else(|| Located::at(SemanticError::NoMain, self.eof().clone()))?;

        let mut globals = SymbolTable {
            outer: None,
            symbols: HashMap::new(),
        };

        let mut context = Context {
            scope: &mut globals,
            sink: &mut TypeCheck,
        };

        let mut statements = main.statements().iter();
        while let Some(parse::Statement::Assignment { targets, values }) = statements.next() {
            for (target, value) in break_assignment(targets, values)? {
                // Inicialmente solo se consideran definiciones y no asignaciones
                let id = target.var().as_ref();
                if context.scope.symbols.get(id).is_none() && target.indices().is_empty() {
                    // Esto solo verifica e infiere tipos, todavía no se genera IR
                    let (typ, _) = context.eval(value, Local(42))?;

                    let var = Variable {
                        access: Access::Global(Global::from(mangle(id, &[]))),
                        typ,
                    };

                    context.scope.symbols.insert(id.clone(), Named::Var(var));
                }
            }
        }

        for procedure in self.iter() {
            let types = context.parameter_types(procedure)?;

            let (location, name) = procedure.name().clone().split();
            let named = context
                .scope
                .symbols
                .entry(name)
                .or_insert_with(|| Named::Procs {
                    variants: HashMap::new(),
                });

            let id = procedure.name().as_ref();
            let symbol = Rc::new(mangle(id, &types));

            match named {
                Named::Var(_) => {
                    return Err(Located::at(SemanticError::NameClash(id.clone()), location))
                }

                Named::Procs { variants } => {
                    if variants.insert(types, symbol).is_some() {
                        return Err(Located::at(
                            SemanticError::SignatureClash(id.clone()),
                            location,
                        ));
                    }
                }
            }
        }

        Ok(globals)
    }
}

struct Context<'b, 'a: 'b, S: Sink> {
    scope: &'b mut SymbolTable<'a>,
    sink: &'b mut S,
}

impl<S: Sink> Context<'_, 'static, S> {
    fn scan_proc(&mut self, procedure: &parse::Procedure) -> Semantic<ir::GeneratedFunction> {
        let types = self.parameter_types(procedure)?;
        let symbol = match self.scope.symbols.get(procedure.name().as_ref()) {
            Some(Named::Procs { variants }) => variants.get(&types).unwrap().clone(),
            _ => unreachable!(),
        };

        Ok(ir::GeneratedFunction {
            name: symbol,
            body: Vec::new(),
            parameters: procedure.parameters().len() as u32,
        })
    }

    fn parameter_types(&mut self, procedure: &parse::Procedure) -> Semantic<Vec<Type>> {
        let mut type_check = Context {
            scope: self.scope,
            sink: &mut TypeCheck,
        };

        procedure
            .parameters()
            .iter()
            .map(|param| match param.of().as_ref() {
                parse::Type::Int => Ok(Type::Int),
                parse::Type::Bool => Ok(Type::Bool),
                parse::Type::List => Ok(Type::List),
                parse::Type::Of(expr) => {
                    let (typ, _) = type_check.eval(expr, Local(42))?;
                    Ok(typ)
                }
            })
            .collect()
    }
}

impl<S: Sink> Context<'_, '_, S> {
    fn eval(&mut self, expr: &Located<parse::Expr>, into: Local) -> Semantic<(Type, Ownership)> {
        use parse::Expr::*;
        use Ownership::Owned;

        match expr.as_ref() {
            True => {
                self.sink.push(Instruction::LoadConst(1, into));
                Ok((Type::Bool, Owned))
            }

            False => {
                self.sink.push(Instruction::LoadConst(0, into));
                Ok((Type::Bool, Owned))
            }

            Integer(constant) => {
                self.sink.push(Instruction::LoadConst(*constant, into));
                Ok((Type::Int, Owned))
            }

            Read(target) => self.read(target, into),

            Len(expr) => self.ephemeral(|this, arg| {
                let (arg_type, arg_ownership) = this.eval(expr, arg)?;
                let target = match arg_type {
                    Type::List => Function::External("builtin_len"),

                    _ => {
                        return Err(Located::at(
                            SemanticError::ExpectedType(Type::List, arg_type),
                            expr.location().clone(),
                        ))
                    }
                };

                this.sink.push(Instruction::Call {
                    target,
                    arguments: vec![arg],
                    output: Some(into),
                });

                Ok((arg_type, arg_ownership, (Type::Int, Owned)))
            }),

            _ => todo!(),
        }
    }

    fn read(&mut self, target: &parse::Target, into: Local) -> Semantic<(Type, Ownership)> {
        let var = target.var();
        let var = match self.scope.lookup(var)? {
            Named::Var(var) => var,
            Named::Procs { .. } => {
                return Err(Located::at(
                    SemanticError::ExpectedVar(var.as_ref().clone()),
                    var.location().clone(),
                ))
            }
        };

        let var = var.clone();

        match &var.access {
            Access::Local(local) => self.sink.push(Instruction::Move(*local, into)),
            Access::Global(global) => self
                .sink
                .push(Instruction::LoadGlobal(global.clone(), into)),
        }

        if !target.indices().is_empty() {
            todo!()
        }

        Ok((var.typ, Ownership::Borrowed))
    }

    fn ephemeral<F, R>(&mut self, callback: F) -> Semantic<R>
    where
        F: FnOnce(&mut Self, Local) -> Semantic<(Type, Ownership, R)>,
        R: 'static,
    {
        let local = self.sink.alloc_local();

        let (typ, ownership, result) = callback(self, local)?;

        self.drop(local, typ, ownership);
        self.sink.free_local(local);

        Ok(result)
    }

    fn drop(&mut self, local: Local, typ: Type, ownership: Ownership) {
        let destructor = match (typ, ownership) {
            (_, Ownership::Borrowed) => None,
            (Type::Int, _) => None,
            (Type::Bool, _) => None,
            (Type::List, Ownership::Owned) => Some("builtin_drop_list"),
            (Type::Mat, Ownership::Owned) => Some("builtin_drop_mat"),
        };

        if let Some(destructor) = destructor {
            self.sink.push(Instruction::Call {
                target: Function::External(destructor),
                arguments: vec![local],
                output: None,
            });
        }
    }
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
