//! Walk the abstract syntax tree and emit bytecode.

use alloc::{string::String, vec::Vec};

use crate::{
    ast::{BinaryOp, Expr, FunctionStmt, IfStmt, LetStmt, Stmt, UnaryOp},
    error::Error,
    opcode::{Chunk, Const, Instr, Upvalue},
};

pub(crate) fn program(body: &[Stmt]) -> Result<Vec<Chunk>, Error> {
    let mut chunks = Vec::new();
    chunks.push(Chunk::placeholder());
    let chunk = compile_script(body, &mut chunks)?;
    let Some(slot) = chunks.first_mut() else {
        return Err(Error::syntax());
    };
    *slot = chunk;
    Ok(chunks)
}

fn compile_script(body: &[Stmt], chunks: &mut Vec<Chunk>) -> Result<Chunk, Error> {
    let mut cx = Cx::new(Vec::new());
    cx.items(body, chunks, true)?;
    cx.implicit_return()?;
    Ok(cx.finish(0))
}

fn compile_child(
    chunks: &mut Vec<Chunk>,
    parent: &[Name],
    fun: &FunctionStmt,
) -> Result<u16, Error> {
    let index = u16::try_from(chunks.len()).map_err(|_| Error::syntax())?;
    chunks.push(Chunk::placeholder());
    let chunk = compile_function(chunks, parent, fun)?;
    let Some(slot) = chunks.get_mut(usize::from(index)) else {
        return Err(Error::syntax());
    };
    *slot = chunk;
    Ok(index)
}

fn compile_function(
    chunks: &mut Vec<Chunk>,
    parent: &[Name],
    fun: &FunctionStmt,
) -> Result<Chunk, Error> {
    let mut cx = Cx::new(child_names(parent)?);
    let arity = u8::try_from(fun.params.len()).map_err(|_| Error::syntax())?;
    for param in &fun.params {
        cx.declare(param, true)?;
    }
    cx.items(&fun.body, chunks, false)?;
    cx.implicit_return()?;
    Ok(cx.finish(u16::from(arity)))
}

fn child_names(parent: &[Name]) -> Result<Vec<Name>, Error> {
    let mut names = Vec::with_capacity(parent.len());
    for name in parent {
        names.push(name.for_child()?);
    }
    Ok(names)
}

struct Name {
    text: String,
    slot: u16,
    scope: u16,
    ready: bool,
    hops: u8,
}

impl Name {
    fn for_child(&self) -> Result<Self, Error> {
        Ok(Self {
            text: self.text.clone(),
            slot: self.slot,
            scope: self.scope,
            ready: self.ready,
            hops: self.hops.checked_add(1).ok_or(Error::syntax())?,
        })
    }
}

struct Cx {
    names: Vec<Name>,
    scope: u16,
    next_local: u16,
    code: Vec<Instr>,
    constants: Vec<Const>,
    upvalues: Vec<Upvalue>,
}

impl Cx {
    fn new(names: Vec<Name>) -> Self {
        Self {
            names,
            scope: 1,
            next_local: 0,
            code: Vec::new(),
            constants: Vec::new(),
            upvalues: Vec::new(),
        }
    }

    fn finish(self, arity: u16) -> Chunk {
        Chunk {
            arity,
            locals: self.next_local,
            code: self.code,
            constants: self.constants,
            upvalues: self.upvalues,
        }
    }

    fn items(&mut self, items: &[Stmt], chunks: &mut Vec<Chunk>, tail: bool) -> Result<(), Error> {
        self.declare_bindings(items)?;
        self.emit_functions(items, chunks)?;
        self.emit_rest(items, chunks, tail)
    }

    fn declare_bindings(&mut self, items: &[Stmt]) -> Result<(), Error> {
        for item in items {
            match item {
                Stmt::Let(stmt) => {
                    self.declare(&stmt.name, false)?;
                }
                Stmt::Function(stmt) => {
                    self.declare(&stmt.name, false)?;
                }
                Stmt::If(_) | Stmt::Return(_) | Stmt::Expr(_) | Stmt::Block(_) => {}
            }
        }
        Ok(())
    }

    fn emit_functions(&mut self, items: &[Stmt], chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        for item in items {
            if let Stmt::Function(fun) = item {
                self.emit_function(fun, chunks)?;
            }
        }
        Ok(())
    }

    fn emit_rest(
        &mut self,
        items: &[Stmt],
        chunks: &mut Vec<Chunk>,
        tail: bool,
    ) -> Result<(), Error> {
        let last = items.len().saturating_sub(1);
        for (index, item) in items.iter().enumerate() {
            if matches!(item, Stmt::Function(_)) {
                continue;
            }
            self.statement(item, chunks, tail && index == last)?;
        }
        Ok(())
    }

