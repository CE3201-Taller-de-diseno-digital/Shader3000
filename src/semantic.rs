use thiserror::Error;

use std::{
    borrow::Borrow,
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
    statics: HashMap<Identifier, Static>,
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

    fn lookup_static(&self, id: &Identifier) -> Option<Static> {
        self.statics.get(id).copied()
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

#[derive(Copy, Clone, Debug)]
pub enum Addressed {
    List,
    Mat,
    Pod(Type),
    ListEntry(Local),
    MatEntry(Local, Local),
    MatRow(Local),
    MatColumn(Local),
    ListSlice(Local, Local),
    MatSlice(Local, Local),
}

impl Display for Addressed {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Addressed::List => write!(fmt, "type `{}`", Type::List),
            Addressed::Mat => write!(fmt, "type `{}`", Type::Mat),
            Addressed::Pod(typ) => write!(fmt, "type `{}`", typ),
            Addressed::ListEntry(_) => fmt.write_str("list entry"),
            Addressed::MatEntry(_, _) => fmt.write_str("matrix entry"),
            Addressed::MatRow(_) => fmt.write_str("matrix row"),
            Addressed::MatColumn(_) => fmt.write_str("matrix column"),
            Addressed::ListSlice(_, _) => fmt.write_str("list slice"),
            Addressed::MatSlice(_, _) => fmt.write_str("matrix slice"),
        }
    }
}

#[derive(Copy, Clone)]
enum Static {
    Int(i32),
    Bool(bool),
    Float(f32),
    List { length: i32 },
    Mat { rows: i32, columns: i32 },
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

    #[error("Definition for `{0}` is in conflict with a global variable")]
    NameClash(Identifier),

    #[error("Redefinition of procedure `{0}` with the same parameter types")]
    SignatureClash(Identifier),

    #[error("Parameter `{0}` is bound more than once")]
    RepeatedParameter(Identifier),

    #[error("Procedure family `{0}` exists, but the overload `{0}({1})` is undefined")]
    NoSuchOverload(Identifier, String),

    #[error("Invalid operands for `{0}`: `{1}` and `{2}`")]
    InvalidOperands(parse::BinOp, Type, Type),

    #[error("Expected {0} arguments, found {1}")]
    BadArgumentCount(usize, usize),

    #[error("Objects of type `{0}` have no attribute `{1}`")]
    NoSuchAttr(Type, Identifier),

    #[error("This global statement is in conflict with another symbol")]
    GlobalLiftConflict,

    #[error("Cannot cast `{0}` to `{1}`")]
    BadCast(Type, Type),

    #[error("Invalid addressing mode for base type `{0}`")]
    InvalidAddressing(Type),

    #[error("Method `{0}` is undefined for {1} instances")]
    NoSuchMethod(Identifier, Addressed),

    #[error("Denominator expression is always zero")]
    DivisionByZero,

    #[error("This parameter must be `0` or `1`, but was proven to always be `{0}`")]
    ExpectedMatMode(i32),

    #[error("Expected `{0}` columns, found `{1}`")]
    ExpectedColumns(usize, usize),

    #[error("Index always evaluates to `{0}`, outside of bounds `[0, {1}{2}`")]
    OutOfBounds(i32, i32, char),
}

