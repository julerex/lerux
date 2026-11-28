pub(crate) mod opcode;
pub(crate) mod vm;
pub(crate) mod heap;
pub(crate) mod value;
pub(crate) mod compile;
pub(crate) use vm::execute;