    fn statement(&mut self, stmt: &Stmt, chunks: &mut Vec<Chunk>, tail: bool) -> Result<(), Error> {
        match stmt {
            Stmt::Let(stmt) => self.let_stmt(stmt, chunks),
            Stmt::Function(_) => Ok(()),
            Stmt::If(stmt) => self.if_stmt(stmt, chunks),
            Stmt::Return(expr) => self.return_stmt(expr.as_ref(), chunks),
            Stmt::Expr(expr) => self.expr_stmt(expr, chunks, tail),
            Stmt::Block(body) => self.block(body, chunks, tail),
        }
    }

    fn let_stmt(&mut self, stmt: &LetStmt, chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        self.expr(&stmt.init, chunks)?;
        self.mark_ready(&stmt.name);
        let slot = self.local_slot(&stmt.name)?;
        self.emit(Instr::Store(slot))
    }

    fn emit_function(&mut self, fun: &FunctionStmt, chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        let index = compile_child(chunks, &self.names, fun)?;
        self.emit_const(Const::Function(index))?;
        let slot = self.local_slot(&fun.name)?;
        self.emit(Instr::Store(slot))?;
        self.mark_ready(&fun.name);
        Ok(())
    }

    fn if_stmt(&mut self, stmt: &IfStmt, chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        self.expr(&stmt.test, chunks)?;
        let jump_else = self.emit_jump(Instr::JumpIfFalse(0))?;
        self.statement(&stmt.consequent, chunks, false)?;
        self.finish_if(stmt.alternate.as_ref(), chunks, jump_else)
    }

    fn finish_if(
        &mut self,
        alternate: Option<&Stmt>,
        chunks: &mut Vec<Chunk>,
        jump_else: usize,
    ) -> Result<(), Error> {
        let Some(alternate) = alternate else {
            return self.patch(jump_else);
        };
        let jump_end = self.emit_jump(Instr::Jump(0))?;
        self.patch(jump_else)?;
        self.statement(alternate, chunks, false)?;
        self.patch(jump_end)
    }

    fn return_stmt(&mut self, expr: Option<&Expr>, chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        if let Some(expr) = expr {
            self.expr(expr, chunks)?;
        } else {
            self.emit(Instr::Undefined)?;
        }
        self.emit(Instr::Return)
    }

    fn expr_stmt(&mut self, expr: &Expr, chunks: &mut Vec<Chunk>, tail: bool) -> Result<(), Error> {
        self.expr(expr, chunks)?;
        if tail {
            self.emit(Instr::Return)
        } else {
            self.emit(Instr::Pop)
        }
    }

    fn block(&mut self, body: &[Stmt], chunks: &mut Vec<Chunk>, tail: bool) -> Result<(), Error> {
        let scope = self.scope;
        self.scope = scope.checked_add(1).ok_or(Error::syntax())?;
        self.items(body, chunks, tail)?;
        self.scope = scope;
        self.names.retain(|name| name.scope <= scope);
        Ok(())
    }

    fn expr(&mut self, expr: &Expr, chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        match expr {
            Expr::Number(number) => self.emit_const(Const::Number(*number)),
            Expr::String(text) => self.emit_const(Const::String(text.clone())),
            Expr::Boolean(bit) => self.emit_const(Const::Boolean(*bit)),
            Expr::Undefined => self.emit(Instr::Undefined),
            Expr::Ident(name) => self.load_name(name),
            Expr::Unary(op, inner) => self.unary(*op, inner, chunks),
            Expr::Binary(op, left, right) => self.binary(*op, left, right, chunks),
            Expr::Call(callee, args) => self.call(callee, args, chunks),
        }
    }

    fn unary(&mut self, op: UnaryOp, inner: &Expr, chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        self.expr(inner, chunks)?;
        self.emit(match op {
            UnaryOp::Neg => Instr::Neg,
            UnaryOp::Pos => Instr::Pos,
        })
    }

