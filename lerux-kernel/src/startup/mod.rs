//! Early kernel initialization: the handoff from the bootloader to `kmain`.
//!
//! This module owns the moment the kernel takes control of the machine. The
//! architecture-specific entry code (for example
//! [`arch::x86_shared::start`](crate::arch)) does the very first setup and then
//! calls [`kmain`], which finishes bringing the kernel up and never returns —
//! it ends in the scheduler loop ([`run_userspace`]).
//!
//! ## What you should already understand
//!
//! - A **bootloader** is the program that runs before the kernel; it loads the
//!   kernel image into memory and describes the machine to it.
//! - That description arrives as a [`KernelArgs`] struct: where RAM is, where the
//!   initial userspace image (the *initfs*) lives, and where firmware tables
//!   (ACPI/RSDP on PCs, a device tree blob on ARM/RISC-V) can be found.
//! - **initfs / bootstrap**: the first userspace program the kernel runs. On a
//!   normal boot the kernel maps it and jumps into it; everything else
//!   (drivers, shell) is started from there.
//!
//! ## lerux divergence: direct-boot
//!
//! Upstream Redox always boots through a real bootloader. lerux adds a
//! `direct-boot` mode (see the `direct_boot` submodule) that synthesizes
//! [`KernelArgs`] so
//! the kernel can run under `qemu-system-x86_64 -kernel ...` with no bootloader,
//! for fast kernel development. In that mode `kmain` can skip spawning userspace
//! entirely (kernel-only smoke testing) unless the `direct-boot-userspace`
//! feature is enabled.
//!
//! See also: [`docs/kernel/architecture.md`] section 3 ("Boot: from CPU reset to
//! the scheduler").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use core::{
    hint,
    ptr::NonNull,
    slice,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    arch::interrupt,
    context,
    context::switch::SwitchResult,
    memory::{PhysicalAddress, RmmA, RmmArch},
    profiling, scheme,
    sync::CleanLockToken,
};

/// Parsing of the bootloader-supplied physical memory map.
pub mod memory;

/// lerux-only: synthetic [`KernelArgs`] for QEMU `-kernel` direct-boot (upstream
/// always receives boot info from an external bootloader). See `direct_boot.rs`
/// and root `VENDORED.md`.
#[cfg(feature = "direct-boot")]
pub mod direct_boot;

/// Everything the bootloader tells the kernel about the machine.
///
/// This is the boot contract between the loader and the kernel. The layout is
/// fixed (`#[repr(C, packed(8))]`) because the bootloader writes these bytes and
/// the kernel reads them; the two are compiled separately and must agree on the
/// exact field order and offsets. All addresses are **physical** (raw RAM
/// addresses), which is why the accessor methods convert them to virtual
/// addresses (via [`RmmA::phys_to_virt`]) before dereferencing.
///
/// Each `*_base`/`*_size` pair describes one region of physical memory:
/// the kernel image itself, its boot stack, the environment string block, the
/// hardware-description table, the memory-map array, and the bootstrap/initfs.
#[repr(C, packed(8))]
pub(crate) struct KernelArgs {
    /// Physical base and byte length of the loaded kernel image.
    kernel_base: u64,
    kernel_size: u64,

    /// Physical base and byte length of the initial kernel stack.
    stack_base: u64,
    stack_size: u64,

    /// Physical base and byte length of the boot environment string
    /// (`key=value` lines passed from the bootloader).
    env_base: u64,
    env_size: u64,

    /// The base pointer to the saved RSDP or device tree blob.
    ///
    /// On x86 this field can be NULL, and if so, the system has not booted
    /// with UEFI or in some other way retrieved the RSDPs. The kernel or a
    /// userspace driver will thus try searching the BIOS memory instead. On
    /// UEFI systems, searching is not guaranteed to actually work though.
    /// On other architectures this field must always contain a pointer to
    /// either an RSDP or device tree blob.
    pub(crate) hwdesc_base: u64,
    pub(crate) hwdesc_size: u64,

    /// Physical base and byte length of the bootloader's memory-map array
    /// (the list of [`memory::BootloaderMemoryEntry`] regions).
    areas_base: u64,
    areas_size: u64,

    /// The physical base 64-bit pointer to the contiguous bootstrap/initfs.
    bootstrap_base: u64,
    /// Size of contiguous bootstrap/initfs physical region, not necessarily page aligned.
    bootstrap_size: u64,
}

