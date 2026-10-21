//! Context switch for one kernel thread.
//!
//! The register set and the first-entry stack match `libco` in the libmicrokitco
//! pin LionsOS 0.4.0 records (`4bf88ee`). Callee-saved registers are stored in a
//! caller-provided buffer. The root buffer is static because the root cothread
//! already has the kernel stack.

use crate::Exclusive;

static ACTIVE: Exclusive<*mut u8> = Exclusive::new(core::ptr::null_mut());

/// Install the root context and return the handle `co_switch` saves into.
pub fn install_root() -> *mut u8 {
    let handle = root_handle();
    // SAFETY: one kernel thread. `handle` points at the static root buffer.
    unsafe { *ACTIVE.get() = handle };
    handle
}

/// Switch to `to`. Returns when that context switches back here.
///
/// # Safety
/// `to` is a handle from [`install_root`] or [`derive`], and the destination
/// stack is live and exclusively owned by that context.
pub unsafe fn co_switch(to: *mut u8) {
    if to.is_null() {
        panic!("lerux-cothread: switch to a null context");
    }
    let active = ACTIVE.get();
    // SAFETY: `install_root` ran, and this kernel thread is the only writer.
    let from = unsafe { *active };
    if from.is_null() {
        panic!("lerux-cothread: switch before the root context exists");
    }
    unsafe { *active = to };
    // SAFETY: caller upholds the handle invariant. The assembly preserves the
    // callee-saved registers across a round trip, matching the platform ABI.
    unsafe { lerux_co_swap(to, from) };
}

/// Plant a first-entry frame in `memory` and return the context handle.
///
/// # Safety
/// `memory` has `size` writable bytes, `size >= 0x1000`, and it is 16-byte aligned.
pub unsafe fn derive(memory: *mut u8, size: usize) -> *mut u8 {
    // SAFETY: caller guarantees the range. The stack was zeroed by spawn.
    unsafe { plant(memory, size) }
}

#[cfg(target_arch = "x86_64")]
const ROOT_WORDS: usize = 8;

#[cfg(target_arch = "x86_64")]
static ROOT: Exclusive<[u64; ROOT_WORDS]> = Exclusive::new([0; ROOT_WORDS]);

#[cfg(target_arch = "x86_64")]
fn root_handle() -> *mut u8 {
    // SAFETY: the root buffer outlives the process, and one kernel thread uses it.
    unsafe { (*ROOT.get()).as_mut_ptr().cast() }
}

#[cfg(target_arch = "x86_64")]
unsafe fn plant(memory: *mut u8, size: usize) -> *mut u8 {
    // System V: the first pop in the swap loads the entry address. Leave the
    // stack 8 mod 16 so that entry's `call` presents the ABI alignment.
    let offset = (size & !15) - 32;
    let mut slot = unsafe { memory.add(offset).cast::<u64>() };
    slot = unsafe { slot.sub(1) };
    unsafe { slot.write(0) };
    slot = unsafe { slot.sub(1) };
    unsafe { slot.write(lerux_co_entry as *const () as u64) };
    unsafe { memory.cast::<u64>().write(slot as u64) };
    memory
}

#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    fn lerux_co_swap(to: *mut u8, from: *mut u8);
    fn lerux_co_entry();
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global lerux_co_swap",
    ".type lerux_co_swap, @function",
    "lerux_co_swap:",
    "mov qword ptr [rsi], rsp",
    "mov rsp, qword ptr [rdi]",
    "pop rax",
    "mov qword ptr [rsi + 8], rbp",
    "mov qword ptr [rsi + 16], rbx",
    "mov qword ptr [rsi + 24], r12",
    "mov qword ptr [rsi + 32], r13",
    "mov qword ptr [rsi + 40], r14",
    "mov qword ptr [rsi + 48], r15",
    "mov rbp, qword ptr [rdi + 8]",
    "mov rbx, qword ptr [rdi + 16]",
    "mov r12, qword ptr [rdi + 24]",
    "mov r13, qword ptr [rdi + 32]",
    "mov r14, qword ptr [rdi + 40]",
    "mov r15, qword ptr [rdi + 48]",
    "jmp rax",
);

// Entered by `jmp`, not `call`, with the return slot already popped. `sub rsp, 8`
// makes the following `call` match the System V entry alignment.
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global lerux_co_entry",
    ".type lerux_co_entry, @function",
    "lerux_co_entry:",
    "sub rsp, 8",
    "call lerux_cothread_started",
    "ud2",
);

#[cfg(target_arch = "aarch64")]
const REGS: usize = 21;
#[cfg(target_arch = "aarch64")]
const LR: usize = 0;
#[cfg(target_arch = "aarch64")]
const SP: usize = 1;
#[cfg(target_arch = "aarch64")]
const FP: usize = 2;

