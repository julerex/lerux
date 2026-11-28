//! Bytecode for the subset: constants, locals, upvalues, arithmetic, jumps, calls, and return.
//!
//! Expression values sit on the operand stack. `let` bindings live in an
//! environment object. A function reads an outer binding through an upvalue.

use alloc::{string::String, vec::Vec};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Instr {
    Const(u16),
    Undefined,
    Load(u16),
    Store(u16),
    LoadUp(u8),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    Pos,
    Jump(u16),
    JumpIfFalse(u16),
    Call(u8),
    Return,
}

#[derive(Clone, Debug)]
pub(crate) enum Const {
    Number(f64),
    Boolean(bool),
    String(String),
    Function(u16),
}

/// A binding owned by an environment `hops` parents away.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Upvalue {
    pub hops: u8,
    pub slot: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct Chunk {
    pub arity: u16,
    pub locals: u16,
    pub code: Vec<Instr>,
    pub constants: Vec<Const>,
    pub upvalues: Vec<Upvalue>,
}

impl Chunk {
    pub(crate) fn placeholder() -> Self {
        Self {
            arity: 0,
            locals: 0,
            code: Vec::new(),
            constants: Vec::new(),
            upvalues: Vec::new(),
        }
    }
}