    fn binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        chunks: &mut Vec<Chunk>,
    ) -> Result<(), Error> {
        self.expr(left, chunks)?;
        self.expr(right, chunks)?;
        self.emit(binary_instr(op))
    }

    fn call(&mut self, callee: &Expr, args: &[Expr], chunks: &mut Vec<Chunk>) -> Result<(), Error> {
        self.expr(callee, chunks)?;
        for arg in args {
            self.expr(arg, chunks)?;
        }
        let count = u8::try_from(args.len()).map_err(|_| Error::syntax())?;
        self.emit(Instr::Call(count))
    }

    fn load_name(&mut self, text: &str) -> Result<(), Error> {
        match self.resolve(text)? {
            Place::Local(slot) => self.emit(Instr::Load(slot)),
            Place::Up(index) => self.emit(Instr::LoadUp(index)),
        }
    }

    fn implicit_return(&mut self) -> Result<(), Error> {
        self.emit(Instr::Undefined)?;
        self.emit(Instr::Return)
    }

    fn declare(&mut self, text: &str, ready: bool) -> Result<u16, Error> {
        if self.duplicate(text) {
            return Err(Error::syntax());
        }
        let slot = self.next_local;
        self.next_local = slot.checked_add(1).ok_or(Error::syntax())?;
        self.names.push(Name {
            text: String::from(text),
            slot,
            scope: self.scope,
            ready,
            hops: 0,
        });
        Ok(slot)
    }

    fn duplicate(&self, text: &str) -> bool {
        self.names
            .iter()
            .any(|name| name.hops == 0 && name.scope == self.scope && name.text == text)
    }

    fn resolve(&mut self, text: &str) -> Result<Place, Error> {
        let (hops, slot, ready) = self.lookup(text)?;
        if hops == 0 {
            if !ready {
                return Err(Error::syntax());
            }
            return Ok(Place::Local(slot));
        }
        Ok(Place::Up(self.upvalue(hops, slot)?))
    }

    fn lookup(&self, text: &str) -> Result<(u8, u16, bool), Error> {
        let name = self
            .names
            .iter()
            .rev()
            .find(|name| name.text == text)
            .ok_or(Error::syntax())?;
        Ok((name.hops, name.slot, name.ready))
    }

    fn upvalue(&mut self, hops: u8, slot: u16) -> Result<u8, Error> {
        if let Some(index) = self.up_index(hops, slot) {
            return u8::try_from(index).map_err(|_| Error::syntax());
        }
        let index = u8::try_from(self.upvalues.len()).map_err(|_| Error::syntax())?;
        self.upvalues.push(Upvalue { hops, slot });
        Ok(index)
    }

    fn up_index(&self, hops: u8, slot: u16) -> Option<usize> {
        self.upvalues
            .iter()
            .position(|up| up.hops == hops && up.slot == slot)
    }

    fn local_slot(&self, text: &str) -> Result<u16, Error> {
        self.names
            .iter()
            .rev()
            .find(|name| name.hops == 0 && name.text == text)
            .map(|name| name.slot)
            .ok_or(Error::syntax())
    }

    fn mark_ready(&mut self, text: &str) {
        if let Some(name) = self
            .names
            .iter_mut()
            .rev()
            .find(|name| name.hops == 0 && name.text == text)
        {
            name.ready = true;
        }
    }

    fn emit_const(&mut self, constant: Const) -> Result<(), Error> {
        let index = u16::try_from(self.constants.len()).map_err(|_| Error::syntax())?;
        self.constants.push(constant);
        self.emit(Instr::Const(index))
    }

    fn emit(&mut self, instr: Instr) -> Result<(), Error> {
        self.reserve()?;
        self.code.push(instr);
        Ok(())
    }

    fn emit_jump(&mut self, instr: Instr) -> Result<usize, Error> {
        let index = self.code.len();
        self.emit(instr)?;
        Ok(index)
    }

    fn patch(&mut self, index: usize) -> Result<(), Error> {
        let target = u16::try_from(self.code.len()).map_err(|_| Error::syntax())?;
        let Some(instr) = self.code.get_mut(index) else {
            return Err(Error::syntax());
        };
        match instr {
            Instr::Jump(slot) | Instr::JumpIfFalse(slot) => *slot = target,
            Instr::Const(_)
            | Instr::Undefined
            | Instr::Load(_)
            | Instr::Store(_)
            | Instr::LoadUp(_)
            | Instr::Pop
            | Instr::Add
            | Instr::Sub
            | Instr::Mul
            | Instr::Div
            | Instr::Rem
            | Instr::Neg
            | Instr::Pos
            | Instr::Call(_)
            | Instr::Return => return Err(Error::syntax()),
        }
        Ok(())
    }

    fn reserve(&self) -> Result<(), Error> {
        if self.code.len() >= usize::from(u16::MAX) {
            return Err(Error::syntax());
        }
        Ok(())
    }
}

enum Place {
    Local(u16),
    Up(u8),
}

fn binary_instr(op: BinaryOp) -> Instr {
    match op {
        BinaryOp::Add => Instr::Add,
        BinaryOp::Sub => Instr::Sub,
        BinaryOp::Mul => Instr::Mul,
        BinaryOp::Div => Instr::Div,
        BinaryOp::Rem => Instr::Rem,
    }
}
