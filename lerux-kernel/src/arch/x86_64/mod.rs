//! The 64-bit x86 (`x86_64`) port — lerux's primary architecture.
//!
//! Re-exports the shared x86 machinery from [`x86_shared`](crate::arch) and adds
//! the 64-bit-only pieces: long-mode constants ([`consts`]), the 64-bit
//! interrupt/syscall entry paths ([`interrupt`]), processor feature setup
//! ([`misc`]), and the `alternative` mechanism for patching code based on
//! detected CPU features.
//!
//! See also: [`docs/kernel/architecture.md`] sections 3, 8, 9.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

pub use crate::arch::x86_shared::*;

pub mod alternative;

#[macro_use]
pub mod macros;

/// Constants like memory locations
pub mod consts;

/// Interrupt instructions
#[macro_use]
pub mod interrupt;

/// Miscellaneous processor features
pub mod misc;

// TODO: Maybe support rewriting relocations (using LD's --emit-relocs) when working with entire
// functions?
#[unsafe(naked)]
pub unsafe extern "C" fn arch_copy_to_user(dst: usize, src: usize, len: usize) -> u8 {
    // TODO: spectre_v1

    core::arch::naked_asm!(
        ".global __usercopy_start
        __usercopy_start:",
        alternative!(
            feature: "smap",
            then: ["
            xor eax, eax
            mov rcx, rdx
            stac
            rep movsb
            clac
            ret
        "],
            default: ["
            xor eax, eax
            mov rcx, rdx
            rep movsb
            ret
        "]
        ),
        ".global __usercopy_end
        __usercopy_end:"
    );
}
pub use arch_copy_to_user as arch_copy_from_user;

pub use alternative::kfx_size;
