//! Direct-boot support for lightweight kernel testing in QEMU.
//!
//! When the `direct-boot` Cargo feature is enabled, the kernel synthesizes
//! its own minimal `KernelArgs` instead of relying on a real bootloader.
//! This allows booting with `qemu-system-x86_64 -kernel ...` without the
//! full Redox bootloader or initfs.
//!
//! This is intended for fast kernel development and bring-up testing only.

use super::memory::{BootloaderMemoryEntry, BootloaderMemoryKind};
use super::KernelArgs;

/// Minimal environment for direct boot.
const ENV: &[u8] = b"direct-boot=1\0";

/// Placeholder bootstrap payload (unused in direct-boot mode).
const BOOTSTRAP: &[u8] = b"DIRECT-BOOT";

/// Static memory map for a typical QEMU machine (512 MiB–2 GiB of RAM).
static DIRECT_MEMORY_MAP: [BootloaderMemoryEntry; 4] = [
    // Low memory (BIOS, VGA, etc.)
    BootloaderMemoryEntry {
        base: 0,
        size: 0x100000,
        kind: BootloaderMemoryKind::Reserved,
    },
    // Main usable RAM starting at 1 MiB (~480 MiB free)
    BootloaderMemoryEntry {
        base: 0x0010_0000,
        size: 0x1E0_00000,
        kind: BootloaderMemoryKind::Free,
    },
    // Typical device/MMIO hole
    BootloaderMemoryEntry {
        base: 0xE000_0000,
        size: 0x2000_0000,
        kind: BootloaderMemoryKind::Reserved,
    },
    // High memory placeholder (zero size = ignored)
    BootloaderMemoryEntry {
        base: 0x1_0000_0000,
        size: 0,
        kind: BootloaderMemoryKind::Free,
    },
];

static mut DIRECT_ARGS: Option<KernelArgs> = None;
static mut ENV_STORAGE: [u8; 32] = [0; 32];
static mut BOOTSTRAP_STORAGE: [u8; 64] = [0; 64];
static mut AREAS_STORAGE: [BootloaderMemoryEntry; 4] = DIRECT_MEMORY_MAP;

/// Returns a synthesized KernelArgs for direct QEMU `-kernel` boot.
pub fn get_direct_boot_args() -> &'static KernelArgs {
    unsafe {
        if DIRECT_ARGS.is_none() {
            ENV_STORAGE[..ENV.len()].copy_from_slice(ENV);
            BOOTSTRAP_STORAGE[..BOOTSTRAP.len()].copy_from_slice(BOOTSTRAP);
            AREAS_STORAGE = DIRECT_MEMORY_MAP;

            DIRECT_ARGS = Some(KernelArgs {
                kernel_base: 0,
                kernel_size: 0,
                stack_base: 0,
                stack_size: 0,
                env_base: ENV_STORAGE.as_ptr() as u64,
                env_size: ENV.len() as u64,
                hwdesc_base: 0,
                hwdesc_size: 0,
                areas_base: AREAS_STORAGE.as_ptr() as u64,
                areas_size: core::mem::size_of_val(&AREAS_STORAGE) as u64,
                bootstrap_base: BOOTSTRAP_STORAGE.as_ptr() as u64,
                bootstrap_size: BOOTSTRAP.len() as u64,
            });
        }
        DIRECT_ARGS.as_ref().unwrap()
    }
}