impl KernelArgs {
    /// Log every region described by these args to the serial console.
    ///
    /// Useful when bringing up a new machine or debugging the boot handoff: it
    /// confirms the kernel and bootloader agree on where everything lives.
    pub(crate) fn print(&self) {
        // lerux direct-boot: KernelArgs dump at info! (serial); upstream uses debug!
        // because graphical_debug is available from the bootloader framebuffer.
        macro_rules! argline {
            ($($t:tt)*) => {{
                #[cfg(feature = "direct-boot")]
                info!($($t)*);
                #[cfg(not(feature = "direct-boot"))]
                debug!($($t)*);
            }};
        }
        argline!(
            "Kernel: {:X}:{:X}",
            { self.kernel_base },
            self.kernel_base + self.kernel_size
        );
        argline!(
            "Env: {:X}:{:X}",
            { self.env_base },
            self.env_base + self.env_size
        );
        argline!(
            "HWDESC: {:X}:{:X}",
            { self.hwdesc_base },
            self.hwdesc_base + self.hwdesc_size
        );
        argline!(
            "Areas: {:X}:{:X}",
            { self.areas_base },
            self.areas_base + self.areas_size
        );
        argline!(
            "Bootstrap: {:X}:{:X}",
            { self.bootstrap_base },
            self.bootstrap_base + self.bootstrap_size
        );
    }

    /// Build the [`Bootstrap`] descriptor (initfs location + environment) that
    /// `kmain` later hands to the first userspace process.
    pub(crate) fn bootstrap(&self) -> Bootstrap {
        Bootstrap {
            base: crate::memory::Frame::containing(crate::memory::PhysicalAddress::new(
                self.bootstrap_base as usize,
            )),
            page_count: (self.bootstrap_size as usize).div_ceil(crate::memory::PAGE_SIZE),
            env: self.env(),
        }
    }

    /// Borrow the boot environment block as raw bytes.
    ///
    /// # Safety / correctness
    ///
    /// Reads physical memory the bootloader promised is valid for `env_size`
    /// bytes; the returned slice is `'static` because the boot environment lives
    /// for the lifetime of the kernel.
    pub(crate) fn env(&self) -> &'static [u8] {
        unsafe {
            slice::from_raw_parts(
                RmmA::phys_to_virt(PhysicalAddress::new(self.env_base as usize)).data()
                    as *const u8,
                self.env_size as usize,
            )
        }
    }

    /// Return a pointer to the ACPI RSDP if the bootloader provided one.
    ///
    /// The **RSDP** (Root System Description Pointer) is the entry point to the
    /// ACPI firmware tables on PCs; finding it lets the kernel discover CPUs,
    /// interrupt controllers, and other hardware. We validate the `"RSD PTR "`
    /// signature so we never hand ACPI code a device tree (or garbage) by
    /// mistake.
    pub(crate) fn acpi_rsdp(&self) -> Option<NonNull<u8>> {
        if self.hwdesc_base != 0 {
            let data = unsafe {
                slice::from_raw_parts(
                    RmmA::phys_to_virt(PhysicalAddress::new(self.hwdesc_base as usize)).data()
                        as *const u8,
                    self.hwdesc_size as usize,
                )
            };
            if data.starts_with(b"RSD PTR ") {
                Some(NonNull::from_ref(data).cast())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Return the parsed device tree blob if the bootloader provided one.
    ///
    /// A **device tree blob (DTB)** is the ARM/RISC-V equivalent of ACPI: a
    /// firmware-provided description of the hardware. Only one of [`acpi_rsdp`]
    /// or [`dtb`] is meaningful on a given machine.
    ///
    /// [`acpi_rsdp`]: KernelArgs::acpi_rsdp
    /// [`dtb`]: KernelArgs::dtb
    pub(crate) fn dtb(&self) -> Option<crate::fdt::Fdt<'static>> {
        if self.hwdesc_base != 0 {
            let data = unsafe {
                slice::from_raw_parts(
                    RmmA::phys_to_virt(PhysicalAddress::new(self.hwdesc_base as usize)).data()
                        as *const u8,
                    self.hwdesc_size as usize,
                )
            };
            crate::fdt::Fdt::new(data).ok()
        } else {
            None
        }
    }
}

/// Borrow the boot environment string captured at startup.
pub(crate) fn init_env() -> &'static [u8] {
    BOOTSTRAP.get().expect("BOOTSTRAP was not set").env
}

/// Entry point of the very first userspace context.
///
/// The scheduler runs this on a fresh kernel stack; it jumps into the initfs
/// bootstrap binary, which becomes PID 1 and starts the rest of the system.
extern "C" fn userspace_init() {
    let mut token = unsafe { CleanLockToken::new() };
    let bootstrap = BOOTSTRAP.get().expect("BOOTSTRAP was not set");
    unsafe { crate::syscall::process::usermode_bootstrap(bootstrap, &mut token) }
}