#[cfg(target_arch = "aarch64")]
static ROOT: Exclusive<[u64; REGS]> = Exclusive::new([0; REGS]);

#[cfg(target_arch = "aarch64")]
fn root_handle() -> *mut u8 {
    // The handle is the top word. Saves use negative offsets back into the buffer.
    // SAFETY: the root buffer outlives the process, and one kernel thread uses it.
    unsafe { (*ROOT.get()).as_mut_ptr().add(REGS - 1).cast() }
}

#[cfg(target_arch = "aarch64")]
unsafe fn plant(memory: *mut u8, size: usize) -> *mut u8 {
    let nwords = size / 8;
    let bottom = memory.cast::<u64>();
    // SAFETY: `size` is at least a page, so the context words sit inside it.
    let top = unsafe { bottom.add(nwords - 1) };
    let unaligned = unsafe { bottom.add(nwords - REGS - 1) };
    let aligned = (unaligned as usize) & !0xF;
    unsafe {
        top.sub(SP).write(aligned as u64);
        top.sub(LR).write(lerux_co_entry as *const () as u64);
        top.sub(FP).write(aligned as u64);
    }
    top.cast()
}

#[cfg(target_arch = "aarch64")]
unsafe extern "C" {
    fn lerux_co_swap(to: *mut u8, from: *mut u8);
    fn lerux_co_entry();
}

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".global lerux_co_swap",
    ".type lerux_co_swap, @function",
    "lerux_co_swap:",
    "stp d15, d14, [x1, #-160]",
    "stp d13, d12, [x1, #-144]",
    "stp d11, d10, [x1, #-128]",
    "stp d9, d8, [x1, #-112]",
    "stp x28, x27, [x1, #-96]",
    "stp x26, x25, [x1, #-80]",
    "stp x24, x23, [x1, #-64]",
    "stp x22, x21, [x1, #-48]",
    "stp x20, x19, [x1, #-32]",
    "mov x16, sp",
    "stp x29, x16, [x1, #-16]",
    "str x30, [x1]",
    "ldp d15, d14, [x0, #-160]",
    "ldp d13, d12, [x0, #-144]",
    "ldp d11, d10, [x0, #-128]",
    "ldp d9, d8, [x0, #-112]",
    "ldp x28, x27, [x0, #-96]",
    "ldp x26, x25, [x0, #-80]",
    "ldp x24, x23, [x0, #-64]",
    "ldp x22, x21, [x0, #-48]",
    "ldp x20, x19, [x0, #-32]",
    "ldp x29, x16, [x0, #-16]",
    "mov sp, x16",
    "ldr x30, [x0]",
    "br x30",
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".global lerux_co_entry",
    ".type lerux_co_entry, @function",
    "lerux_co_entry:",
    "bl lerux_cothread_started",
    "brk #1",
);

#[cfg(all(target_arch = "riscv64", not(target_feature = "d")))]
const REGS: usize = 16;
#[cfg(all(target_arch = "riscv64", not(target_feature = "d")))]
const PC: usize = 1;
#[cfg(all(target_arch = "riscv64", not(target_feature = "d")))]
const FP: usize = 13;
#[cfg(all(target_arch = "riscv64", not(target_feature = "d")))]
const SP: usize = 14;
#[cfg(all(target_arch = "riscv64", not(target_feature = "d")))]
const RA: usize = 15;

#[cfg(all(target_arch = "riscv64", target_feature = "d"))]
const REGS: usize = 28;
#[cfg(all(target_arch = "riscv64", target_feature = "d"))]
const PC: usize = 1;
#[cfg(all(target_arch = "riscv64", target_feature = "d"))]
const FP: usize = 25;
#[cfg(all(target_arch = "riscv64", target_feature = "d"))]
const SP: usize = 26;
#[cfg(all(target_arch = "riscv64", target_feature = "d"))]
const RA: usize = 27;

#[cfg(target_arch = "riscv64")]
static ROOT: Exclusive<[u64; REGS]> = Exclusive::new([0; REGS]);

#[cfg(target_arch = "riscv64")]
fn root_handle() -> *mut u8 {
    // SAFETY: the root buffer outlives the process, and one kernel thread uses it.
    unsafe { (*ROOT.get()).as_mut_ptr().add(REGS - 1).cast() }
}

