//! Fallback (no-op) MADT handling for architectures without specific support.
//!
//! Compiled on targets that do not (yet) act on MADT entries, so the shared MADT
//! parser still links. See the x86/aarch64 versions for real implementations.

use super::Madt;

pub(super) fn init(madt: Madt) {
    for madt_entry in madt.iter() {
        debug!("      {:#x?}", madt_entry);
    }

    warn!("MADT not yet handled on this platform");
}
