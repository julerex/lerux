//! The 32-bit x86 port.
//!
//! Re-exports the shared x86 machinery from [`x86_shared`](crate::arch) and adds
//! the 32-bit-only pieces (memory-map constants, interrupt/syscall entry). lerux
//! targets x86_64 primarily; this port exists for completeness and is less
//! exercised.
//!
//! See also: [`docs/kernel/architecture.md`] sections 3, 8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

pub use crate::arch::x86_shared::*;

/// Constants like memory locations
pub mod consts;

/// Interrupt instructions
#[macro_use]
pub mod interrupt;

#[unsafe(naked)]
pub unsafe extern "C" fn arch_copy_to_user(dst: usize, src: usize, len: usize) -> u8 {
    core::arch::naked_asm!(
        "
    .global __usercopy_start
    __usercopy_start:
        push edi
        push esi

        mov edi, [esp + 12] # dst
        mov esi, [esp + 16] # src
        mov ecx, [esp + 20] # len
        rep movsb

        pop esi
        pop edi

        xor eax, eax
        ret
    .global __usercopy_end
    __usercopy_end:
    "
    );
}
pub use arch_copy_to_user as arch_copy_from_user;

pub const KFX_SIZE: usize = 512;

// This function exists as the KFX size is dynamic on x86_64.
pub fn kfx_size() -> usize {
    KFX_SIZE
}
