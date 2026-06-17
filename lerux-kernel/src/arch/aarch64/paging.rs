//! aarch64 paging glue: programs the MAIR (memory attribute) register.
/// Initialize MAIR
#[cold]
pub unsafe fn init() {
    unsafe {
        crate::rmm::aarch64::init_mair();
    }
}
