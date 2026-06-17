/// Initialize MAIR
#[cold]
pub unsafe fn init() {
    unsafe {
        crate::rmm::aarch64::init_mair();
    }
}
