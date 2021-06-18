use thiserror::Error;

use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
    rc::Rc,
};

use crate::{
    ir::{self, Function, Global, Instruction, Label, Local},
    lex::{Identifier, NoCase},
    parse,
    source::{Located, Location},
};

#[derive(Default)]
struct SymbolTable<'a> {
    outer: Option<&'a SymbolTable<'a>>,
    symbols: HashMap<Identifier, Named>,
    lifted: HashSet<Identifier>,
}

impl SymbolTable<'_> {
    fn is_lifted(&self, id: &Identifier) -> bool {
        let mut table = self;

        loop {
            if table.lifted.contains(id) {
                break true;
            }

            match table.outer {
                None => break false,
                Some(outer) => table = outer,
            }
        }
    }

    fn lookup(&self, id: &Located<Identifier>) -> Semantic<&Named> {
        self.try_lookup(id).ok_or_else(|| {
            Located::at(
                SemanticError::Undefined(id.as_ref().clone()),
                id.location().clone(),
            )
        })
    }

    fn try_lookup(&self, id: &Located<Identifier>) -> Option<&Named> {
        let mut table = self;

        loop {
            match table.symbols.get(id) {
                Some(id) => break Some(id),

                None => match table.outer.as_ref() {
                    Some(outer) => table = outer,
                    None => break None,
                },
            }
        }
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
    Float,
}

impl Display for Type {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match self {
            Type::Int => "int",
            Type::Bool => "bool",
            Type::List => "list",
            Type::Mat => "mat",
            Type::Float => "float",
        };

        fmt.write_str(string)
    }
}

#[derive(Copy, Clone)]
enum Ownership {
    Owned,
    Borrowed,
}

#[derive(Copy, Clone)]
enum AssignmentMode {
    /// Asignación en main() que no inicializa una global
    Main,

    /// Asignación en main() que inicializa una global
    GlobalInit,

    /// Ninguna de las otras opciones es correcta
    Normal,
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
        assert!(
            local.0 < self.next_local.0
                && self
                    .free_locals
                    .iter()
                    .find(|&&other| other == local)
                    .is_none(),
            "bad free_local(): {:?}",
            local
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

    #[error("Type mismatch: expected `{0}` or `{1}`, found `{2}`")]
    ExpectedTwo(Type, Type, Type),

    #[error("Type mismatch: expected `{0}`, `{1}` or `{2}`, found `{3}`")]
    ExpectedThree(Type, Type, Type, Type),

    #[error("Expected variable, found procedure family `{0}`")]
    ExpectedVar(Identifier),

    #[error("Expected procedure, found variable `{0}`")]
    ExpectedProc(Identifier),

    #[error("Symbol `{0}` is undefined")]
    Undefined(Identifier),

    #[error("This definition for `{0}` is in conflict with a global variable")]
    NameClash(Identifier),

    #[error("Redefinition of procedure `{0}` with the same parameter types")]
    SignatureClash(Identifier),

    #[error("Parameter `{0}` is bound more than once")]
    RepeatedParameter(Identifier),

    #[error("Procedure family `{0}` exists, but the overload `{0}({1})` is not defined")]
    NoSuchOverload(Identifier, String),

    #[error("Invalid operands for `{0}`: `{1}` and `{2}`")]
    InvalidOperands(parse::BinOp, Type, Type),

    #[error("Expected {0} arguments, found {1}")]
    BadArgumentCount(usize, usize),

    #[error("Objects of type `{0}` have no attribute `{1}`")]
    NoSuchAttr(Type, Identifier),

    #[error("This global statement is in conflict with another symbol")]
    GlobalLiftConflict,
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
                        ..Default::default()
                    },

                    sink: Listing::for_parameters(parameters),
                    procedure: Some(procedure),
                    is_toplevel: Default::default(),
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
            .find(|proc| proc.is_entrypoint())
            .ok_or_else(|| Located::at(SemanticError::NoMain, self.eof().clone()))?;

