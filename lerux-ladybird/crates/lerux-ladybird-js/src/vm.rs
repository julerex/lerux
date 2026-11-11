//! Stack machine.
//!
//! Arithmetic pops and pushes the operand stack. Bindings live in environment
//! objects on the mark-and-sweep heap. A call allocates a fresh environment.

use alloc::vec::Vec;

use crate::{
    error::Error,
    heap::{Data, Heap},
    opcode::{Chunk, Const, Instr, Upvalue},
    value::Value,
};

const MAX_FRAMES: usize = 64;
const MAX_STACK: usize = 1024;
const MAX_STEPS: u32 = 1_000_000;

pub(crate) fn execute(chunks: &[Chunk]) -> Result<Value, Error> {
    let mut vm = Vm::load(chunks)?;
    vm.start()?;
    loop {
        match vm.step()? {
            Control::Continue => {}
            Control::Done(data) => return vm.export(data),
        }
    }
}

enum Control {
    Continue,
    Done(Data),
}

struct Frame {
    chunk: u16,
    ip: usize,
    env: u32,
    base: usize,
}

#[derive(Clone, Copy)]
enum RtConst {
    Number(f64),
    Boolean(bool),
    String(u32),
    Function(u16),
}

struct Func {
    arity: u16,
    locals: u16,
    code: Vec<Instr>,
    constants: Vec<RtConst>,
    upvalues: Vec<Upvalue>,
}

struct Vm {
    funcs: Vec<Func>,
    heap: Heap,
    stack: Vec<Data>,
    frames: Vec<Frame>,
    steps: u32,
}

