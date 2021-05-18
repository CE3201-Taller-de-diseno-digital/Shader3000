use std::rc::Rc;

pub struct Program {
    pub globals: Vec<Rc<Global>>,
    pub code: Vec<Rc<Function>>,
}

pub struct Function {
    pub name: String,
    pub body: FunctionBody,
    pub parameters: u32,
}

pub enum FunctionBody {
    External,
    Generated {
        inner_locals: u32,
        instructions: Vec<Instruction>,
    },
}

#[derive(Copy, Clone)]
pub struct Label(pub u32);

#[derive(Copy, Clone)]
pub struct Local(pub u32);

pub struct Global(pub String);

pub enum Instruction {
    Label(Label),
    Jump(Label),
    JumpIfFalse(Local, Label),
    LoadGlobal(Rc<Global>, Local),
    StoreGlobal(Local, Rc<Global>),
    Call {
        target: Rc<Function>,
        arguments: Vec<Local>,
        output: Option<Local>,
    },
}
