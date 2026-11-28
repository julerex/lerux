use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Debug)]
pub(crate) struct LetStmt {
    pub name: String,
    pub init: Expr,
}

#[derive(Debug)]
pub(crate) struct FunctionStmt {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub(crate) struct IfStmt {
    pub test: Expr,
    pub consequent: Stmt,
    pub alternate: Option<Stmt>,
}

#[derive(Debug)]
pub(crate) enum Stmt {
    Let(Box<LetStmt>),
    Function(Box<FunctionStmt>),
    If(Box<IfStmt>),
    Return(Option<Expr>),
    Expr(Expr),
    Block(Vec<Stmt>),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum UnaryOp {
    Neg,
    Pos,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug)]
pub(crate) enum Expr {
    Number(f64),
    String(String),
    Boolean(bool),
    Undefined,
    Ident(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}
