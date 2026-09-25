//! Closed Wasm subset emitted by `support/prog/smoke.rs` at `opt-level=z`.
//!
//! Sections: type, import, function, memory, global, export, code, data.
//! Custom sections are skipped. Any other section or opcode is [`Error::Opcode`]
//! or [`Error::Section`].

use crate::{Error, MEMORY_LEN};

const WASM_MAGIC: &[u8; 4] = b"\0asm";
const MAX_TYPES: usize = 4;
const MAX_GLOBALS: usize = 4;
const MAX_LOCALS: usize = 8;
const MAX_STACK: usize = 32;
const MAX_CTRL: usize = 8;

const END: u8 = 0x0b;
const BLOCK: u8 = 0x02;
const BR_IF: u8 = 0x0d;
const CALL: u8 = 0x10;
const LOCAL_GET: u8 = 0x20;
const LOCAL_SET: u8 = 0x21;
const LOCAL_TEE: u8 = 0x22;
const GLOBAL_GET: u8 = 0x23;
const GLOBAL_SET: u8 = 0x24;
const I32_LOAD: u8 = 0x28;
const I32_STORE: u8 = 0x36;
const I32_CONST: u8 = 0x41;
const I32_NE: u8 = 0x47;
const I32_ADD: u8 = 0x6a;
const I32_SUB: u8 = 0x6b;
const I32: u8 = 0x7f;
const EMPTY_BLOCK: u8 = 0x40;

#[derive(Clone, Copy)]
struct FuncType {
    params: u8,
    results: u8,
}

#[derive(Clone, Copy)]
struct Global {
    mutable: bool,
    init: u32,
}

struct Parsed<'a> {
    globals: [Global; MAX_GLOBALS],
    global_len: usize,
    code: &'a [u8],
    data_off: u32,
    data: &'a [u8],
}

#[derive(Clone, Copy)]
struct Ctrl {
    end: usize,
    base: usize,
}

/// Instantiate `wasm` in `memory` and call the `start` export.
///
/// `memory` must be at least [`MEMORY_LEN`] bytes. `log` receives each
/// `lerux.log` payload. A rejected opcode does not call `log`.
pub fn run(wasm: &[u8], memory: &mut [u8], mut log: impl FnMut(&[u8])) -> Result<(), Error> {
    let parsed = parse(wasm)?;
    if memory.len() < MEMORY_LEN {
        return Err(Error::Memory);
    }
    let memory = &mut memory[..MEMORY_LEN];
    memory.fill(0);
    let data_end = (parsed.data_off as usize)
        .checked_add(parsed.data.len())
        .ok_or(Error::Bounds)?;
    if data_end > memory.len() {
        return Err(Error::Data);
    }
    memory[parsed.data_off as usize..data_end].copy_from_slice(parsed.data);

    let mut globals = [0u32; MAX_GLOBALS];
    for (slot, global) in globals.iter_mut().zip(parsed.globals.iter()) {
        *slot = global.init;
    }
    exec(&parsed, memory, &mut globals, &mut log)
}