impl parse::Ast {
    pub fn resolve(self) -> Semantic<ir::Program> {
        let mut global_scope = self.scan_global_scope()?;
        let mut global_statics = Some(std::mem::take(&mut global_scope.statics));

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

                let is_main = procedure.is_entrypoint();
                if is_main {
                    context.scope.statics = global_statics.take().unwrap_or_default();
                }

                let (mut sink, symbol) = context.scan_procedure(procedure)?;
                if is_main {
                    drop_globals(&mut sink, &global_scope);
                }

                Ok(ir::GeneratedFunction {
                    name: symbol,
                    body: sink.body,
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
                    if let Some(static_init) = context.const_eval(value) {
                        context.scope.statics.insert(id.clone(), static_init);
                    }
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
    fn scan_procedure(mut self, procedure: &parse::Procedure) -> Semantic<(S, Rc<String>)> {
        let types = self.parameter_types(procedure)?;

        self.is_toplevel = true;

        let parameters = procedure.parameters().iter();
        for (i, (parameter, typ)) in parameters.zip(types.iter().copied()).enumerate() {
            let name = parameter.name();
            let var = Named::Var(Variable {
                access: Access::Local(Local(i as u32)),
                typ,
            });

            let id = name.as_ref().clone();
            if self.scope.symbols.insert(id, var).is_some() {
                return Err(Located::at(
                    SemanticError::RepeatedParameter(name.as_ref().clone()),
                    name.location().clone(),
                ));
            }
        }

        let symbol = match self.scope.lookup(procedure.name()) {
            Ok(Named::Procs { variants }) => variants.get(&types).unwrap().clone(),
            _ => unreachable!(),
        };

        self.scan_statements(procedure.statements())?;
        Ok((self.expire(), symbol))
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

                MethodCall {
                    target,
                    method,
                    args,
                } => {
                    self.scan_method_call(target, method, args)?;
                }
            }
        }

        Ok(())
    }

    fn scan_conditional(
        &mut self,
        condition: &Located<parse::Expr>,
        body: &[parse::Statement],
    ) -> Semantic<()> {
        match condition.as_ref() {
            parse::Expr::Binary {
                lhs,
                op: op @ (parse::BinOp::Equal | parse::BinOp::NotEqual),
                rhs,
                ..
            } if (self.type_check(lhs)?, self.type_check(rhs)?) == (Type::List, Type::Bool) => {
                return self.scan_iterated_conditional(lhs, *op, rhs, body);
            }

            _ => {
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
        }
    }

    fn scan_iterated_conditional(
        &mut self,
        lhs: &Located<parse::Expr>,
        op: parse::BinOp,
        rhs: &Located<parse::Expr>,
        body: &[parse::Statement],
    ) -> Semantic<()> {
        let limit = self.sink.alloc_local();
        let iterator = self.sink.alloc_local();
        let iterable = self.sink.alloc_local();

        let (_, ownership) = self.eval(lhs, iterable)?;
        self.sink.push(Instruction::LoadConst(0, iterator));
        self.sink.push(Instruction::Call {
            target: Function::External("builtin_len_list"),
            arguments: vec![iterable],
            output: Some(limit),
        });

        let condition_label = self.sink.next_label();
        let end_label = self.sink.next_label();
        let false_label = self.sink.next_label();

        self.sink.push(Instruction::SetLabel(condition_label));
        self.ephemeral(|this, test_local| {
            let less = ir::BinOp::Logic(ir::LogicOp::Less);

            this.sink.push(Instruction::Move(iterator, test_local));
            this.sink.push(Instruction::Binary(test_local, less, limit));
            this.sink
                .push(Instruction::JumpIfFalse(test_local, end_label));

            Ok((Type::Bool, Ownership::Owned, ()))
        })?;

        self.ephemeral(|this, rhs_local| {
            this.ephemeral(|this, entry_local| {
                this.sink.push(Instruction::Call {
                    target: Function::External("builtin_index_list"),
                    arguments: vec![iterable, iterator],
                    output: Some(entry_local),
                });

                let op = match op {
                    parse::BinOp::Equal => ir::LogicOp::Equal,
                    parse::BinOp::NotEqual => ir::LogicOp::NotEqual,
                    _ => unreachable!(),
                };

                let op = ir::BinOp::Logic(op);

                this.eval(rhs, rhs_local)?;
                this.sink
                    .push(Instruction::Binary(rhs_local, op, entry_local));
                this.sink
                    .push(Instruction::JumpIfFalse(rhs_local, false_label));

                Ok((
                    Type::Bool,
                    Ownership::Owned,
                    (Type::Bool, Ownership::Owned, ()),
                ))
            })
        })?;

        self.subscope(|this| this.scan_statements(body))?;
        self.sink.push(Instruction::SetLabel(false_label));

        self.ephemeral(|this, one| {
            let add = ir::BinOp::Arithmetic(ir::ArithmeticOp::Add);

            this.sink.push(Instruction::LoadConst(1, one));
            this.sink.push(Instruction::Binary(iterator, add, one));
            this.sink.push(Instruction::Jump(condition_label));

            Ok((Type::Int, Ownership::Owned, ()))
        })?;

        self.sink.push(Instruction::SetLabel(end_label));
        self.drop(iterable, Type::List, ownership);

        self.sink.free_local(limit);
        self.sink.free_local(iterator);
        self.sink.free_local(iterable);

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

    fn scan_method_call(
        &mut self,
        target: &Located<parse::Target>,
        name: &Located<Identifier>,
        args: &[Located<parse::Expr>],
    ) -> Semantic<()> {
        self.address(target, |this, base, addressed| {
            #[derive(Copy, Clone)]
            enum Method {
                Insert,
                Delete,
                Neg,
                F,
                T,
            }

            use Addressed::*;
            use Method::*;

            const METHODS: &'static [(NoCase<&'static str>, Method)] = &[
                (NoCase::new("insert"), Insert),
                (NoCase::new("delete"), Delete),
                (NoCase::new("del"), Delete),
                (NoCase::new("neg"), Neg),
                (NoCase::new("f"), F),
                (NoCase::new("t"), T),
            ];

            let mut arg_locals = vec![base];
            match addressed {
                List | Mat | Pod(_) => (),
                ListEntry(local) | MatRow(local) | MatColumn(local) => arg_locals.push(local),
                MatEntry(from, to) | ListSlice(from, to) | MatSlice(from, to) => {
                    arg_locals.push(from);
                    arg_locals.push(to);
                }
            }

            macro_rules! mutator {
                ($op:literal) => {{
                    let builtin = match addressed {
                        List => concat!("builtin_", $op, "_list"),
                        Mat => concat!("builtin_", $op, "_mat"),
                        ListEntry(_) => concat!("builtin_", $op, "_entry_list"),
                        MatEntry(_, _) => concat!("builtin_", $op, "_entry_mat"),
                        MatRow(_) => concat!("builtin_", $op, "_row_mat"),
                        MatColumn(_) => concat!("builtin_", $op, "_column_mat"),
                        ListSlice(_, _) => concat!("builtin_", $op, "_slice_list"),
                        MatSlice(_, _) => concat!("builtin_", $op, "_slice_mat"),

                        Pod(_) => {
                            return Err(Located::at(
                                SemanticError::NoSuchMethod(name.as_ref().clone(), addressed),
                                name.location().clone(),
                            ));
                        }
                    };

                    (Some(builtin), &[][..])
                }};
            }

            let check_mat_mode = |this: &Self, expr| match this.const_eval(expr) {
                Some(Static::Int(0 | 1)) => Ok(()),
                Some(Static::Int(mode)) => Err(Located::at(
                    SemanticError::ExpectedMatMode(mode),
                    expr.location().clone(),
                )),

                _ => Ok(()),
            };

            let method = METHODS
                .iter()
                .find(|(key, _)| key == name.as_ref())
                .map(|(_, method)| *method);

            let (builtin, arg_types) = match (method, addressed) {
                (Some(Insert), List) => {
                    this.update_static(target.var(), |_, old| match old {
                        Static::List { length } => Some(Static::List { length: length + 1 }),
                        _ => None,
                    });

                    (Some("builtin_insert_list"), &[Type::Int, Type::Bool][..])
                }

                (Some(Insert), Mat) => {
                    if let Some(mode) = args.get(1) {
                        check_mat_mode(this, mode)?;
                    }

                    this.update_static(target.var(), |_, old| match old {
                        Static::Mat { rows, columns } => Some(Static::Mat {
                            rows: rows + 1,
                            columns,
                        }),

                        _ => None,
                    });

                    let (builtin, types) = if args.len() >= 3 {
                        ("builtin_insert_mat", &[Type::Mat, Type::Int, Type::Int][..])
                    } else {
                        ("builtin_insert_end_mat", &[Type::Mat, Type::Int][..])
                    };

                    (Some(builtin), types)
                }

                (Some(Delete), List) => {
                    this.update_static(target.var(), |_, old| match old {
                        Static::List { length } => Some(Static::List { length: length - 1 }),

                        _ => None,
                    });

                    (Some("builtin_delete_list"), &[Type::Int][..])
                }

                (Some(Delete), Mat) => {
                    if let Some(mode) = args.get(1) {
                        check_mat_mode(this, mode)?;
                    }

                    this.update_static(target.var(), |_, old| match old {
                        Static::Mat { rows, columns } if rows > 0 => Some(Static::Mat {
                            rows: rows - 1,
                            columns,
                        }),

                        _ => None,
                    });

                    (Some("builtin_delete_mat"), &[Type::Int, Type::Int][..])
                }

                (Some(Neg), Pod(Type::Bool)) => {
                    this.sink.push(Instruction::Not(base));
                    (None, &[][..])
                }

                (Some(F), Pod(Type::Bool)) => {
                    this.sink.push(Instruction::LoadConst(0, base));
                    (None, &[][..])
                }

                (Some(T), Pod(Type::Bool)) => {
                    this.sink.push(Instruction::LoadConst(1, base));
                    (None, &[][..])
                }

                (Some(Neg), _) => mutator!("neg"),
                (Some(F), _) => mutator!("f"),
                (Some(T), _) => mutator!("t"),

                _ => {
                    return Err(Located::at(
                        SemanticError::NoSuchMethod(name.as_ref().clone(), addressed),
                        name.location().clone(),
                    ));
                }
            };

            let args = this.alloc_expecting(name.location(), args, arg_types)?;
            if let Some(builtin) = builtin {
                arg_locals.extend(args.iter().map(|(local, _, _)| *local));

                this.sink.push(Instruction::Call {
                    target: Function::External(builtin),
                    arguments: arg_locals,
                    output: None,
                });
            }

            for (local, typ, ownership) in args.into_iter() {
                this.drop(local, typ, ownership);
            }

            Ok((builtin.is_none(), ()))
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

        self.scope.statics.clear();
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
            return self.assign_indexed(target, value);
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
                if let Some(value) = self.const_eval(value) {
                    self.scope.statics.insert(target.as_ref().clone(), value);
                }

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

                    let instruction = Instruction::StoreGlobal(value_local, global);
                    this.sink.push(instruction);

                    Ok((Type::Int, Ownership::Owned, ()))
                })?;
            }
        }

