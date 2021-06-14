use thiserror::Error;

use std::{
    collections::HashMap,
    fmt::{self, Display},
    rc::Rc,
};

use crate::{
    ir::{self, Function, Global, Instruction, Label, Local},
    lex::Identifier,
    parse,
    source::Located,
};

struct SymbolTable<'a> {
    outer: Option<&'a SymbolTable<'a>>,
    symbols: HashMap<Identifier, Named>,
}

impl SymbolTable<'_> {
    fn lookup(&self, id: &Located<Identifier>) -> Semantic<&Named> {
        let mut table = self;
        let named = loop {
            match table.symbols.get(id) {
                Some(id) => break Some(id),

                None => match table.outer.as_ref() {
                    Some(outer) => table = outer,
                    None => break None,
                },
            }
        };

        named.ok_or_else(|| {
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
        let string = match self {
            Type::Int => "int",
            Type::Bool => "bool",
            Type::List => "list",
            Type::Mat => "mat",
        };

        fmt.write_str(string)
    }
}

#[derive(Copy, Clone)]
enum Ownership {
    Owned,
    Borrowed,
}

trait Sink: Default {
    fn push(&mut self, instruction: Instruction);

    fn alloc_local(&mut self) -> Local;

    fn free_local(&mut self, local: Local);

    fn next_label(&mut self) -> Label;
}

#[derive(Copy, Clone, Default)]
struct TypeCheck;

impl Sink for TypeCheck {
    fn push(&mut self, _instruction: Instruction) {}

    fn alloc_local(&mut self) -> Local {
        Local::default()
    }

    fn free_local(&mut self, _local: Local) {}

    fn next_label(&mut self) -> Label {
        Label::default()
    }
}

#[derive(Default)]
struct Listing {
    body: Vec<Instruction>,
    free_locals: Vec<Local>,
    next_local: Local,
    next_label: Label,
}

impl Listing {
    fn for_parameters(parameters: u32) -> Self {
        Listing {
            body: Vec::new(),
            free_locals: Vec::new(),
            next_local: Local(parameters),
            next_label: Label::default(),
        }
    }
}

impl Sink for Listing {
    fn push(&mut self, instruction: Instruction) {
        self.body.push(instruction);
    }

    fn alloc_local(&mut self) -> Local {
        if let Some(local) = self.free_locals.pop() {
            local
        } else {
            let Local(next_local) = self.next_local;
            self.next_local = Local(next_local + 1);

            Local(next_local)
        }
    }

    fn free_local(&mut self, local: Local) {
        debug_assert!(
            local.0 < self.next_local.0
                && self
                    .free_locals
                    .iter()
                    .find(|&&other| other == local)
                    .is_none()
        );

        self.free_locals.push(local);
    }

    fn next_label(&mut self) -> Label {
        let label = self.next_label;
        self.next_label = Label(label.0 + 1);

        label
    }
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

    #[error("Parameter `{0}` is bound more than once")]
    RepeatedParameter(Identifier),
}

impl parse::Ast {
    pub fn resolve(self) -> Semantic<ir::Program> {
        let global_scope = self.scan_global_scope()?;

        let code = self
            .iter()
            .map(|procedure| {
                let parameters = procedure.parameters().len() as u32;
                let mut context = Context {
                    scope: SymbolTable {
                        outer: Some(&global_scope),
                        symbols: Default::default(),
                    },

                    sink: Listing::for_parameters(parameters),
                };

                let symbol = context.scan_procedure(procedure)?;
                Ok(ir::GeneratedFunction {
                    name: symbol,
                    body: context.sink.body,
                    parameters,
                })
            })
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

    fn scan_global_scope(&self) -> Semantic<SymbolTable<'_>> {
        let main = self
            .iter()
            .find(|proc| {
                let id = proc.name().as_ref();
                unicase::eq_ascii(id.as_ref(), "main") && proc.parameters().is_empty()
            })
            .ok_or_else(|| Located::at(SemanticError::NoMain, self.eof().clone()))?;

        let mut context = Context {
            scope: SymbolTable {
                outer: None,
                symbols: Default::default(),
            },

            sink: TypeCheck,
        };

        let mut statements = main.statements().iter();
        while let Some(parse::Statement::Assignment { targets, values }) = statements.next() {
            for (target, value) in break_assignment(targets, values)? {
                // Inicialmente solo se consideran definiciones y no asignaciones
                let id = target.var().as_ref();
                if context.scope.symbols.get(id).is_none() && target.indices().is_empty() {
                    // Esto solo verifica e infiere tipos, todavía no se genera IR
                    let (typ, _) = context.eval(value, Local::default())?;

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

        let globals = context.scope;
        Ok(globals)
    }
}

struct Context<'a, S: Sink> {
    scope: SymbolTable<'a>,
    sink: S,
}

impl<S: Sink> Context<'_, S> {
    fn scan_procedure(&mut self, procedure: &parse::Procedure) -> Semantic<Rc<String>> {
        let types = self.parameter_types(procedure)?;

        self.subscope(|this| {
            let parameters = procedure.parameters().iter();
            for (i, (parameter, typ)) in parameters.zip(types.iter().copied()).enumerate() {
                let name = parameter.name();
                let var = Named::Var(Variable {
                    access: Access::Local(Local(i as u32)),
                    typ,
                });

                let id = name.as_ref().clone();
                if this.scope.symbols.insert(id, var).is_some() {
                    return Err(Located::at(
                        SemanticError::RepeatedParameter(name.as_ref().clone()),
                        name.location().clone(),
                    ));
                }
            }

            this.scan_statements(procedure.statements())
        })?;

        match self.scope.lookup(procedure.name()) {
            Ok(Named::Procs { variants }) => Ok(variants.get(&types).unwrap().clone()),
            _ => unreachable!(),
        }
    }

    fn scan_statements(&mut self, statements: &[parse::Statement]) -> Semantic<()> {
        for statement in statements.iter() {
            use parse::Statement::*;

            match statement {
                If { condition, body } => self.scan_conditional(condition, body)?,

                _ => todo!(),
            }
        }

        Ok(())
    }

    fn scan_conditional(
        &mut self,
        condition: &Located<parse::Expr>,
        body: &[parse::Statement],
    ) -> Semantic<()> {
        let if_false = self.sink.next_label();

        self.ephemeral(|this, local| {
            this.eval_expecting(condition, local, Type::Bool)?;
            this.sink.push(Instruction::JumpIfFalse(local, if_false));

            Ok((Type::Bool, Ownership::Owned, ()))
        })?;

        self.scan_statements(body)?;
        self.sink.push(Instruction::SetLabel(if_false));

        Ok(())
    }

    fn parameter_types(&mut self, procedure: &parse::Procedure) -> Semantic<Vec<Type>> {
        let mut type_check = Context {
            scope: SymbolTable {
                outer: Some(&mut self.scope),
                symbols: Default::default(),
            },

            sink: TypeCheck,
        };

        procedure
            .parameters()
            .iter()
            .map(|param| match param.of().as_ref() {
                parse::Type::Int => Ok(Type::Int),
                parse::Type::Bool => Ok(Type::Bool),
                parse::Type::List => Ok(Type::List),
                parse::Type::Mat => Ok(Type::Mat),
                parse::Type::Of(expr) => {
                    let (typ, _) = type_check.eval(expr, Local::default())?;
                    Ok(typ)
                }
            })
            .collect()
    }

    fn eval_expecting(
        &mut self,
        expr: &Located<parse::Expr>,
        into: Local,
        typ: Type,
    ) -> Semantic<Ownership> {
        let (actual, ownership) = self.eval(expr, into)?;
        if actual == typ {
            Ok(ownership)
        } else {
            Err(Located::at(
                SemanticError::ExpectedType(typ, actual),
                expr.location().clone(),
            ))
        }
    }

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
                    Type::List => Function::External("builtin_len_list"),
                    Type::Mat => Function::External("builtin_len_mat"),

                    _ => {
                        return Err(Located::at(
                            SemanticError::ExpectedListOrMat(arg_type),
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

    fn expire(mut self) -> S {
        std::mem::take(&mut self.scope.symbols)
            .into_iter()
            .for_each(|(_, named)| match named {
                Named::Var(Variable {
                    access: Access::Local(local),
                    typ,
                }) => {
                    self.drop(local, typ, Ownership::Owned);
                    self.sink.free_local(local);
                }

                _ => (),
            });

        self.sink
    }

    fn subscope<F, R>(&mut self, callback: F) -> R
    where
        F: FnOnce(&mut Context<'_, S>) -> R,
    {
        let sink = std::mem::take(&mut self.sink);
        let mut subcontext = Context {
            scope: SymbolTable {
                outer: Some(&self.scope),
                symbols: Default::default(),
            },

            sink,
        };

        let result = callback(&mut subcontext);
        self.sink = subcontext.expire();

        result
    }

    fn ephemeral<F, R>(&mut self, callback: F) -> Semantic<R>
    where
        F: FnOnce(&mut Self, Local) -> Semantic<(Type, Ownership, R)>,
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