impl Vm {
    fn load(chunks: &[Chunk]) -> Result<Self, Error> {
        let mut heap = Heap::new();
        let mut funcs = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            funcs.push(load_func(chunk, &mut heap)?);
        }
        Ok(Self {
            funcs,
            heap,
            stack: Vec::new(),
            frames: Vec::new(),
            steps: 0,
        })
    }

    fn start(&mut self) -> Result<(), Error> {
        let locals = self.signature(0)?.1;
        self.maybe_gc();
        let env = self.heap.alloc_env(locals, None)?;
        self.frames.push(Frame {
            chunk: 0,
            ip: 0,
            env,
            base: 0,
        });
        Ok(())
    }

    fn step(&mut self) -> Result<Control, Error> {
        self.steps = self.steps.checked_add(1).ok_or(Error::runtime())?;
        if self.steps > MAX_STEPS {
            return Err(Error::runtime());
        }
        let Some(instr) = self.next_instr()? else {
            return self.return_undefined();
        };
        self.dispatch(instr)
    }

    fn dispatch(&mut self, instr: Instr) -> Result<Control, Error> {
        match instr {
            Instr::Const(index) => self.op_const(index)?,
            Instr::Undefined => self.push(Data::Undefined)?,
            Instr::Load(slot) => self.op_load(slot)?,
            Instr::Store(slot) => self.op_store(slot)?,
            Instr::LoadUp(index) => self.op_load_up(index)?,
            Instr::Pop => {
                self.pop()?;
            }
            Instr::Add => self.arith(add)?,
            Instr::Sub => self.arith(sub)?,
            Instr::Mul => self.arith(mul)?,
            Instr::Div => self.arith(div)?,
            Instr::Rem => self.arith(rem)?,
            Instr::Neg => self.unary(neg)?,
            Instr::Pos => self.unary(pos)?,
            Instr::Jump(target) => self.jump(target)?,
            Instr::JumpIfFalse(target) => self.jump_if_false(target)?,
            Instr::Call(argc) => self.op_call(argc)?,
            Instr::Return => return self.op_return(),
        }
        Ok(Control::Continue)
    }

    fn op_const(&mut self, index: u16) -> Result<(), Error> {
        match self.constant(index)? {
            RtConst::Number(number) => self.push(Data::Number(number)),
            RtConst::Boolean(bit) => self.push(Data::Boolean(bit)),
            RtConst::String(text) => self.push(Data::String(text)),
            RtConst::Function(chunk) => self.push_function(chunk),
        }
    }

    fn push_function(&mut self, chunk: u16) -> Result<(), Error> {
        let env = self.current_env()?;
        self.maybe_gc();
        let object = self.heap.alloc_function(chunk, env)?;
        self.push(Data::Object(object))
    }

    fn op_load(&mut self, slot: u16) -> Result<(), Error> {
        let env = self.current_env()?;
        let value = self.heap.slot(env, slot).ok_or(Error::runtime())?;
        self.push(value)
    }

    fn op_store(&mut self, slot: u16) -> Result<(), Error> {
        let value = self.pop()?;
        let env = self.current_env()?;
        self.heap.set_slot(env, slot, value)
    }

    fn op_load_up(&mut self, index: u8) -> Result<(), Error> {
        let up = self.upvalue(index)?;
        let mut env = self.current_env()?;
        for _ in 0..up.hops {
            env = self.heap.parent(env).ok_or(Error::runtime())?;
        }
        let value = self.heap.slot(env, up.slot).ok_or(Error::runtime())?;
        self.push(value)
    }

    fn op_call(&mut self, argc: u8) -> Result<(), Error> {
        if self.frames.len() >= MAX_FRAMES {
            return Err(Error::runtime());
        }
        let argc = usize::from(argc);
        let callee_at = self.callee_at(argc)?;
        let callee = self.stack_at(callee_at)?;
        let (chunk, parent) = self.function_of(callee)?;
        let (arity, locals) = self.signature(chunk)?;
        self.maybe_gc();
        let env = self.heap.alloc_env(locals, Some(parent))?;
        self.copy_args(env, callee_at, argc, arity)?;
        self.stack.truncate(callee_at);
        let base = self.stack.len();
        self.frames.push(Frame {
            chunk,
            ip: 0,
            env,
            base,
        });
        Ok(())
    }

    fn copy_args(
        &mut self,
        env: u32,
        callee_at: usize,
        argc: usize,
        arity: u16,
    ) -> Result<(), Error> {
        let count = argc.min(usize::from(arity));
        for offset in 0..count {
            let value = self.stack_at(callee_at + 1 + offset)?;
            let slot = u16::try_from(offset).map_err(|_| Error::runtime())?;
            self.heap.set_slot(env, slot, value)?;
        }
        Ok(())
    }

    fn op_return(&mut self) -> Result<Control, Error> {
        let value = self.pop()?;
        let frame = self.frames.pop().ok_or(Error::runtime())?;
        self.stack.truncate(frame.base);
        if self.frames.is_empty() {
            return Ok(Control::Done(value));
        }
        self.push(value)?;
        Ok(Control::Continue)
    }

    fn return_undefined(&mut self) -> Result<Control, Error> {
        self.push(Data::Undefined)?;
        self.op_return()
    }

    fn jump_if_false(&mut self, target: u16) -> Result<(), Error> {
        let value = self.pop()?;
        if !self.truthy(value) {
            self.jump(target)?;
        }
        Ok(())
    }

    fn jump(&mut self, target: u16) -> Result<(), Error> {
        let target = usize::from(target);
        let chunk = self.current_chunk()?;
        let len = self
            .funcs
            .get(usize::from(chunk))
            .ok_or(Error::runtime())?
            .code
            .len();
        if target > len {
            return Err(Error::runtime());
        }
        self.frames.last_mut().ok_or(Error::runtime())?.ip = target;
        Ok(())
    }

    fn arith(&mut self, op: fn(f64, f64) -> f64) -> Result<(), Error> {
        let right = self.pop_number()?;
        let left = self.pop_number()?;
        self.push(Data::Number(op(left, right)))
    }

    fn unary(&mut self, op: fn(f64) -> f64) -> Result<(), Error> {
        let number = self.pop_number()?;
        self.push(Data::Number(op(number)))
    }

    fn truthy(&self, value: Data) -> bool {
        match value {
            Data::Undefined => false,
            Data::Boolean(bit) => bit,
            Data::Number(number) => number_is_true(number),
            Data::String(index) => self.heap.string_nonempty(index),
            Data::Object(_) => true,
        }
    }

    fn maybe_gc(&mut self) {
        if !self.heap.needs_gc() {
            return;
        }
        let values = self.root_values();
        let envs = self.root_envs();
        self.heap.collect(&values, &envs);
    }

    fn root_values(&self) -> Vec<Data> {
        let mut values = self.stack.clone();
        for func in &self.funcs {
            for constant in &func.constants {
                if let RtConst::String(index) = constant {
                    values.push(Data::String(*index));
                }
            }
        }
        values
    }

    fn root_envs(&self) -> Vec<u32> {
        self.frames.iter().map(|frame| frame.env).collect()
    }

    fn export(&self, data: Data) -> Result<Value, Error> {
        match data {
            Data::Undefined => Ok(Value::Undefined),
            Data::Boolean(bit) => Ok(Value::Boolean(bit)),
            Data::Number(number) => Ok(Value::Number(number)),
            Data::String(index) => {
                let text = self.heap.string(index).ok_or(Error::runtime())?;
                Ok(Value::String(text.into()))
            }
            Data::Object(_) => Ok(Value::Object),
        }
    }

    fn next_instr(&mut self) -> Result<Option<Instr>, Error> {
        let chunk = self.current_chunk()?;
        let ip = self.frames.last().ok_or(Error::runtime())?.ip;
        let instr = self
            .funcs
            .get(usize::from(chunk))
            .ok_or(Error::runtime())?
            .code
            .get(ip)
            .copied();
        if instr.is_some() {
            self.frames.last_mut().ok_or(Error::runtime())?.ip = ip + 1;
        }
        Ok(instr)
    }

    fn constant(&self, index: u16) -> Result<RtConst, Error> {
        let chunk = self.current_chunk()?;
        self.funcs
            .get(usize::from(chunk))
            .ok_or(Error::runtime())?
            .constants
            .get(usize::from(index))
            .copied()
            .ok_or(Error::runtime())
    }

    fn upvalue(&self, index: u8) -> Result<Upvalue, Error> {
        let chunk = self.current_chunk()?;
        self.funcs
            .get(usize::from(chunk))
            .ok_or(Error::runtime())?
            .upvalues
            .get(usize::from(index))
            .copied()
            .ok_or(Error::runtime())
    }

    fn signature(&self, chunk: u16) -> Result<(u16, u16), Error> {
        let func = self.funcs.get(usize::from(chunk)).ok_or(Error::runtime())?;
        Ok((func.arity, func.locals))
    }

    fn function_of(&self, value: Data) -> Result<(u16, u32), Error> {
        let Data::Object(index) = value else {
            return Err(Error::runtime());
        };
        self.heap.function(index).ok_or(Error::runtime())
    }

    fn current_chunk(&self) -> Result<u16, Error> {
        self.frames
            .last()
            .map(|frame| frame.chunk)
            .ok_or(Error::runtime())
    }

    fn current_env(&self) -> Result<u32, Error> {
        self.frames
            .last()
            .map(|frame| frame.env)
            .ok_or(Error::runtime())
    }

    fn callee_at(&self, argc: usize) -> Result<usize, Error> {
        let base = self.frames.last().ok_or(Error::runtime())?.base;
        let index = self
            .stack
            .len()
            .checked_sub(argc + 1)
            .ok_or(Error::runtime())?;
        if index < base {
            return Err(Error::runtime());
        }
        Ok(index)
    }

    fn stack_at(&self, index: usize) -> Result<Data, Error> {
        self.stack.get(index).copied().ok_or(Error::runtime())
    }

    fn pop_number(&mut self) -> Result<f64, Error> {
        match self.pop()? {
            Data::Number(number) => Ok(number),
            Data::Undefined | Data::Boolean(_) | Data::String(_) | Data::Object(_) => {
                Err(Error::runtime())
            }
        }
    }

    fn pop(&mut self) -> Result<Data, Error> {
        let base = self.frames.last().ok_or(Error::runtime())?.base;
        if self.stack.len() <= base {
            return Err(Error::runtime());
        }
        self.stack.pop().ok_or(Error::runtime())
    }

    fn push(&mut self, value: Data) -> Result<(), Error> {
        if self.stack.len() >= MAX_STACK {
            return Err(Error::runtime());
        }
        self.stack.push(value);
        Ok(())
    }
}