/// Where the first userspace program lives, plus its environment.
pub(crate) struct Bootstrap {
    /// First physical frame of the contiguous initfs image.
    pub(crate) base: crate::memory::Frame,
    /// Number of page-sized frames the initfs image spans.
    pub(crate) page_count: usize,
    /// Boot environment passed through to the first process.
    env: &'static [u8],
}

/// Set once by the BSP in [`kmain`]; read by [`userspace_init`].
static BOOTSTRAP: crate::spin::Once<Bootstrap> = crate::spin::Once::new();
/// Set by an application processor once it has finished early init; the BSP
/// waits on this when starting each AP so startup is serialized.
pub(crate) static AP_READY: AtomicBool = AtomicBool::new(false);
/// Set by the BSP once core init is done; APs spin on this before proceeding.
static BSP_READY: AtomicBool = AtomicBool::new(false);

/// Kernel entry point for the **primary CPU** (the bootstrap processor, BSP).
///
/// The architecture-specific startup code calls this after it has set up paging
/// and the heap. It finishes global initialization (contexts, schemes), spawns
/// the first userspace process (unless direct-boot is skipping it), and then
/// enters the scheduler loop, which never returns.
pub(crate) fn kmain(bootstrap: Bootstrap) -> ! {
    let mut token = unsafe { CleanLockToken::new() };

    BSP_READY.store(true, Ordering::SeqCst);

    //Initialize the first context, stored in kernel/src/context/mod.rs
    context::init(&mut token);

    //Initialize global schemes, such as `acpi:`.
    scheme::init_globals();

    debug!("BSP: {} CPUs", crate::cpu_count());
    debug!("Env: {:?}", ::core::str::from_utf8(bootstrap.env));

    BOOTSTRAP.call_once(|| bootstrap);

    profiling::ready_for_profiling();

    if cfg!(feature = "direct-boot") && !cfg!(feature = "direct-boot-userspace") {
        // lerux divergence: upstream always spawns userspace_init here; direct-boot
        // skips bootstrap for kernel-only smoke testing unless direct-boot-userspace is set.
        info!("direct-boot mode: skipping userspace bootstrap for kernel-only testing");
    } else {
        let owner = None; // kmain not owned by any fd
        match context::spawn(true, owner, userspace_init, &mut token) {
            Ok(context_lock) => {
                let mut context = context_lock.write(token.token());
                context.status = context::Status::Runnable;
                context.name.clear();
                context.name.push_str("[bootstrap]");

                // TODO: Remove these from kernel
                context.euid = 0;
                context.egid = 0;
            }
            Err(err) => {
                panic!("failed to spawn userspace_init: {:?}", err);
            }
        }
    }

    run_userspace(&mut token)
}

/// Kernel entry point for the **secondary CPUs** (application processors, APs).
///
/// Each AP is started by the trampoline and ends up here. It waits for the BSP
/// to signal readiness, does its own per-CPU context init, and then joins the
/// shared scheduler loop. If the kernel was built without `multi_core`, the AP
/// simply halts forever.
#[allow(unreachable_code, unused_variables, dead_code)]
pub(crate) fn kmain_ap(cpu_id: crate::cpu_set::LogicalCpuId) -> ! {
    let mut token = unsafe { CleanLockToken::new() };

    AP_READY.store(true, Ordering::SeqCst);
    while !BSP_READY.load(Ordering::SeqCst) {
        hint::spin_loop();
    }

    profiling::maybe_run_profiling_helper_forever(cpu_id);

    if !cfg!(feature = "multi_core") {
        debug!("AP {}: Disabled", cpu_id);

        loop {
            unsafe {
                interrupt::disable();
                interrupt::halt();
            }
        }
    }

    context::init(&mut token);

    debug!("AP {}", cpu_id);

    profiling::ready_for_profiling();

    run_userspace(&mut token);
}

/// The scheduler loop that every CPU runs forever.
///
/// Each iteration: disable interrupts (so the switch is atomic), ask the
/// scheduler to switch to the next runnable context, then re-enable interrupts.
/// When there is nothing to run, the CPU halts until the next interrupt instead
/// of busy-spinning, which saves power. Interrupts must be disabled around the
/// switch because a timer interrupt mid-switch could corrupt the saved state.
fn run_userspace(token: &mut CleanLockToken) -> ! {
    loop {
        unsafe {
            interrupt::disable();
            match context::switch(token) {
                SwitchResult::Switched => {
                    interrupt::enable_and_nop();
                }
                SwitchResult::AllContextsIdle => {
                    // Enable interrupts, then halt CPU (to save power) until the next interrupt is actually fired.
                    interrupt::enable_and_halt();
                }
            }
        }
    }
}