fn parse(wasm: &[u8]) -> Result<Parsed<'_>, Error> {
    if wasm.len() < 8 || wasm.get(..4) != Some(WASM_MAGIC.as_slice()) || wasm[4..8] != [1, 0, 0, 0]
    {
        return Err(Error::Section);
    }
    let mut i = 8usize;
    let mut last_id = 0u8;
    let mut types = [FuncType {
        params: 0,
        results: 0,
    }; MAX_TYPES];
    let mut type_len = 0usize;
    let mut import_type = None;
    let mut func_type_idx = None;
    let mut saw_memory = false;
    let mut globals = [Global {
        mutable: false,
        init: 0,
    }; MAX_GLOBALS];
    let mut global_len = 0usize;
    let mut start = None;
    let mut code: Option<&[u8]> = None;
    let mut data_off = 0u32;
    let mut data: &[u8] = &[];
    let mut saw_data = false;

    while i < wasm.len() {
        let id = wasm[i];
        i += 1;
        let (size, next) = read_uleb(wasm, i)?;
        i = next;
        let size = size as usize;
        let end = i.checked_add(size).ok_or(Error::Truncated)?;
        if end > wasm.len() {
            return Err(Error::Truncated);
        }
        let body = &wasm[i..end];
        i = end;
        if id == 0 {
            continue;
        }
        if id <= last_id {
            return Err(Error::Section);
        }
        last_id = id;
        match id {
            1 => type_len = parse_types(body, &mut types)?,
            2 => import_type = Some(parse_import(body, &types, type_len)?),
            3 => func_type_idx = Some(parse_function(body, type_len)?),
            5 => {
                parse_memory(body)?;
                saw_memory = true;
            }
            6 => global_len = parse_globals(body, &mut globals)?,
            7 => start = Some(parse_export(body)?),
            10 => code = Some(parse_code(body)?),
            11 => {
                let (off, bytes) = parse_data(body)?;
                data_off = off;
                data = bytes;
                saw_data = true;
            }
            _ => return Err(Error::Section),
        }
    }

    if import_type.is_none() {
        return Err(Error::Import);
    }
    let Some(func_idx) = func_type_idx else {
        return Err(Error::Function);
    };
    if !saw_memory {
        return Err(Error::Memory);
    }
    if start != Some(1) {
        return Err(Error::Export);
    }
    let Some(code) = code else {
        return Err(Error::Code);
    };
    if !saw_data {
        return Err(Error::Data);
    }
    let func_type = types[func_idx];
    if func_type.params != 0 || func_type.results != 0 {
        return Err(Error::Type);
    }
    Ok(Parsed {
        globals,
        global_len,
        code,
        data_off,
        data,
    })
}

fn parse_types(body: &[u8], types: &mut [FuncType; MAX_TYPES]) -> Result<usize, Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count as usize > MAX_TYPES {
        return Err(Error::Type);
    }
    for slot in types.iter_mut().take(count as usize) {
        if i >= body.len() || body[i] != 0x60 {
            return Err(Error::Type);
        }
        i += 1;
        let (nparams, next) = read_uleb(body, i)?;
        i = next;
        if nparams > 2 {
            return Err(Error::Type);
        }
        for _ in 0..nparams {
            if body.get(i) != Some(&I32) {
                return Err(Error::Type);
            }
            i += 1;
        }
        let (nresults, next) = read_uleb(body, i)?;
        i = next;
        if nresults > 1 {
            return Err(Error::Type);
        }
        for _ in 0..nresults {
            if body.get(i) != Some(&I32) {
                return Err(Error::Type);
            }
            i += 1;
        }
        *slot = FuncType {
            params: nparams as u8,
            results: nresults as u8,
        };
    }
    if i != body.len() {
        return Err(Error::Type);
    }
    Ok(count as usize)
}

fn parse_import(
    body: &[u8],
    types: &[FuncType; MAX_TYPES],
    type_len: usize,
) -> Result<FuncType, Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count != 1 {
        return Err(Error::Import);
    }
    let (module, next) = read_name(body, i)?;
    i = next;
    let (field, next) = read_name(body, i)?;
    i = next;
    if module != b"lerux" || field != b"log" {
        return Err(Error::Import);
    }
    if body.get(i) != Some(&0x00) {
        return Err(Error::Import);
    }
    i += 1;
    let (type_idx, next) = read_uleb(body, i)?;
    i = next;
    if i != body.len() || type_idx as usize >= type_len {
        return Err(Error::Import);
    }
    let ty = types[type_idx as usize];
    if ty.params != 2 || ty.results != 0 {
        return Err(Error::Import);
    }
    Ok(ty)
}

fn parse_function(body: &[u8], type_len: usize) -> Result<usize, Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count != 1 {
        return Err(Error::Function);
    }
    let (type_idx, next) = read_uleb(body, i)?;
    i = next;
    if i != body.len() || type_idx as usize >= type_len {
        return Err(Error::Function);
    }
    Ok(type_idx as usize)
}

fn parse_memory(body: &[u8]) -> Result<(), Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count != 1 || body.get(i) != Some(&0x00) {
        return Err(Error::Memory);
    }
    i += 1;
    let (pages, next) = read_uleb(body, i)?;
    i = next;
    if i != body.len() || pages != 1 {
        return Err(Error::Memory);
    }
    Ok(())
}