#[cfg(target_arch = "riscv64")]
unsafe fn plant(memory: *mut u8, size: usize) -> *mut u8 {
    let nwords = size / 8;
    let bottom = memory.cast::<u64>();
    // SAFETY: `size` is at least a page, so the context words sit inside it.
    let top = unsafe { bottom.add(nwords - 1) };
    let unaligned = unsafe { bottom.add(nwords - REGS - 1) };
    let aligned = (unaligned as usize) & !0xF;
    unsafe {
        top.sub(RA).write(0);
        top.sub(SP).write(aligned as u64);
        top.sub(FP).write(aligned as u64);
        top.sub(PC).write(lerux_co_entry as *const () as u64);
    }
    top.cast()
}

#[cfg(target_arch = "riscv64")]
unsafe extern "C" {
    fn lerux_co_swap(to: *mut u8, from: *mut u8);
    fn lerux_co_entry();
}

#[cfg(all(target_arch = "riscv64", not(target_feature = "d")))]
core::arch::global_asm!(
    ".global lerux_co_swap",
    ".type lerux_co_swap, @function",
    "lerux_co_swap:",
    "addi a1, a1, -120",
    "sd ra, 0(a1)",
    "sd sp, 8(a1)",
    "sd s0, 16(a1)",
    "sd s1, 24(a1)",
    "sd s2, 32(a1)",
    "sd s3, 40(a1)",
    "sd s4, 48(a1)",
    "sd s5, 56(a1)",
    "sd s6, 64(a1)",
    "sd s7, 72(a1)",
    "sd s8, 80(a1)",
    "sd s9, 88(a1)",
    "sd s10, 96(a1)",
    "sd s11, 104(a1)",
    "sd ra, 112(a1)",
    "addi a0, a0, -120",
    "ld ra, 0(a0)",
    "ld sp, 8(a0)",
    "ld s0, 16(a0)",
    "ld s1, 24(a0)",
    "ld s2, 32(a0)",
    "ld s3, 40(a0)",
    "ld s4, 48(a0)",
    "ld s5, 56(a0)",
    "ld s6, 64(a0)",
    "ld s7, 72(a0)",
    "ld s8, 80(a0)",
    "ld s9, 88(a0)",
    "ld s10, 96(a0)",
    "ld s11, 104(a0)",
    "ld a2, 112(a0)",
    "jr a2",
);

#[cfg(all(target_arch = "riscv64", target_feature = "d"))]
core::arch::global_asm!(
    ".option push",
    ".option arch, +d",
    ".global lerux_co_swap",
    ".type lerux_co_swap, @function",
    "lerux_co_swap:",
    "addi a1, a1, -216",
    "sd ra, 0(a1)",
    "sd sp, 8(a1)",
    "sd s0, 16(a1)",
    "sd s1, 24(a1)",
    "sd s2, 32(a1)",
    "sd s3, 40(a1)",
    "sd s4, 48(a1)",
    "sd s5, 56(a1)",
    "sd s6, 64(a1)",
    "sd s7, 72(a1)",
    "sd s8, 80(a1)",
    "sd s9, 88(a1)",
    "sd s10, 96(a1)",
    "sd s11, 104(a1)",
    "fsd f8, 112(a1)",
    "fsd f9, 120(a1)",
    "fsd f18, 128(a1)",
    "fsd f19, 136(a1)",
    "fsd f20, 144(a1)",
    "fsd f21, 152(a1)",
    "fsd f22, 160(a1)",
    "fsd f23, 168(a1)",
    "fsd f24, 176(a1)",
    "fsd f25, 184(a1)",
    "fsd f26, 192(a1)",
    "fsd f27, 200(a1)",
    "sd ra, 208(a1)",
    "addi a0, a0, -216",
    "ld ra, 0(a0)",
    "ld sp, 8(a0)",
    "ld s0, 16(a0)",
    "ld s1, 24(a0)",
    "ld s2, 32(a0)",
    "ld s3, 40(a0)",
    "ld s4, 48(a0)",
    "ld s5, 56(a0)",
    "ld s6, 64(a0)",
    "ld s7, 72(a0)",
    "ld s8, 80(a0)",
    "ld s9, 88(a0)",
    "ld s10, 96(a0)",
    "ld s11, 104(a0)",
    "fld f8, 112(a0)",
    "fld f9, 120(a0)",
    "fld f18, 128(a0)",
    "fld f19, 136(a0)",
    "fld f20, 144(a0)",
    "fld f21, 152(a0)",
    "fld f22, 160(a0)",
    "fld f23, 168(a0)",
    "fld f24, 176(a0)",
    "fld f25, 184(a0)",
    "fld f26, 192(a0)",
    "fld f27, 200(a0)",
    "ld a2, 208(a0)",
    "jr a2",
    ".option pop",
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".global lerux_co_entry",
    ".type lerux_co_entry, @function",
    "lerux_co_entry:",
    "call lerux_cothread_started",
    "ebreak",
);

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
compile_error!("lerux-cothread context switch is implemented for aarch64, riscv64, and x86_64");
