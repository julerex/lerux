//! Architecture paging glue (x86 / x86_64).
//!
//! Thin bridge between the kernel's generic memory code and the inlined `rmm`
//! crate's x86 paging implementation. [`init`] programs the **PAT** (Page
//! Attribute Table), the CPU feature that decides how each page's memory is
//! cached. The heavy lifting of reading and writing page tables lives in `rmm`;
//! this file just exposes the per-arch entry points the kernel calls during
//! boot.
//!
//! See also: [`docs/kernel/architecture.md`] section 4 ("Memory model").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

/// Initialize PAT
#[cold]
pub unsafe fn init() {
    unsafe {
        #[cfg(target_arch = "x86")]
        crate::rmm::x86::init_pat();
        #[cfg(target_arch = "x86_64")]
        crate::rmm::x86_64::init_pat();
    }
}