fn parse_globals(body: &[u8], globals: &mut [Global; MAX_GLOBALS]) -> Result<usize, Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count as usize > MAX_GLOBALS {
        return Err(Error::Global);
    }
    for slot in globals.iter_mut().take(count as usize) {
        if body.get(i) != Some(&I32) {
            return Err(Error::Global);
        }
        i += 1;
        let mutable = match body.get(i) {
            Some(&0x00) => false,
            Some(&0x01) => true,
            _ => return Err(Error::Global),
        };
        i += 1;
        if body.get(i) != Some(&I32_CONST) {
            return Err(Error::Global);
        }
        i += 1;
        let (value, next) = read_sleb(body, i)?;
        i = next;
        if body.get(i) != Some(&END) {
            return Err(Error::Global);
        }
        i += 1;
        *slot = Global {
            mutable,
            init: value as u32,
        };
    }
    if i != body.len() {
        return Err(Error::Global);
    }
    Ok(count as usize)
}

fn parse_export(body: &[u8]) -> Result<u32, Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    let mut start = None;
    for _ in 0..count {
        let (_, next) = read_name(body, i)?;
        let name_at = i;
        i = next;
        let (name, _) = read_name(body, name_at)?;
        if i >= body.len() {
            return Err(Error::Export);
        }
        let kind = body[i];
        i += 1;
        let (idx, next) = read_uleb(body, i)?;
        i = next;
        if name == b"start" && kind == 0x00 {
            start = Some(idx);
        }
    }
    if i != body.len() {
        return Err(Error::Export);
    }
    start.ok_or(Error::Export)
}

fn parse_code(body: &[u8]) -> Result<&[u8], Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count != 1 {
        return Err(Error::Code);
    }
    let (size, next) = read_uleb(body, i)?;
    i = next;
    let size = size as usize;
    let end = i.checked_add(size).ok_or(Error::Truncated)?;
    if end != body.len() {
        return Err(Error::Code);
    }
    Ok(&body[i..end])
}

fn parse_data(body: &[u8]) -> Result<(u32, &[u8]), Error> {
    let mut i = 0usize;
    let (count, next) = read_uleb(body, i)?;
    i = next;
    if count != 1 || body.get(i) != Some(&0x00) {
        return Err(Error::Data);
    }
    i += 1;
    if body.get(i) != Some(&I32_CONST) {
        return Err(Error::Data);
    }
    i += 1;
    let (off, next) = read_sleb(body, i)?;
    i = next;
    if off < 0 || body.get(i) != Some(&END) {
        return Err(Error::Data);
    }
    i += 1;
    let (len, next) = read_uleb(body, i)?;
    i = next;
    let len = len as usize;
    let end = i.checked_add(len).ok_or(Error::Truncated)?;
    if end != body.len() {
        return Err(Error::Data);
    }
    Ok((off as u32, &body[i..end]))
}