        let id = target.as_ref();
        if self.scope.statics.get(id).is_some() {
            match self.const_eval(value) {
                Some(new) => self.scope.statics.insert(id.clone(), new),
                None => self.scope.statics.remove(id),
            };
        }

        Ok(())
    }

    fn assign_indexed(
        &mut self,
        target: &Located<parse::Target>,
        value: &Located<parse::Expr>,
    ) -> Semantic<()> {
        self.address(target, |this, base, addressed| {
            use Addressed::*;
            let (builtin, typ, mut args) = match addressed {
                ListEntry(index) => ("builtin_set_entry_list", Type::Bool, vec![base, index]),
                MatEntry(row, col) => ("builtin_set_entry_mat", Type::Bool, vec![base, row, col]),
                MatRow(row) => ("builtin_set_row_mat", Type::List, vec![base, row]),
                MatColumn(col) => ("builtin_set_column_mat", Type::List, vec![base, col]),
                ListSlice(from, to) => ("builtin_set_slice_list", Type::List, vec![base, from, to]),
                MatSlice(from, to) => ("builtin_set_slice_mat", Type::Mat, vec![base, from, to]),
                List | Mat | Pod(_) => unreachable!(),
            };

            this.ephemeral(move |this, value_local| {
                let ownership = this.eval_expecting(value, value_local, typ)?;
                args.push(value_local);

                this.sink.push(Instruction::Call {
                    target: Function::External(builtin),
                    arguments: args,
                    output: None,
                });

                Ok((typ, ownership, (false, ())))
            })
        })
    }

    fn address<F, R>(&mut self, target: &Located<parse::Target>, callback: F) -> Semantic<R>
    where
        F: FnOnce(&mut Self, Local, Addressed) -> Semantic<(bool, R)>,
    {
        let base = self.sink.alloc_local();
        let base_type = self.read(target.var(), base)?;

        use Addressed::*;

        let mut addressed = match base_type {
            Type::Bool | Type::Int | Type::Float => Pod(base_type),
            Type::List => List,
            Type::Mat => Mat,
        };

        let first = self.sink.alloc_local();
        let second = self.sink.alloc_local();

        let single = |this: &mut Self, local, expr, constructor: &dyn Fn(Local) -> Addressed| {
            this.eval_expecting(expr, local, Type::Int)?;
            Ok(constructor(local))
        };

        let double = |this: &mut Self,
                      first_local,
                      second_local,
                      first_expr,
                      second_expr,
                      constructor: &dyn Fn(Local, Local) -> Addressed| {
            this.eval_expecting(first_expr, first_local, Type::Int)?;
            this.eval_expecting(second_expr, second_local, Type::Int)?;

            Ok(constructor(first_local, second_local))
        };

        let mut static_value = self.scope.statics.get(target.var().as_ref()).copied();

        for index in target.indices().iter() {
            use parse::Index;

            static_value = static_value
                .and_then(|value| self.check_index(value, index).transpose())
                .transpose()?;

            addressed = match (addressed, index.as_ref()) {
                (List, Index::Single(expr)) => single(self, first, expr, &ListEntry)?,

                (List, Index::Range(from, to)) => {
                    double(self, first, second, from, to, &ListSlice)?
                }

                (Mat, Index::Single(expr)) => single(self, first, expr, &MatRow)?,

                (Mat, Index::Range(from, to)) => double(self, first, second, from, to, &MatSlice)?,

                (Mat, Index::Indirect(row, col)) => {
                    double(self, first, second, row, col, &MatEntry)?
                }

                (Mat, Index::Transposed(expr)) => single(self, first, expr, &MatColumn)?,

                (MatRow(row), Index::Single(col)) => {
                    single(self, second, col, &|col| MatEntry(row, col))?
                }

                (MatColumn(col), Index::Single(row)) => {
                    single(self, second, row, &|row| MatEntry(row, col))?
                }

                _ => {
                    return Err(Located::at(
                        SemanticError::InvalidAddressing(base_type),
                        target.location().clone(),
                    ))
                }
            }
        }

        let (copy, result) = callback(self, base, addressed)?;
        self.sink.free_local(first);
        self.sink.free_local(second);

        if copy {
            let var = match self.scope.lookup(target.var())? {
                Named::Var(var) => var,
                _ => unreachable!(),
            };

            let instruction = match &var.access {
                Access::Local(local) => Instruction::Move(base, *local),
                Access::Global(global) => Instruction::StoreGlobal(base, global.clone()),
            };

            self.sink.push(instruction);
        }

        // No se ocupa self.drop(), necesariamente se está bajo Ownership::Borrowed
        self.sink.free_local(base);
        Ok(result)
    }

    fn parameter_types(&mut self, procedure: &parse::Procedure) -> Semantic<Vec<Type>> {
        procedure
            .parameters()
            .iter()
            .map(|param| self.scan_type(param.of()))
            .collect()
    }

    fn scan_type(&self, typ: &Located<parse::Type>) -> Semantic<Type> {
        match typ.as_ref() {
            parse::Type::Int => Ok(Type::Int),
            parse::Type::Bool => Ok(Type::Bool),
            parse::Type::List => Ok(Type::List),
            parse::Type::Mat => Ok(Type::Mat),
            parse::Type::Float => Ok(Type::Float),
            parse::Type::Of(expr) => self.type_check(expr),
        }
    }

    fn type_check(&self, expr: &Located<parse::Expr>) -> Semantic<Type> {
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
        let allocs = self.alloc_expecting(at, args, types)?;
        let arg_locals = allocs.iter().map(|(local, _, _)| *local).collect();

        self.sink.push(Instruction::Call {
            target: Function::External(builtin),
            arguments: arg_locals,
            output,
        });

        for (local, typ, ownership) in allocs.into_iter() {
            self.drop(local, typ, ownership);
            self.sink.free_local(local);
        }

        Ok(())
    }

    fn alloc_expecting<A>(
        &mut self,
        at: &Location,
        args: &[A],
        types: &[Type],
    ) -> Semantic<Vec<(Local, Type, Ownership)>>
    where
        A: Borrow<Located<parse::Expr>>,
    {
        if args.len() != types.len() {
            return Err(Located::at(
                SemanticError::BadArgumentCount(types.len(), args.len()),
                at.clone(),
            ));
        }

        let mut allocs = Vec::new();
        for (arg, typ) in args.iter().map(Borrow::borrow).zip(types.iter()) {
            let local = self.sink.alloc_local();
            let ownership = self.eval_expecting(arg, local, *typ)?;

            allocs.push((local, *typ, ownership));
        }

        Ok(allocs)
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

            New(typ) => {
                let typ = self.eval_new(typ, into)?;
                Ok((typ, Owned))
            }

            Cast(typ, inner) => {
                let (typ, ownership) = self.eval_cast(expr.location(), typ, inner, into)?;
                Ok((typ, ownership))
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

    fn eval_new(&mut self, typ: &Located<parse::Type>, into: Local) -> Semantic<Type> {
        let at = typ.location();
        let typ = self.scan_type(typ)?;

        match typ {
            Type::Int | Type::Bool => self.sink.push(Instruction::LoadConst(0, into)),
            Type::List => self.eval_fixed_call("builtin_new_list", at, &[], &[], Some(into))?,
            Type::Mat => self.eval_fixed_call("builtin_new_mat", at, &[], &[], Some(into))?,

            Type::Float => {
                self.sink.push(Instruction::LoadConst(0, into));
                self.sink.push(Instruction::Call {
                    target: Function::External("builtin_cast_int_float"),
                    arguments: vec![into],
                    output: Some(into),
                });
            }
        }

        Ok(typ)
    }

    fn eval_cast(
        &mut self,
        at: &Location,
        typ: &Located<parse::Type>,
        expr: &Located<parse::Expr>,
        into: Local,
    ) -> Semantic<(Type, Ownership)> {
        let (from_type, ownership) = self.eval(expr, into)?;
        let to_type = self.scan_type(typ)?;

        let caster = match (from_type, to_type) {
            (a, b) if a == b => None,

            (Type::Int, Type::Bool) => self.ephemeral(|this, zero| {
                this.sink.push(Instruction::LoadConst(0, zero));

                let op = ir::BinOp::Logic(ir::LogicOp::NotEqual);
                this.sink.push(Instruction::Binary(into, op, zero));

                Ok((Type::Int, Ownership::Owned, None))
            })?,

            // Este cast no altera la representación binaria
            (Type::Bool, Type::Int) => None,

            (Type::Int, Type::Float) => Some("builtin_cast_int_float"),
            (Type::Float, Type::Int) => Some("builtin_cast_float_int"),

            _ => {
                return Err(Located::at(
                    SemanticError::BadCast(from_type, to_type),
                    at.clone(),
                ))
            }
        };

        if let Some(caster) = caster {
            self.sink.push(Instruction::Call {
                target: Function::External(caster),
                arguments: vec![into],
                output: Some(into),
            });
        }

        Ok((to_type, ownership))
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

            let expect_non_zero = |this: &Self, expr| match this.const_eval(expr) {
                Some(Static::Int(0)) => Err(Located::at(
                    SemanticError::DivisionByZero,
                    expr.location().clone(),
                )),

                _ => Ok(()),
            };

            let op = match (op, typ) {
                (_, Float) => {
                    let typ = this.do_float_binary(at, into, op, rhs_local)?;
                    return Ok((Type::Float, Ownership::Owned, typ));
                }

                (ParseOp::Add, Int) => IrOp::Arithmetic(ArithmeticOp::Add),
                (ParseOp::Sub, Int) => IrOp::Arithmetic(ArithmeticOp::Sub),
                (ParseOp::Mul, Int) => IrOp::Arithmetic(ArithmeticOp::Mul),

                (ParseOp::Mod, Int) => {
                    expect_non_zero(this, rhs)?;
                    IrOp::Arithmetic(ArithmeticOp::Mod)
                }

                (ParseOp::IntegerDiv, Int) => {
                    expect_non_zero(this, rhs)?;
                    IrOp::Arithmetic(ArithmeticOp::Div)
                }

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

        let (is_mat, expected_columns) = match items.first() {
            None => (false, None),

            Some(first) => match self.type_check(first)? {
                Type::Bool => (false, None),
                Type::List => {
                    let columns = match self.const_eval(first) {
                        Some(Static::List { length }) => Some(length),
                        _ => None,
                    };

                    (true, columns)
                }

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

        let index = self.sink.alloc_local();
        for (i, expr) in items.iter().enumerate() {
            self.sink.push(Instruction::LoadConst(i as i32, index));

            let (insert, expected, arguments) = if is_mat {
                ("builtin_push_mat", Type::List, vec![into, item])
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

            if let Some(expected_columns) = expected_columns {
                match self.const_eval(expr) {
                    Some(Static::List { length }) if length != expected_columns => {
                        return Err(Located::at(
                            SemanticError::ExpectedColumns(
                                expected_columns as usize,
                                length as usize,
                            ),
                            expr.location().clone(),
                        ));
                    }

                    _ => (),
                }
            }
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

        self.const_eval(base)
            .and_then(|base| self.check_index(base, index).transpose())
            .transpose()?;

        Ok(typ)
    }

    fn const_eval(&self, expr: &Located<parse::Expr>) -> Option<Static> {
        use parse::Expr::{self, *};
        use Static::{List, *};

        match expr.as_ref() {
            True => Some(Bool(true)),
            False => Some(Bool(false)),
            Integer(integer) => Some(Int(*integer)),
            Read(id) => self.scope.lookup_static(id),

            Attr(base, attr) => {
                let (base, attr) = (self.const_eval(base)?, attr.as_ref().as_ref());
                match (base, attr) {
                    (Mat { rows, .. }, attr) if attr == "shapeF" => Some(Int(rows)),
                    (Mat { columns, .. }, attr) if attr == "shapeC" => Some(Int(columns)),
                    _ => None,
                }
            }

            Index(base, index) => {
                use parse::Index::*;

                let (base, index) = (self.const_eval(base)?, index.as_ref().as_ref());
                match (base, index) {
                    (Mat { columns, .. }, Single(_)) => Some(List { length: columns }),
                    (Mat { rows, .. }, Transposed(_)) => Some(List { length: rows }),

                    (List { length }, Range(from, to)) => {
                        let (from, to) = (self.const_eval(from)?, self.const_eval(to)?);
                        match (from, to) {
                            (Int(from), Int(to))
                                if from >= 0 && to >= 0 && from < length && to <= length =>
                            {
                                Some(List { length: to - from })
                            }

                            _ => None,
                        }
                    }

                    (Mat { rows, columns }, Range(from, to)) => {
                        let (from, to) = (self.const_eval(from)?, self.const_eval(to)?);
                        match (from, to) {
                            (Int(from), Int(to))
                                if from >= 0 && to >= 0 && from < rows && to <= rows =>
                            {
                                Some(Mat {
                                    rows: to - from,
                                    columns,
                                })
                            }

                            _ => None,
                        }
                    }

                    _ => None,
                }
            }

            Len(expr) => match self.const_eval(expr)? {
                List { length } => Some(Int(length)),
                Mat { rows, .. } => Some(Int(rows)),
                _ => None,
            },

            Range(length, _) => match self.const_eval(length)? {
                Int(length) => Some(List {
                    length: length.max(0),
                }),
                _ => None,
            },

            Expr::List(items) => match items.first().map(|first| self.type_check(first)) {
                None => Some(List { length: 0 }),
                Some(Ok(Type::Bool)) => Some(List {
                    length: items.len() as i32,
                }),
                Some(Ok(Type::List)) => match self.const_eval(items.first().unwrap())? {
                    List { length } => Some(Mat {
                        rows: items.len() as i32,
                        columns: length,
                    }),
                    _ => None,
                },

                Some(_) => None,
            },

            New(typ) => match self.scan_type(typ) {
                Ok(Type::Bool) => Some(Bool(false)),
                Ok(Type::Int) => Some(Int(0)),
                Ok(Type::Float) => Some(Float(0.0)),
                Ok(Type::List) => Some(List { length: 0 }),
                Ok(Type::Mat) => Some(Mat {
                    rows: 0,
                    columns: 0,
                }),
                Err(_) => None,
            },

            Cast(typ, expr) => match (self.const_eval(expr)?, self.scan_type(typ)) {
                (Bool(boolean), Ok(Type::Bool)) => Some(Bool(boolean)),
                (Int(integer), Ok(Type::Int)) => Some(Int(integer)),
                (Float(float), Ok(Type::Float)) => Some(Float(float)),
                (list @ List { .. }, Ok(Type::List)) => Some(list),
                (mat @ Mat { .. }, Ok(Type::Mat)) => Some(mat),

                (Bool(boolean), Ok(Type::Int)) => Some(Int(boolean as i32)),
                (Int(integer), Ok(Type::Bool)) => Some(Bool(integer != 0)),
                (Int(integer), Ok(Type::Float)) => Some(Float(integer as f32)),
                (Float(float), Ok(Type::Int)) => Some(Int(float as i32)),

                _ => None,
            },

            Negate(expr) => match self.const_eval(expr)? {
                Int(integer) => Some(Int(-integer)),
                _ => None,
            },

            Binary { lhs, op, rhs, .. } => {
                use parse::BinOp::*;

                match (self.const_eval(lhs)?, op, self.const_eval(rhs)?) {
                    (Bool(lhs), Equal, Bool(rhs)) => Some(Bool(lhs == rhs)),
                    (Bool(lhs), NotEqual, Bool(rhs)) => Some(Bool(lhs != rhs)),

                    (Int(lhs), Add, Int(rhs)) => Some(Int(lhs + rhs)),
                    (Int(lhs), Sub, Int(rhs)) => Some(Int(lhs - rhs)),
                    (Int(lhs), Mul, Int(rhs)) => Some(Int(lhs * rhs)),
                    (Int(lhs), Pow, Int(rhs)) => Some(Float((lhs as f32).powf(rhs as f32))),
                    (Int(lhs), Div, Int(rhs)) => Some(Float(lhs as f32 / rhs as f32)),
                    (Int(lhs), Mod, Int(rhs)) if rhs != 0 => Some(Int(lhs % rhs)),
                    (Int(lhs), IntegerDiv, Int(rhs)) if rhs != 0 => Some(Int(lhs / rhs)),
                    (Int(lhs), Equal, Int(rhs)) if rhs != 0 => Some(Bool(lhs == rhs)),
                    (Int(lhs), NotEqual, Int(rhs)) if rhs != 0 => Some(Bool(lhs != rhs)),
                    (Int(lhs), Greater, Int(rhs)) if rhs != 0 => Some(Bool(lhs > rhs)),
                    (Int(lhs), GreaterOrEqual, Int(rhs)) if rhs != 0 => Some(Bool(lhs >= rhs)),
                    (Int(lhs), Less, Int(rhs)) if rhs != 0 => Some(Bool(lhs < rhs)),
                    (Int(lhs), LessOrEqual, Int(rhs)) if rhs != 0 => Some(Bool(lhs <= rhs)),

                    (Float(lhs), Add, Float(rhs)) => Some(Float(lhs + rhs)),
                    (Float(lhs), Sub, Float(rhs)) => Some(Float(lhs - rhs)),
                    (Float(lhs), Mul, Float(rhs)) => Some(Float(lhs * rhs)),
                    (Float(lhs), Pow, Float(rhs)) => Some(Float(lhs.powf(rhs))),
                    (Float(lhs), Div, Float(rhs)) => Some(Float(lhs / rhs)),
                    (Float(lhs), Equal, Float(rhs)) => Some(Bool(lhs == rhs)),
                    (Float(lhs), NotEqual, Float(rhs)) => Some(Bool(lhs != rhs)),
                    (Float(lhs), Greater, Float(rhs)) => Some(Bool(lhs > rhs)),
                    (Float(lhs), GreaterOrEqual, Float(rhs)) => Some(Bool(lhs >= rhs)),
                    (Float(lhs), Less, Float(rhs)) => Some(Bool(lhs < rhs)),
                    (Float(lhs), LessOrEqual, Float(rhs)) => Some(Bool(lhs <= rhs)),

                    _ => None,
                }
            }
        }
    }

    fn check_index(&self, base: Static, index: &Located<parse::Index>) -> Semantic<Option<Static>> {
        use parse::Index;
        use Static::*;

        let check = |length, index| match self.const_eval(index) {
            Some(Int(value)) if (0..length).contains(&value) => Ok(Some(value)),
            Some(Int(value)) => Err(Located::at(
                SemanticError::OutOfBounds(value, length, '['),
                index.location().clone(),
            )),

            _ => Ok(None),
        };

        let check_range = |length, from, to| {
            let from_value = check(length, from);
            let to_value = match self.const_eval(to) {
                Some(Int(to_value)) if (0..=length).contains(&to_value) => Some(to_value),

                Some(Int(value)) => {
                    return Err(Located::at(
                        SemanticError::OutOfBounds(value, length, ']'),
                        to.location().clone(),
                    ))
                }

                _ => None,
            };

            match (from_value, to_value) {
                (Err(error), _) => Err(error),
                (Ok(Some(from)), Some(to)) => Ok(Some(to - from)),
                (Ok(_), _) => Ok(None),
            }
        };

        match (base, index.as_ref()) {
            (List { length }, Index::Single(index)) => {
                check(length, index)?;
            }

            (List { length }, Index::Range(from, to)) => {
                if let Some(slice_length) = check_range(length, from, to)? {
                    return Ok(Some(List {
                        length: slice_length,
                    }));
                }
            }

            (Mat { rows, columns }, Index::Single(index)) => {
                check(rows, index)?;
                return Ok(Some(List { length: columns }));
            }

            (Mat { rows, columns }, Index::Range(from, to)) => {
                if let Some(slice_rows) = check_range(rows, from, to)? {
                    return Ok(Some(Mat {
                        rows: slice_rows,
                        columns,
                    }));
                }
            }

            (Mat { rows, columns }, Index::Indirect(first, second)) => {
                check(rows, first)?;
                check(columns, second)?;
            }

            (Mat { rows, columns }, Index::Transposed(index)) => {
                check(columns, index)?;
                return Ok(Some(List { length: rows }));
            }

            _ => (),
        }

        Ok(None)
    }

    fn update_static<F>(&mut self, id: &Located<Identifier>, updater: F)
    where
        F: FnOnce(&mut Self, Static) -> Option<Static>,
    {
        let id = id.as_ref();
        if let Some(old) = self.scope.lookup_static(id) {
            match updater(self, old) {
                Some(new) => self.scope.statics.insert(id.clone(), new),
                None => self.scope.statics.remove(id),
            };
        }
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
        self.scope.statics.clear();

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

fn drop_globals<S: Sink>(sink: &mut S, globals: &SymbolTable<'_>) {
    for named in globals.symbols.values() {
        if let Named::Var(Variable {
            access: Access::Global(global),
            typ,
        }) = named
        {
            if let Some(destructor) = destructor(*typ, Ownership::Owned) {
                // Ya no quedan otras locales
                let local = Local::default();
                let load = Instruction::LoadGlobal(global.clone(), local);

                sink.push(load);
                sink.push(Instruction::Call {
                    target: Function::External(destructor),
                    arguments: vec![local],
                    output: None,
                });
            }
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
