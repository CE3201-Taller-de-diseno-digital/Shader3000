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
    source::{Located, Location},
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

    #[error("Floats are not supported by this implementation")]
    Floats,
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
            use parse::{ObjectKind::*, Statement::*, TimeUnit::*};

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

                    self.eval_fixed_call(builtin, location, None, &args, &types, None)?;
                }

                Delay { count, unit } => {
                    let builtin = match unit {
                        Millis => "builtin_delay_mil",
                        Seconds => "builtin_delay_seg",
                        Minutes => "builtin_delay_min",
                    };

                    let types = [Type::Int];
                    let location = count.location();
                    self.eval_fixed_call(builtin, location, None, &[count], &types, None)?;
                }

                PrintLed { column, row, value } => {
                    let args = [column, row, value];
                    let types = [Type::Int, Type::Int, Type::Bool];
                    let location = value.location();
                    let builtin = "builtin_printled";

                    self.eval_fixed_call(builtin, location, None, &args, &types, None)?;
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

                    self.eval_fixed_call(builtin, location, None, &args, &types, None)?;
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

        self.scan_statements(body)?;
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

    fn parameter_types(&mut self, procedure: &parse::Procedure) -> Semantic<Vec<Type>> {
        procedure
            .parameters()
            .iter()
            .map(|param| match param.of().as_ref() {
                parse::Type::Int => Ok(Type::Int),
                parse::Type::Bool => Ok(Type::Bool),
                parse::Type::List => Ok(Type::List),
                parse::Type::Mat => Ok(Type::Mat),
                parse::Type::Of(expr) => self.type_check(expr),
            })
            .collect()
    }

    fn type_check(&mut self, expr: &Located<parse::Expr>) -> Semantic<Type> {
        let mut context = Context {
            scope: SymbolTable {
                outer: Some(&self.scope),
                symbols: Default::default(),
            },

            sink: TypeCheck,
        };

        let (typ, _) = context.eval(expr, Local::default())?;
        Ok(typ)
    }

    fn eval_fixed_call(
        &mut self,
        builtin: &'static str,
        at: &Location,
        subject: Option<Local>,
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

        if let Some(subject) = subject {
            arg_locals.push(subject);
        }

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
            .skip(subject.iter().count())
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
            (Type::Int, _) => None,
            (Type::Bool, _) => None,
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
            Attr(_target, _attr) => todo!(),

            Len(expr) => {
                self.eval_len(expr, into)?;
                Ok((Type::Int, Owned))
            }

            Range(length, value) => {
                let builtin = "builtin_range";
                let args = [&**length, &**value];
                let types = [Type::Int, Type::Bool];
                let location = value.location();

                self.eval_fixed_call(builtin, location, None, &args, &types, Some(into))?;
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
                (ParseOp::Add, Int) => IrOp::Arithmetic(ArithmeticOp::Add),
                (ParseOp::Sub, Int) => IrOp::Arithmetic(ArithmeticOp::Sub),
                (ParseOp::Mul, Int) => IrOp::Arithmetic(ArithmeticOp::Mul),
                (ParseOp::Mod, Int) => IrOp::Arithmetic(ArithmeticOp::Mod),
                (ParseOp::IntegerDiv, Int) => IrOp::Arithmetic(ArithmeticOp::Div),

                (ParseOp::Equal, Int | Bool) => IrOp::Logic(LogicOp::Equal),
                (ParseOp::NotEqual, Int | Bool) => IrOp::Logic(LogicOp::NotEqual),
                (ParseOp::Greater, Int | Bool) => IrOp::Logic(LogicOp::Greater),
                (ParseOp::GreaterOrEqual, Int | Bool) => IrOp::Logic(LogicOp::GreaterOrEqual),
                (ParseOp::Less, Int | Bool) => IrOp::Logic(LogicOp::Less),
                (ParseOp::LessOrEqual, Int | Bool) => IrOp::Logic(LogicOp::LessOrEqual),

                (ParseOp::Equal | ParseOp::NotEqual, List | Mat) => {
                    let comparator = if typ == List {
                        "builtin_cmp_list"
                    } else {
                        "builtin_cmp_mat"
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

                (ParseOp::Div, Int) => return Err(Located::at(SemanticError::Floats, at.clone())),

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

    fn eval_len(&mut self, expr: &Located<parse::Expr>, into: Local) -> Semantic<()> {
        self.ephemeral(|this, arg| {
            let (arg_type, arg_ownership) = this.eval(expr, arg)?;
            let target = match arg_type {
                Type::List => Function::External("builtin_len_list"),
                Type::Mat => Function::External("builtin_len_mat"),

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

        let mut typ = var.typ;
        let mut ownership = Ownership::Borrowed;

        for index in target.indices() {
            use parse::Index;

            let expect_mat = || {
                (typ == Type::Mat).then(|| ()).ok_or_else(|| {
                    Located::at(
                        SemanticError::ExpectedType(Type::Mat, typ),
                        target.var().location().clone(),
                    )
                })
            };

            let expect_list_or_mat = || {
                (typ == Type::List || typ == Type::Mat)
                    .then(|| ())
                    .ok_or_else(|| {
                        Located::at(
                            SemanticError::ExpectedTwo(Type::List, Type::Mat, typ),
                            target.var().location().clone(),
                        )
                    })
            };

            let (builtin, next_type, args) = match index.as_ref() {
                Index::Single(expr) => {
                    expect_list_or_mat()?;
                    let (builtin, next_type) = if typ == Type::List {
                        ("builtin_index_list", Type::Bool)
                    } else {
                        ("builtin_index_row_mat", Type::List)
                    };

                    (builtin, next_type, vec![expr])
                }

                Index::Range(from, to) => {
                    expect_list_or_mat()?;
                    let builtin = if typ == Type::List {
                        "builtin_slice_list"
                    } else {
                        "builtin_slice_mat"
                    };

                    (builtin, typ, vec![from, to])
                }

                Index::Indirect(row, column) => {
                    expect_mat()?;
                    ("builtin_index_entry_mat", Type::Bool, vec![row, column])
                }

                Index::Transposed(column) => {
                    expect_mat()?;
                    ("builtin_index_column_mat", Type::List, vec![column])
                }
            };

            const TYPES: [Type; 2] = [Type::Int, Type::Int];

            let location = index.location();
            let types = &TYPES[..args.len()];

            if destructor(typ, ownership).is_none() {
                self.eval_fixed_call(builtin, location, Some(into), &args, types, Some(into))?;
            } else {
                self.ephemeral(|this, local| {
                    this.eval_fixed_call(builtin, location, Some(into), &args, types, Some(local))?;

                    this.drop(into, typ, ownership);
                    this.sink.push(Instruction::Move(local, into));

                    // Se tomó ownership con Instruction::Move(), no se debe hacer drop
                    Ok((Type::Int, Ownership::Borrowed, ()))
                })?;
            }

            ownership = Ownership::Owned;
            typ = next_type;
        }

        Ok((typ, ownership))
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
        (Type::Int, _) => None,
        (Type::Bool, _) => None,
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
        }));
    }

    mangled
}