fn exec(
    parsed: &Parsed<'_>,
    memory: &mut [u8],
    globals: &mut [u32],
    log: &mut impl FnMut(&[u8]),
) -> Result<(), Error> {
    let code = parsed.code;
    let mut pc = 0usize;
    let (groups, next) = read_uleb(code, pc)?;
    pc = next;
    let mut nlocals = 0usize;
    for _ in 0..groups {
        let (n, next) = read_uleb(code, pc)?;
        pc = next;
        if code.get(pc) != Some(&I32) {
            return Err(Error::Local);
        }
        pc += 1;
        nlocals = nlocals.checked_add(n as usize).ok_or(Error::Local)?;
    }
    if nlocals > MAX_LOCALS {
        return Err(Error::Local);
    }
    let mut locals = [0u32; MAX_LOCALS];
    let mut stack = [0u32; MAX_STACK];
    let mut sp = 0usize;
    let mut ctrls = [Ctrl { end: 0, base: 0 }; MAX_CTRL];
    let mut csp = 0usize;
    let mutable = {
        let mut flags = [false; MAX_GLOBALS];
        for (flag, global) in flags.iter_mut().zip(parsed.globals.iter()) {
            *flag = global.mutable;
        }
        flags
    };

    while pc < code.len() {
        let op = code[pc];
        pc += 1;
        match op {
            END => {
                if csp == 0 {
                    if sp != 0 {
                        return Err(Error::Stack);
                    }
                    return Ok(());
                }
                csp -= 1;
            }
            BLOCK => {
                if code.get(pc) != Some(&EMPTY_BLOCK) {
                    return Err(Error::Control);
                }
                pc += 1;
                if csp >= MAX_CTRL {
                    return Err(Error::Control);
                }
                let end = find_end(code, pc)?;
                ctrls[csp] = Ctrl { end, base: sp };
                csp += 1;
            }
            BR_IF => {
                let (label, next) = read_uleb(code, pc)?;
                pc = next;
                let cond = pop(&stack, &mut sp)?;
                if cond != 0 {
                    let label = label as usize;
                    if label >= csp {
                        return Err(Error::Control);
                    }
                    let target = csp - 1 - label;
                    csp = target + 1;
                    sp = ctrls[target].base;
                    pc = ctrls[target].end;
                }
            }
            CALL => {
                let (idx, next) = read_uleb(code, pc)?;
                pc = next;
                if idx != 0 {
                    return Err(Error::Call);
                }
                let len = pop(&stack, &mut sp)? as i32;
                let ptr = pop(&stack, &mut sp)? as i32;
                host_log(memory, ptr, len, log)?;
            }
            LOCAL_GET => {
                let (idx, next) = read_uleb(code, pc)?;
                pc = next;
                let idx = idx as usize;
                if idx >= nlocals {
                    return Err(Error::Local);
                }
                push(&mut stack, &mut sp, locals[idx])?;
            }
            LOCAL_SET => {
                let (idx, next) = read_uleb(code, pc)?;
                pc = next;
                let idx = idx as usize;
                if idx >= nlocals {
                    return Err(Error::Local);
                }
                locals[idx] = pop(&stack, &mut sp)?;
            }
            LOCAL_TEE => {
                let (idx, next) = read_uleb(code, pc)?;
                pc = next;
                let idx = idx as usize;
                if idx >= nlocals {
                    return Err(Error::Local);
                }
                let value = pop(&stack, &mut sp)?;
                locals[idx] = value;
                push(&mut stack, &mut sp, value)?;
            }
            GLOBAL_GET => {
                let (idx, next) = read_uleb(code, pc)?;
                pc = next;
                let idx = idx as usize;
                if idx >= parsed.global_len {
                    return Err(Error::Global);
                }
                push(&mut stack, &mut sp, globals[idx])?;
            }
            GLOBAL_SET => {
                let (idx, next) = read_uleb(code, pc)?;
                pc = next;
                let idx = idx as usize;
                if idx >= parsed.global_len || !mutable[idx] {
                    return Err(Error::Global);
                }
                globals[idx] = pop(&stack, &mut sp)?;
            }
            I32_LOAD => {
                let (_, next) = read_uleb(code, pc)?;
                let (offset, next) = read_uleb(code, next)?;
                pc = next;
                let addr = pop(&stack, &mut sp)?;
                let at = mem_addr(addr, offset, 4, memory.len())?;
                let bytes = [memory[at], memory[at + 1], memory[at + 2], memory[at + 3]];
                push(&mut stack, &mut sp, u32::from_le_bytes(bytes))?;
            }
            I32_STORE => {
                let (_, next) = read_uleb(code, pc)?;
                let (offset, next) = read_uleb(code, next)?;
                pc = next;
                let value = pop(&stack, &mut sp)?;
                let addr = pop(&stack, &mut sp)?;
                let at = mem_addr(addr, offset, 4, memory.len())?;
                memory[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
            I32_CONST => {
                let (value, next) = read_sleb(code, pc)?;
                pc = next;
                push(&mut stack, &mut sp, value as u32)?;
            }
            I32_NE => {
                let b = pop(&stack, &mut sp)?;
                let a = pop(&stack, &mut sp)?;
                push(&mut stack, &mut sp, u32::from(a != b))?;
            }
            I32_ADD => {
                let b = pop(&stack, &mut sp)? as i32;
                let a = pop(&stack, &mut sp)? as i32;
                push(&mut stack, &mut sp, a.wrapping_add(b) as u32)?;
            }
            I32_SUB => {
                let b = pop(&stack, &mut sp)? as i32;
                let a = pop(&stack, &mut sp)? as i32;
                push(&mut stack, &mut sp, a.wrapping_sub(b) as u32)?;
            }
            _ => return Err(Error::Opcode),
        }
    }
    Err(Error::Truncated)
}

fn host_log(memory: &[u8], ptr: i32, len: i32, log: &mut impl FnMut(&[u8])) -> Result<(), Error> {
    if ptr < 0 || len < 0 {
        return Err(Error::Bounds);
    }
    let start = ptr as usize;
    let end = start.checked_add(len as usize).ok_or(Error::Bounds)?;
    if end > memory.len() {
        return Err(Error::Bounds);
    }
    log(&memory[start..end]);
    Ok(())
}

fn find_end(code: &[u8], mut pc: usize) -> Result<usize, Error> {
    let mut depth = 1u32;
    while pc < code.len() {
        let op = code[pc];
        if op == END {
            depth -= 1;
            if depth == 0 {
                return Ok(pc);
            }
        } else if op == BLOCK {
            depth += 1;
        }
        pc = skip_op(code, pc)?;
    }
    Err(Error::Control)
}

fn skip_op(code: &[u8], mut pc: usize) -> Result<usize, Error> {
    if pc >= code.len() {
        return Err(Error::Truncated);
    }
    let op = code[pc];
    pc += 1;
    match op {
        END | I32_ADD | I32_SUB | I32_NE => Ok(pc),
        BLOCK => {
            if code.get(pc) != Some(&EMPTY_BLOCK) {
                return Err(Error::Control);
            }
            Ok(pc + 1)
        }
        BR_IF | CALL | LOCAL_GET | LOCAL_SET | LOCAL_TEE | GLOBAL_GET | GLOBAL_SET => {
            let (_, next) = read_uleb(code, pc)?;
            Ok(next)
        }
        I32_LOAD | I32_STORE => {
            let (_, next) = read_uleb(code, pc)?;
            let (_, next) = read_uleb(code, next)?;
            Ok(next)
        }
        I32_CONST => {
            let (_, next) = read_sleb(code, pc)?;
            Ok(next)
        }
        _ => Err(Error::Opcode),
    }
}

fn push(stack: &mut [u32], sp: &mut usize, value: u32) -> Result<(), Error> {
    if *sp >= stack.len() {
        return Err(Error::Stack);
    }
    stack[*sp] = value;
    *sp += 1;
    Ok(())
}

fn pop(stack: &[u32], sp: &mut usize) -> Result<u32, Error> {
    if *sp == 0 {
        return Err(Error::Stack);
    }
    *sp -= 1;
    Ok(stack[*sp])
}

fn mem_addr(addr: u32, offset: u32, size: u32, mem_len: usize) -> Result<usize, Error> {
    let start = (addr as usize)
        .checked_add(offset as usize)
        .ok_or(Error::Bounds)?;
    let end = start.checked_add(size as usize).ok_or(Error::Bounds)?;
    if end > mem_len {
        return Err(Error::Bounds);
    }
    Ok(start)
}

fn read_name(bytes: &[u8], i: usize) -> Result<(&[u8], usize), Error> {
    let (len, next) = read_uleb(bytes, i)?;
    let len = len as usize;
    let end = next.checked_add(len).ok_or(Error::Truncated)?;
    if end > bytes.len() {
        return Err(Error::Truncated);
    }
    Ok((&bytes[next..end], end))
}

/// Unsigned LEB128, at most 5 bytes. Overlong encodings are accepted because
/// `rustc`'s Wasm backend emits them for `call` and `global.get` indices.
fn read_uleb(bytes: &[u8], mut i: usize) -> Result<(u32, usize), Error> {
    let mut result = 0u32;
    let mut shift = 0;
    for _ in 0..5 {
        let byte = *bytes.get(i).ok_or(Error::Truncated)?;
        i += 1;
        if shift == 28 && byte & 0x7f > 0x0f {
            return Err(Error::Truncated);
        }
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i));
        }
        shift += 7;
    }
    Err(Error::Truncated)
}

fn read_sleb(bytes: &[u8], mut i: usize) -> Result<(i32, usize), Error> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut byte;
    for _ in 0..5 {
        byte = *bytes.get(i).ok_or(Error::Truncated)?;
        i += 1;
        if shift == 28 && (byte & 0x7f) > 0x0f && byte & 0x7f != 0x7f {
            return Err(Error::Truncated);
        }
        result |= u32::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < 32 && byte & 0x40 != 0 {
                result |= !0u32 << shift;
            }
            return Ok((result as i32, i));
        }
    }
    Err(Error::Truncated)
}