fn load_func(chunk: &Chunk, heap: &mut Heap) -> Result<Func, Error> {
    let mut constants = Vec::with_capacity(chunk.constants.len());
    for constant in &chunk.constants {
        constants.push(load_const(constant, heap)?);
    }
    Ok(Func {
        arity: chunk.arity,
        locals: chunk.locals,
        code: chunk.code.clone(),
        constants,
        upvalues: chunk.upvalues.clone(),
    })
}

fn load_const(constant: &Const, heap: &mut Heap) -> Result<RtConst, Error> {
    match constant {
        Const::Number(number) => Ok(RtConst::Number(*number)),
        Const::Boolean(bit) => Ok(RtConst::Boolean(*bit)),
        Const::String(text) => Ok(RtConst::String(heap.alloc_string(text.clone())?)),
        Const::Function(index) => Ok(RtConst::Function(*index)),
    }
}

fn number_is_true(number: f64) -> bool {
    !number.is_nan() && number.to_bits() != 0 && number.to_bits() != (-0.0_f64).to_bits()
}

fn add(left: f64, right: f64) -> f64 {
    left + right
}

fn sub(left: f64, right: f64) -> f64 {
    left - right
}

fn mul(left: f64, right: f64) -> f64 {
    left * right
}

fn div(left: f64, right: f64) -> f64 {
    left / right
}

fn rem(left: f64, right: f64) -> f64 {
    left % right
}

fn neg(number: f64) -> f64 {
    -number
}

fn pos(number: f64) -> f64 {
    number
}
