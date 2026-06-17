//! riscv64 paging glue (the x86 `paging.rs` analog).
#[cold]
pub unsafe fn init() {
    // Assuming SBI already set up PMAs correctly for us
    // TODO: detect Svpbmt present/enabled and override device memory with PBMT=IO
}
