//! Stand-in for Ladybird `Libraries/LibJS`.
//!
//! Source text is a sequence of Unicode scalar values. [`run`] lexes that
//! text, parses it with recursive descent and precedence climbing, compiles
//! the abstract syntax tree to bytecode, and interprets the bytecode on a
//! stack machine with a mark-and-sweep heap.
//!
//! The language is literals, arithmetic, `let`, `function`, `if`, and calls.
//! Functions are ordinary objects. A syntax error is returned to the caller.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod ast;
mod compile;
mod error;
mod heap;
mod lex;
mod opcode;
mod parse;
mod value;
mod vm;

pub use error::Error;
pub use value::Value;

/// Lex, parse, compile, and run `source`.
///
/// # Errors
///
/// Returns [`Error`] when `source` is outside the language subset or the run
/// fails. Guest script does not panic.
pub fn run(source: &str) -> Result<Value, Error> {
    let tokens = lex::tokenize(source)?;
    let body = parse::script(&tokens)?;
    let chunks = compile::program(&body)?;
    vm::execute(&chunks)
}