        let mut context = Context {
            scope: SymbolTable {
                outer: None,
                ..Default::default()
            },

            sink: TypeCheck,
            procedure: None,
            is_toplevel: Default::default(),
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

impl parse::Procedure {
    fn is_entrypoint(&self) -> bool {
        let id = self.name().as_ref();
        unicase::eq_ascii(id.as_ref(), "main") && self.parameters().is_empty()
    }
}

struct Context<'a, S: Sink> {
    scope: SymbolTable<'a>,
    sink: S,
    procedure: Option<&'a parse::Procedure>,
    is_toplevel: bool,
}

impl<S: Sink> Context<'_, S> {
    fn scan_procedure(&mut self, procedure: &parse::Procedure) -> Semantic<Rc<String>> {
        let types = self.parameter_types(procedure)?;

        self.subscope(|this| {
            this.is_toplevel = true;

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
        let is_entrypoint = self
            .procedure
            .map(|proc| proc.is_entrypoint())
            .unwrap_or(false);

        let mut initialized_globals = HashSet::new();
        let mut assignment_mode = match (is_entrypoint, self.is_toplevel) {
            (true, true) => AssignmentMode::GlobalInit,
            (true, false) => AssignmentMode::Main,
            _ => AssignmentMode::Normal,
        };

        for statement in statements.iter() {
            use parse::{ObjectKind::*, Statement::*, TimeUnit::*};
            use AssignmentMode::*;

            assignment_mode = match (assignment_mode, statement) {
                (GlobalInit, Assignment { .. }) => GlobalInit,
                (GlobalInit, _) => Main,

                _ => assignment_mode,
            };

            match statement {
                If { condition, body } => self.scan_conditional(condition, body)?,

                For {
                    variable,
                    iterable,
                    step,
                    body,
                } => {
                    self.scan_loop(variable, iterable, step.as_ref(), body)?;
                }

                UserCall { procedure, args } => self.scan_user_call(procedure, args)?,
                Debug { location, hint } => self.scan_debug(location, hint.as_ref())?,

                Blink {
                    column,
                    row,
                    count,
                    unit,
                    state,
                } => {
                    let builtin = match unit {
                        Millis => "builtin_blink_mil",
                        Seconds => "builtin_blink_seg",
                        Minutes => "builtin_blink_min",
                    };

                    let args = [column, row, count, state];
                    let types = [Type::Int, Type::Int, Type::Int, Type::Bool];
                    let location = state.location();

                    self.eval_fixed_call(builtin, location, &args, &types, None)?;
                }

                Delay { count, unit } => {
                    let builtin = match unit {
                        Millis => "builtin_delay_mil",
                        Seconds => "builtin_delay_seg",
                        Minutes => "builtin_delay_min",
                    };

                    let types = [Type::Int];
                    let location = count.location();
                    self.eval_fixed_call(builtin, location, &[count], &types, None)?;
                }

                PrintLed { column, row, value } => {
                    let args = [column, row, value];
                    let types = [Type::Int, Type::Int, Type::Bool];
                    let location = value.location();
                    let builtin = "builtin_printled";

                    self.eval_fixed_call(builtin, location, &args, &types, None)?;
                }

                PrintLedX {
                    kind,
                    index,
                    object,
                } => {
                    let (builtin, object_type) = match kind {
                        Column => ("builtin_printledx_c", Type::List),
                        Row => ("builtin_printledx_f", Type::List),
                        Matrix => ("builtin_printledx_m", Type::Mat),
                    };

                    let args = [index, object];
                    let types = [Type::Int, object_type];
                    let location = object.location();

                    self.eval_fixed_call(builtin, location, &args, &types, None)?;
                }

                GlobalLift(id) => self.global_lift(id)?,

                Assignment { targets, values } => {
                    for (target, value) in break_assignment(targets, values)? {
                        let var = target.var().as_ref().as_ref();
                        let (global_init, mode) = match assignment_mode {
                            GlobalInit if initialized_globals.get(var).is_none() => {
                                (true, GlobalInit)
                            }

                            GlobalInit => (false, Main),
                            _ => (false, assignment_mode),
                        };

                        self.assign(mode, target, value)?;

                        if global_init {
                            initialized_globals.insert(var.clone());
                        }
                    }
                }

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

        self.subscope(|this| this.scan_statements(body))?;
        self.sink.push(Instruction::SetLabel(if_false));

        Ok(())
    }

    fn scan_loop(
        &mut self,
        variable: &Located<Identifier>,
        iterable: &Located<parse::Expr>,
        step: Option<&Located<parse::Expr>>,
        body: &[parse::Statement],
    ) -> Semantic<()> {
        let limit = self.sink.alloc_local();
        match self.type_check(iterable)? {
            Type::Int => drop(self.eval(iterable, limit)?),
            Type::List | Type::Mat => drop(self.eval_len(iterable, limit)?),

            bad => {
                return Err(Located::at(
                    SemanticError::ExpectedThree(Type::Int, Type::List, Type::Mat, bad),
                    iterable.location().clone(),
                ))
            }
        }

        let iterator = self.sink.alloc_local();
        self.sink.push(Instruction::LoadConst(0, iterator));

        let step = {
            let local = self.sink.alloc_local();
            if let Some(step) = step {
                self.eval_expecting(step, local, Type::Int)?;
            } else {
                self.sink.push(Instruction::LoadConst(1, local));
            }

            local
        };

        let condition_label = self.sink.next_label();
        let end_label = self.sink.next_label();

        self.sink.push(Instruction::SetLabel(condition_label));
        self.ephemeral(|this, is_less| {
            let op = ir::BinOp::Logic(ir::LogicOp::Less);
            this.sink.push(Instruction::Move(iterator, is_less));
            this.sink.push(Instruction::Binary(is_less, op, limit));
            this.sink.push(Instruction::JumpIfFalse(is_less, end_label));

            Ok((Type::Bool, Ownership::Owned, ()))
        })?;

        self.subscope(|this| {
            let named = Named::Var(Variable {
                access: Access::Local(iterator),
                typ: Type::Int,
            });

            this.scope.symbols.insert(variable.as_ref().clone(), named);
            this.scan_statements(body)
        })?;

        let op = ir::BinOp::Arithmetic(ir::ArithmeticOp::Add);
        self.sink.push(Instruction::Binary(iterator, op, step));
        self.sink.push(Instruction::Jump(condition_label));
        self.sink.push(Instruction::SetLabel(end_label));

        self.sink.free_local(step);
        self.sink.free_local(limit);
        // iterator es liberado por expire()

        Ok(())
    }

    fn scan_debug(
        &mut self,
        location: &Location,
        hint: Option<&Located<parse::Expr>>,
    ) -> Semantic<()> {
        let line = location.start().line() as i32;
        self.ephemeral(|this, line_local| {
            this.sink.push(Instruction::LoadConst(line, line_local));

            match hint {
                None => this.sink.push(Instruction::Call {
                    target: Function::External("builtin_debug"),
                    arguments: vec![line_local],
                    output: None,
                }),

                Some(hint) => this.ephemeral(|this, hint_local| {
                    let (typ, ownership) = this.eval(hint, hint_local)?;
                    let builtin = match typ {
                        Type::Bool => "builtin_debug_bool",
                        Type::Int => "builtin_debug_int",
                        Type::List => "builtin_debug_list",
                        Type::Mat => "builtin_debug_mat",
                        Type::Float => "builtin_debug_float",
                    };

                    this.sink.push(Instruction::Call {
                        target: Function::External(builtin),
                        arguments: vec![line_local, hint_local],
                        output: None,
                    });

                    Ok((typ, ownership, ()))
                })?,
            }

            Ok((Type::Int, Ownership::Owned, ()))
        })
    }

    fn scan_user_call(
        &mut self,
        target: &Located<Identifier>,
        args: &[Located<parse::Expr>],
    ) -> Semantic<()> {
        let mut types = Vec::new();
        let mut arg_locals = Vec::new();

        for arg in args.iter() {
            let local = self.sink.alloc_local();
            let typ = self.eval_owned(arg, local)?;

            arg_locals.push(local);
            types.push(typ);
        }

        let target = match self.scope.lookup(target)? {
            Named::Procs { variants } => variants.get(&types).ok_or_else(|| {
                let types = types.iter().map(ToString::to_string).collect::<Vec<_>>();
                let types = types.join(", ");

                Located::at(
                    SemanticError::NoSuchOverload(target.as_ref().clone(), types),
                    target.location().clone(),
                )
            })?,

            Named::Var(_) => {
                return Err(Located::at(
                    SemanticError::ExpectedProc(target.as_ref().clone()),
                    target.location().clone(),
                ))
            }
        };

        self.sink.push(Instruction::Call {
            target: Function::Generated(target.clone()),
            arguments: arg_locals.clone(),
            output: None,
        });

        for local in arg_locals.into_iter() {
            self.sink.free_local(local);
        }

        Ok(())
    }

    fn global_lift(&mut self, id: &Located<Identifier>) -> Semantic<()> {
        match self.scope.lookup(id)? {
            Named::Var(Variable {
                access: Access::Global(_),
                ..
            }) => {
                self.scope.lifted.insert(id.as_ref().clone());
                Ok(())
            }

            _ => Err(Located::at(
                SemanticError::GlobalLiftConflict,
                id.location().clone(),
            )),
        }
    }

    fn assign(
        &mut self,
        mode: AssignmentMode,
        target: &Located<parse::Target>,
        value: &Located<parse::Expr>,
    ) -> Semantic<()> {
        use AssignmentMode::*;

        if !target.indices().is_empty() {
            todo!();
        }

        let value_type = self.type_check(value)?;
        let target = target.var();

        let should_override = |var: &Variable, scope: &SymbolTable<'_>| {
            matches!(&var.access, Access::Global(_)) && !scope.is_lifted(target.as_ref())
        };

        let var = match (mode, self.scope.try_lookup(target)) {
            (
                GlobalInit,
                Some(Named::Var(Variable {
                    access: Access::Global(global),
                    ..
                })),
            ) => {
                let global = global.clone();

                return self.ephemeral(|this, local| {
                    this.eval_owned(value, local)?;
                    this.sink.push(Instruction::StoreGlobal(local, global));

                    // Esto evita un drop de la local
                    Ok((Type::Int, Ownership::Owned, ()))
                });
            }

            (Main, Some(Named::Var(var))) => var,
            (_, Some(Named::Var(var))) if !should_override(var, &self.scope) => var,

            _ => {
                let local = self.sink.alloc_local();
                self.eval_owned(value, local)?;

                let named = Named::Var(Variable {
                    access: Access::Local(local),
                    typ: value_type,
                });

                self.scope.symbols.insert(target.as_ref().clone(), named);
                return Ok(());
            }
        };

        if var.typ != value_type {
            return Err(Located::at(
                SemanticError::ExpectedType(var.typ, value_type),
                value.location().clone(),
            ));
        }

        let must_drop = destructor(var.typ, Ownership::Owned).is_some();
        match (&var.access, must_drop) {
            (Access::Local(local), false) => {
                let local = *local;
                self.eval(value, local)?;
            }

            (Access::Local(local), true) => {
                let local = *local;

                self.ephemeral(|this, value_local| {
                    this.eval(value, value_local)?;
                    this.drop(local, value_type, Ownership::Owned);
                    this.sink.push(Instruction::Move(value_local, local));

                    // Se evita otro drop
                    Ok((Type::Int, Ownership::Owned, ()))
                })?;
            }

            (Access::Global(global), drop) => {
                let global = global.clone();
                self.ephemeral(|this, value_local| {
                    this.eval(value, value_local)?;
                    if drop {
                        let global = global.clone();

                        this.ephemeral(|this, old_local| {
                            this.sink.push(Instruction::LoadGlobal(global, old_local));
                            Ok((value_type, Ownership::Owned, ()))
                        })?;
                    }

                    this.sink
                        .push(Instruction::StoreGlobal(value_local, global));
                    Ok((Type::Int, Ownership::Owned, ()))
                })?;
            }
        }

        Ok(())
    }

    fn parameter_types(&mut self, procedure: &parse::Procedure) -> Semantic<Vec<Type>> {
        procedure
            .parameters()
            .iter()
            .map(|param| match param.of().as_ref() {
                parse::Type::Int => Ok(Type::Int),
                parse::Type::Bool => Ok(Type::Bool),
                parse::Type::List => Ok(Type::List),
                parse::Type::Mat => Ok(Type::Mat),
                parse::Type::Float => Ok(Type::Float),
                parse::Type::Of(expr) => self.type_check(expr),
            })
            .collect()
    }

    fn type_check(&mut self, expr: &Located<parse::Expr>) -> Semantic<Type> {
        let mut context = Context {
            scope: SymbolTable {
                outer: Some(&self.scope),
                ..Default::default()
            },

            sink: TypeCheck,
            procedure: None,
            is_toplevel: Default::default(),
        };

        let (typ, _) = context.eval(expr, Local::default())?;
        Ok(typ)
    }

    fn eval_fixed_call(
        &mut self,
        builtin: &'static str,
        at: &Location,
        args: &[&Located<parse::Expr>],
        types: &[Type],
        output: Option<Local>,
    ) -> Semantic<()> {
        if args.len() != types.len() {
            return Err(Located::at(
                SemanticError::BadArgumentCount(types.len(), args.len()),
                at.clone(),
            ));
        }

        let mut arg_locals = Vec::new();
        let mut ownerships = Vec::new();

        for (arg, typ) in args.iter().copied().zip(types.iter()) {
            let local = self.sink.alloc_local();
            let ownership = self.eval_expecting(arg, local, *typ)?;

            arg_locals.push(local);
            ownerships.push(ownership);
        }

        self.sink.push(Instruction::Call {
            target: Function::External(builtin),
            arguments: arg_locals.clone(),
            output,
        });

        let dropped = arg_locals
            .into_iter()
            .zip(ownerships.into_iter())
            .zip(types.iter().cloned());

        for ((local, ownership), typ) in dropped {
            self.drop(local, typ, ownership);
            self.sink.free_local(local);
        }

        Ok(())
    }

    fn eval_owned(&mut self, expr: &Located<parse::Expr>, into: Local) -> Semantic<Type> {
        use Ownership::{Borrowed, Owned};

        let (typ, ownership) = self.eval(expr, into)?;
        let cloner = match (typ, ownership) {
            (_, Owned) => None,
            (Type::Int | Type::Bool | Type::Float, _) => None,
            (Type::List, Borrowed) => Some("builtin_ref_list"),
            (Type::Mat, Borrowed) => Some("builtin_ref_mat"),
        };

        if let Some(cloner) = cloner {
            self.sink.push(Instruction::Call {
                target: Function::External(cloner),
                arguments: vec![into],
                output: Some(into),
            });
        }

        Ok(typ)
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
        use Ownership::{Borrowed, Owned};

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

            Read(id) => {
                let typ = self.read(id, into)?;
                Ok((typ, Borrowed))
            }

            Attr(base, attr) => {
                let typ = self.read_attr(base, attr, into)?;
                Ok((typ, Owned))
            }

            Index(base, index) => {
                let typ = self.read_index(base, index, into)?;
                Ok((typ, Owned))
            }

            Len(expr) => {
                self.eval_len(expr, into)?;
                Ok((Type::Int, Owned))
            }

            Range(length, value) => {
                let builtin = "builtin_range";
                let args = [&**length, &**value];
                let types = [Type::Int, Type::Bool];
                let location = value.location();

                self.eval_fixed_call(builtin, location, &args, &types, Some(into))?;
                Ok((Type::List, Owned))
            }

            List(items) => {
                let typ = self.eval_sequence(&items, into)?;
                Ok((typ, Owned))
            }

            Negate(expr) => {
                self.eval_expecting(expr, into, Type::Int)?;
                self.sink.push(Instruction::Negate(into));

                Ok((Type::Int, Owned))
            }

            Binary { lhs, op, rhs, .. } => {
                let typ = self.eval_binary(expr.location(), lhs, *op, rhs, into)?;
                Ok((typ, Owned))
            }
        }
    }

    fn eval_binary(
        &mut self,
        at: &Location,
        lhs: &Located<parse::Expr>,
        op: parse::BinOp,
        rhs: &Located<parse::Expr>,
        into: Local,
    ) -> Semantic<Type> {
        self.ephemeral(|this, rhs_local| {
            let (typ, lhs_ownership) = this.eval(lhs, into)?;
            let rhs_ownership = match this.eval(rhs, rhs_local)? {
                (rhs_typ, _) if rhs_typ != typ => {
                    return Err(Located::at(
                        SemanticError::InvalidOperands(op, typ, rhs_typ),
                        at.clone(),
                    ))
                }

                (_, rhs_ownership) => rhs_ownership,
            };

            use ir::{ArithmeticOp, BinOp as IrOp, LogicOp};
            use parse::BinOp as ParseOp;
            use Type::*;

            let op = match (op, typ) {
                (_, Float) => {
                    let typ = this.do_float_binary(at, into, op, rhs_local)?;
                    return Ok((Type::Float, Ownership::Owned, typ));
                }

                (ParseOp::Add, Int) => IrOp::Arithmetic(ArithmeticOp::Add),
                (ParseOp::Sub, Int) => IrOp::Arithmetic(ArithmeticOp::Sub),
                (ParseOp::Mul, Int) => IrOp::Arithmetic(ArithmeticOp::Mul),
                (ParseOp::Mod, Int) => IrOp::Arithmetic(ArithmeticOp::Mod),
                (ParseOp::IntegerDiv, Int) => IrOp::Arithmetic(ArithmeticOp::Div),

                (ParseOp::Div, Int) => {
                    this.do_builtin_assign(into, "builtin_div_int", rhs_local);
                    return Ok((Type::Int, Ownership::Owned, Type::Float));
                }

                (ParseOp::Pow, Int) => {
                    this.do_builtin_assign(into, "builtin_pow_int", rhs_local);
                    return Ok((Type::Int, Ownership::Owned, Type::Float));
                }

                (ParseOp::Equal, Int | Bool) => IrOp::Logic(LogicOp::Equal),
                (ParseOp::NotEqual, Int | Bool) => IrOp::Logic(LogicOp::NotEqual),
                (ParseOp::Greater, Int | Bool) => IrOp::Logic(LogicOp::Greater),
                (ParseOp::GreaterOrEqual, Int | Bool) => IrOp::Logic(LogicOp::GreaterOrEqual),
                (ParseOp::Less, Int | Bool) => IrOp::Logic(LogicOp::Less),
                (ParseOp::LessOrEqual, Int | Bool) => IrOp::Logic(LogicOp::LessOrEqual),

                (ParseOp::Equal | ParseOp::NotEqual, List | Mat) => {
                    let comparator = if typ == List {
                        "builtin_eq_list"
                    } else {
                        "builtin_eq_mat"
                    };

                    this.ephemeral(|this, lhs_local| {
                        this.sink.push(Instruction::Move(into, lhs_local));
                        this.sink.push(Instruction::Call {
                            target: Function::External(comparator),
                            arguments: vec![lhs_local, rhs_local],
                            output: Some(into),
                        });

                        Ok((typ, lhs_ownership, ()))
                    })?;

                    if op == ParseOp::NotEqual {
                        this.sink.push(Instruction::Not(into));
                    }

                    return Ok((typ, rhs_ownership, Bool));
                }

                _ => {
                    return Err(Located::at(
                        SemanticError::InvalidOperands(op, typ, typ),
                        at.clone(),
                    ))
                }
            };

            let result_type = match op {
                IrOp::Arithmetic(_) => Int,
                IrOp::Logic(_) => Bool,
            };

            this.sink.push(Instruction::Binary(into, op, rhs_local));
            Ok((typ, rhs_ownership, result_type))
        })
    }

    fn do_float_binary(
        &mut self,
        location: &Location,
        lhs: Local,
        op: parse::BinOp,
        rhs: Local,
    ) -> Semantic<Type> {
        enum FloatEval {
            Arithmetic(&'static str),
            Logic(ir::LogicOp),
        }

        use parse::BinOp::*;
        use FloatEval::*;

        let float_eval = match op {
            Add => Arithmetic("builtin_add_float"),
            Sub => Arithmetic("builtin_sub_float"),
            Mul => Arithmetic("builtin_mul_float"),
            Div => Arithmetic("builtin_div_float"),
            Pow => Arithmetic("builtin_pow_float"),
            Equal => Logic(ir::LogicOp::Equal),
            NotEqual => Logic(ir::LogicOp::NotEqual),
            Less => Logic(ir::LogicOp::Less),
            LessOrEqual => Logic(ir::LogicOp::LessOrEqual),
            Greater => Logic(ir::LogicOp::Greater),
            GreaterOrEqual => Logic(ir::LogicOp::GreaterOrEqual),

            _ => {
                return Err(Located::at(
                    SemanticError::InvalidOperands(op, Type::Float, Type::Float),
                    location.clone(),
                ))
            }
        };

        match float_eval {
            Arithmetic(builtin) => {
                self.do_builtin_assign(lhs, builtin, rhs);
                Ok(Type::Float)
            }

            Logic(op) => self.ephemeral(|this, zero| {
                this.sink.push(Instruction::Call {
                    target: Function::External("builtin_cmp_float"),
                    arguments: vec![lhs, rhs],
                    output: Some(lhs),
                });

                this.sink.push(Instruction::LoadConst(0, zero));
                this.sink
                    .push(Instruction::Binary(lhs, ir::BinOp::Logic(op), zero));

                Ok((Type::Int, Ownership::Owned, Type::Bool))
            }),
        }
    }

    fn do_builtin_assign(&mut self, lhs: Local, builtin: &'static str, rhs: Local) {
        self.sink.push(Instruction::Call {
            target: Function::External(builtin),
            arguments: vec![lhs, rhs],
            output: Some(lhs),
        });
    }

    fn eval_len(&mut self, expr: &Located<parse::Expr>, into: Local) -> Semantic<()> {
        self.ephemeral(|this, arg| {
            let (arg_type, arg_ownership) = this.eval(expr, arg)?;
            let target = match arg_type {
                Type::List => Function::External("builtin_len_list"),
                Type::Mat => Function::External("builtin_shapef"),

                _ => {
                    return Err(Located::at(
                        SemanticError::ExpectedTwo(Type::List, Type::Mat, arg_type),
                        expr.location().clone(),
                    ))
                }
            };

            this.sink.push(Instruction::Call {
                target,
                arguments: vec![arg],
                output: Some(into),
            });

            Ok((arg_type, arg_ownership, ()))
        })
    }

    fn eval_sequence(&mut self, items: &[Located<parse::Expr>], into: Local) -> Semantic<Type> {
        let item = self.sink.alloc_local();

        let is_mat = match items.first() {
            None => false,

            Some(first) => match self.type_check(first)? {
                Type::Bool => false,
                Type::List => true,

                bad => {
                    return Err(Located::at(
                        SemanticError::ExpectedTwo(Type::Bool, Type::List, bad),
                        first.location().clone(),
                    ))
                }
            },
        };

        let constructor = if is_mat {
            "builtin_new_mat"
        } else {
            "builtin_new_list"
        };

        self.sink.push(Instruction::Call {
            target: Function::External(constructor),
            arguments: Vec::new(),
            output: Some(into),
        });

        let zero = if is_mat {
            let zero = self.sink.alloc_local();
            self.sink.push(Instruction::LoadConst(0, zero));

            Some(zero)
        } else {
            None
        };

        let index = self.sink.alloc_local();
        for (i, expr) in items.iter().enumerate() {
            self.sink.push(Instruction::LoadConst(i as i32, index));

            let (insert, expected, arguments) = if is_mat {
                let arguments = vec![into, item, zero.unwrap(), index];
                ("builtin_insert_mat", Type::List, arguments)
            } else {
                ("builtin_insert_list", Type::Bool, vec![into, index, item])
            };

            let ownership = self.eval_expecting(expr, item, expected)?;
            self.sink.push(Instruction::Call {
                target: Function::External(insert),
                arguments,
                output: None,
            });

            self.drop(item, expected, ownership);
        }

        if let Some(zero) = zero {
            self.sink.free_local(zero);
        }

        self.sink.free_local(index);
        self.sink.free_local(item);

        if is_mat {
            Ok(Type::Mat)
        } else {
            Ok(Type::List)
        }
    }

    fn read_attr(
        &mut self,
        base: &Located<parse::Expr>,
        attr: &Located<Identifier>,
        into: Local,
    ) -> Semantic<Type> {
        const ATTRS: &'static [((Type, NoCase<&'static str>), (&'static str, Type))] = &[
            (
                (Type::Mat, NoCase::new("shapeF")),
                ("builtin_shapef", Type::Int),
            ),
            (
                (Type::Mat, NoCase::new("shapeC")),
                ("builtin_shapec", Type::Int),
            ),
        ];

        let typ = self.type_check(base)?;

        let attr_name = NoCase::new(attr.as_ref().as_ref());
        let (getter, result) = ATTRS
            .iter()
            .find(|(key, _)| *key == (typ, attr_name))
            .map(|(_, value)| value)
            .copied()
            .ok_or_else(|| {
                Located::at(
                    SemanticError::NoSuchAttr(typ, attr.as_ref().clone()),
                    attr.location().clone(),
                )
            })?;

        self.eval_fixed_call(getter, attr.location(), &[base], &[typ], Some(into))?;
        Ok(result)
    }

    fn read(&mut self, target: &Located<Identifier>, into: Local) -> Semantic<Type> {
        let var = match self.scope.lookup(target)? {
            Named::Var(var) => var,
            Named::Procs { .. } => {
                return Err(Located::at(
                    SemanticError::ExpectedVar(target.as_ref().clone()),
                    target.location().clone(),
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

        Ok(var.typ)
    }

    fn read_index(
        &mut self,
        base: &Located<parse::Expr>,
        index: &Located<parse::Index>,
        into: Local,
    ) -> Semantic<Type> {
        use parse::Index;

        let base_type = self.type_check(base)?;

        let expect_mat = || {
            (base_type == Type::Mat).then(|| ()).ok_or_else(|| {
                Located::at(
                    SemanticError::ExpectedType(Type::Mat, base_type),
                    base.location().clone(),
                )
            })
        };

        let expect_list_or_mat = || {
            (base_type == Type::List || base_type == Type::Mat)
                .then(|| ())
                .ok_or_else(|| {
                    Located::at(
                        SemanticError::ExpectedTwo(Type::List, Type::Mat, base_type),
                        base.location().clone(),
                    )
                })
        };

        let (builtin, typ, args) = match index.as_ref() {
            Index::Single(expr) => {
                expect_list_or_mat()?;
                let (builtin, typ) = if base_type == Type::List {
                    ("builtin_index_list", Type::Bool)
                } else {
                    ("builtin_index_row_mat", Type::List)
                };

                (builtin, typ, vec![base, expr])
            }

            Index::Range(from, to) => {
                expect_list_or_mat()?;
                let builtin = if base_type == Type::List {
                    "builtin_slice_list"
                } else {
                    "builtin_slice_mat"
                };

                (builtin, base_type, vec![base, from, to])
            }

            Index::Indirect(row, column) => {
                expect_mat()?;
                (
                    "builtin_index_entry_mat",
                    Type::Bool,
                    vec![base, row, column],
                )
            }

            Index::Transposed(column) => {
                expect_mat()?;
                ("builtin_index_column_mat", Type::List, vec![base, column])
            }
        };

        let types = [base_type, Type::Int, Type::Int];
        let types = &types[..args.len()];

        let location = index.location();
        self.eval_fixed_call(builtin, location, &args, types, Some(into))?;

        Ok(typ)
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
                ..Default::default()
            },

            sink,
            procedure: self.procedure,
            is_toplevel: false,
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
        if let Some(destructor) = destructor(typ, ownership) {
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

fn destructor(typ: Type, ownership: Ownership) -> Option<&'static str> {
    match (typ, ownership) {
        (_, Ownership::Borrowed) => None,
        (Type::Int | Type::Bool | Type::Float, _) => None,
        (Type::List, Ownership::Owned) => Some("builtin_drop_list"),
        (Type::Mat, Ownership::Owned) => Some("builtin_drop_mat"),
    }
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
            Type::Float => 'f',
        }));
    }

    mangled
}
