use core::mem;
use syscall::{data::Map, flag::MapFlags, number::SYS_FMAP};

const STACK_SIZE: usize = 64 * 1024;
pub const USERMODE_END: usize = 0x0000_8000_0000_0000;
pub const STACK_START: usize = USERMODE_END - syscall::KERNEL_METADATA_SIZE - STACK_SIZE;

static MAP: Map = Map {
    offset: 0,
    size: STACK_SIZE,
    flags: MapFlags::PROT_READ
        .union(MapFlags::PROT_WRITE)
        .union(MapFlags::MAP_PRIVATE)
        .union(MapFlags::MAP_FIXED_NOREPLACE),
    address: STACK_START,
};

core::arch::global_asm!(
    "
    .globl _start
    _start:
    mov rcx, rsp
    mov rax, {stack_start}
    cmp rcx, rax
    jae 2f
    mov rax, {number}
    mov rdi, {fd}
    mov rsi, offset {map}
    mov rdx, {map_size}
    syscall
    cmp rax, 0
    jg 1f
    ud2
    1:
    lea rcx, [rax + {stack_size} - 16]
    2:
    mov rsp, rcx
    mov rbp, rsp
    call lerux_entry_start
    ud2
    ",
    fd = const usize::MAX,
    map = sym MAP,
    map_size = const mem::size_of::<Map>(),
    number = const SYS_FMAP,
    stack_size = const STACK_SIZE,
    stack_start = const STACK_START,
);
