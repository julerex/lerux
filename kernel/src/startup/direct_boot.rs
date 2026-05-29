//! Direct-boot support for lightweight kernel testing in QEMU.
//!
//! When the `direct-boot` Cargo feature is enabled, the kernel synthesizes
//! its own minimal `KernelArgs` instead of relying on a real bootloader.
//! This allows booting with `qemu-system-x86_64 -kernel ...` without the
//! full Redox bootloader or initfs.
//!
//! This is intended for fast kernel development and bring-up testing only.

use super::{
    memory::{BootloaderMemoryEntry, BootloaderMemoryKind},
    KernelArgs,
};

/// Must match [linkers/x86_64-direct.ld](linkers/x86_64-direct.ld) `KERNEL_PHYS_BASE`.
const KERNEL_LOAD_PHYS: usize = 0x200_000;

#[inline]
fn virt_to_phys(virt: usize) -> u64 {
    virt.wrapping_sub(crate::kernel_executable_offsets::KERNEL_OFFSET())
        .wrapping_add(KERNEL_LOAD_PHYS) as u64
}

/// Minimal environment for direct boot.
const ENV: &[u8] = b"direct-boot=1\0";

/// Placeholder bootstrap payload (unused in direct-boot mode).
const BOOTSTRAP: &[u8] = b"DIRECT-BOOT";

/// Static memory map for a typical QEMU machine (512 MiB–2 GiB of RAM).
static DIRECT_MEMORY_MAP: [BootloaderMemoryEntry; 6] = [
    // Low memory (BIOS, VGA, etc.)
    BootloaderMemoryEntry {
        base: 0,
        size: 0x100000,
        kind: BootloaderMemoryKind::Reserved,
    },
    // PVH note, boot stub, and bootstrap page tables
    BootloaderMemoryEntry {
        base: 0x100000,
        size: 0x100000,
        kind: BootloaderMemoryKind::Reserved,
    },
    // Kernel image at 2 MiB (must match KERNEL_LOAD_PHYS)
    BootloaderMemoryEntry {
        base: KERNEL_LOAD_PHYS as u64,
        size: 0x0100_0000,
        kind: BootloaderMemoryKind::Kernel,
    },
    // Main usable RAM after the kernel image (trimmed for 512 MiB guests)
    BootloaderMemoryEntry {
        base: 0x0120_0000,
        size: 0x0CD0_0000,
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
static mut AREAS_STORAGE: [BootloaderMemoryEntry; 6] = DIRECT_MEMORY_MAP;

/// Returns a synthesized KernelArgs for direct QEMU `-kernel` boot.
pub fn get_direct_boot_args() -> &'static KernelArgs {
    unsafe {
        if DIRECT_ARGS.is_none() {
            ENV_STORAGE[..ENV.len()].copy_from_slice(ENV);
            BOOTSTRAP_STORAGE[..BOOTSTRAP.len()].copy_from_slice(BOOTSTRAP);
            AREAS_STORAGE = DIRECT_MEMORY_MAP;

            DIRECT_ARGS = Some(KernelArgs {
                // Kernel extent is covered by the static memory map + linker layout.
                kernel_base: 0,
                kernel_size: 0,
                stack_base: 0,
                stack_size: 0,
                env_base: virt_to_phys(ENV_STORAGE.as_ptr() as usize),
                env_size: ENV.len() as u64,
                hwdesc_base: 0,
                hwdesc_size: 0,
                areas_base: virt_to_phys(AREAS_STORAGE.as_ptr() as usize),
                areas_size: core::mem::size_of_val(&AREAS_STORAGE) as u64,
                // Bootstrap is unused in direct-boot; skip IdentityMap registration.
                bootstrap_base: 0,
                bootstrap_size: 0,
            });
        }
        DIRECT_ARGS.as_ref().unwrap()
    }
}

// Retain the PVH boot stub + ELF note under --gc-sections (QEMU reads them, not Rust).
#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    fn pvh_start32();
}

#[cfg(target_arch = "x86_64")]
#[used]
static _KEEP_PVH_BOOT: unsafe extern "C" fn() = pvh_start32;
