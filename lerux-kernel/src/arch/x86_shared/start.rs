//! x86 / x86_64 boot entry: the first Rust the CPU runs.
//!
//! This is the architecture-specific bridge between "the firmware/bootloader
//! just jumped to us" and "the architecture-independent kernel is running"
//! ([`crate::startup::kmain`]). It is intentionally tiny and extremely unsafe:
//! at this point there is no heap, no logging, and barely any CPU state set up.
//!
//! The flow is:
//!
//! 1. [`kstart`] — a `#[naked]` assembly entry point. It sanity-checks that the
//!    BSS/data sections were laid out correctly, sets up a real stack, and tail
//!    calls [`start`]. (`kstart_ap`/`start_ap` are the equivalents for secondary
//!    CPUs.)
//! 2. [`start`] — the first real Rust function. In a fixed order it brings up
//!    serial output, the **GDT** (segment table) and **IDT** (interrupt table),
//!    physical memory + paging, the kernel heap, devices, and ACPI, then calls
//!    `kmain` (which never returns).
//!
//! The ordering here is load-bearing: each step depends on the previous one
//! (for example, nothing can allocate until `allocator::init()` runs, and
//! interrupts must not fire until the IDT is installed).
//!
//! ## lerux `direct-boot` note
//!
//! When `feature = "direct-boot"` is enabled, `start` ignores the bootloader
//! `KernelArgs` pointer and uses `startup::direct_boot::get_direct_boot_args`
//! instead; graphical debug and ACPI init are skipped (no framebuffer env / RSDP).
//! Upstream Redox always uses bootloader-supplied args. See root `VENDORED.md`.
//!
//! See also: [`docs/kernel/architecture.md`] section 3 ("Boot").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md
use core::{arch::naked_asm, cell::SyncUnsafeCell, mem::offset_of};

use crate::{
    allocator,
    arch::{device, gdt, idt, interrupt, paging},
    cpu_set::LogicalCpuId,
    devices::graphical_debug,
    startup::KernelArgs,
};

/// Test of zero values in BSS.
static BSS_TEST_ZERO: SyncUnsafeCell<usize> = SyncUnsafeCell::new(0);
/// Test of non-zero values in data.
static DATA_TEST_NONZERO: SyncUnsafeCell<usize> = SyncUnsafeCell::new(usize::MAX);

#[repr(C, align(16))]
struct StackAlign<T>(T);

static STACK: SyncUnsafeCell<StackAlign<[u8; 128 * 1024]>> =
    SyncUnsafeCell::new(StackAlign([0; 128 * 1024]));

/// The raw entry symbol the bootloader (or PVH stub) jumps to.
///
/// Written as naked assembly because at entry there is no valid stack to spill
/// registers to. It verifies the linker zeroed BSS and preserved initialized
/// data (a cheap guard against a broken kernel image), installs the static
/// boot [`STACK`], then jumps to [`start`] with the `KernelArgs` pointer intact.
// FIXME use extern "custom"
#[unsafe(naked)]
#[unsafe(no_mangle)]
extern "C" fn kstart() {
    naked_asm!(
        #[cfg(target_arch = "x86")]
        "
        // BSS should already be zero
        cmp dword ptr [{bss_test_zero}], 0
        jne .Lkstart_crash
        cmp dword ptr [{data_test_nonzero}], 0
        je .Lkstart_crash

        mov eax, [esp + 4]
        lea esp, [{stack}+{stack_size}-16]
        mov [esp + 4], eax
        mov [esp + 8], esp

        jmp {start}

    .Lkstart_crash:
        xor eax, eax
        jmp eax
    ",

        #[cfg(target_arch = "x86_64")]
        "
        // BSS should already be zero
        cmp qword ptr [rip + {bss_test_zero}], 0
        jne .Lkstart_crash
        cmp qword ptr [rip + {data_test_nonzero}], 0
        je .Lkstart_crash

        // Note: The System V ABI requires the stack to be aligned to 16 bytes
        // before the call instruction. As we jump rather than call it has to
        // be offset by 8 bytes. Additionally reserve a bit more space at the
        // end of the stack to ensure that the start function returns to
        // address 0.
        lea rsp, [rip + {stack}+{stack_size}-24]
        mov rsi, rsp

        jmp {start}

    .Lkstart_crash:
        xor rax, rax
        jmp rax
    ",

        bss_test_zero = sym BSS_TEST_ZERO,
        data_test_nonzero = sym DATA_TEST_NONZERO,
        stack = sym STACK,
        stack_size = const size_of_val(&STACK),
        start = sym start,
    );
}

/// First real Rust function on the boot CPU; initializes everything in order.
///
/// Each line of the body is a boot step that must happen before the next:
/// serial → GDT/IDT → physical memory → paging → syscall entry → heap →
/// logging → devices → ACPI → `kmain`. After this, the kernel is a normal
/// running system. Never returns (ends in the scheduler loop via `kmain`).
///
/// # Safety
///
/// Runs once, on a single CPU, with interrupts effectively disabled. `args_ptr`
/// must point at a valid [`KernelArgs`] (except in direct-boot, where it is
/// ignored in favor of synthesized args).
/// The entry to Rust, all things must be initialized
unsafe extern "C" fn start(args_ptr: *const KernelArgs, stack_end: usize) -> ! {
    unsafe {
        let bootstrap = {
            #[cfg(feature = "direct-boot")]
            let args = {
                let _ = args_ptr;
                crate::startup::direct_boot::get_direct_boot_args()
            };

            #[cfg(not(feature = "direct-boot"))]
            let args = args_ptr.read();

            // Set up serial debug
            device::serial::init();

            // Set up graphical debug (needs bootloader framebuffer env; skip in direct-boot)
            #[cfg(not(feature = "direct-boot"))]
            graphical_debug::init(args.env());

            info!("Redox OS starting...");
            args.print();

            // Set up GDT
            gdt::init_bsp(stack_end);

            // Set up IDT
            idt::init_bsp();

            // Initialize RMM
            #[cfg(target_arch = "x86")]
            crate::startup::memory::init(&args, Some(0x100000), Some(0x40000000));
            #[cfg(target_arch = "x86_64")]
            {
                #[cfg(feature = "direct-boot")]
                crate::startup::memory::init(&args, Some(0x100000), None);
                #[cfg(not(feature = "direct-boot"))]
                crate::startup::memory::init(&args, Some(0x100000), None);
            }

            // Initialize paging
            paging::init();

            #[cfg(target_arch = "x86_64")]
            crate::arch::alternative::early_init(true);

            // Set up syscall instruction
            interrupt::syscall::init();

            // Setup kernel heap
            allocator::init();

            // Activate memory logging
            crate::log::init();

            // Initialize miscellaneous processor features
            #[cfg(target_arch = "x86_64")]
            crate::arch::misc::init(LogicalCpuId::BSP);

            // Initialize devices
            device::init();

            // Read ACPI tables, starts APs (no RSDP in lerux direct-boot; upstream
            // expects bootloader-provided RSDP or BIOS search via normal boot path)
            if cfg!(all(feature = "acpi", not(feature = "direct-boot"))) {
                crate::acpi::init(args.acpi_rsdp());
                device::init_after_acpi();
            }
            crate::profiling::init();

            // Initialize all of the non-core devices not otherwise needed to complete initialization
            device::init_noncore();

            args.bootstrap()
        };

        crate::startup::kmain(bootstrap);
    }
}

/// Per-CPU startup arguments for an application processor (AP).
///
/// The BSP fills one of these in for each secondary CPU before kicking it off
/// via the trampoline. It tells the AP where its stack is, its logical CPU id,
/// and which GDT/IDT structures to load.
pub struct KernelArgsAp {
    /// Top of the stack the AP should run on.
    pub stack_end: *mut u8,
    /// Logical id assigned to this CPU by the kernel.
    pub cpu_id: LogicalCpuId,
    /// This CPU's processor-control region (per-CPU GDT data).
    pub pcr_ptr: *mut gdt::ProcessorControlRegion,
    /// Interrupt descriptor table this CPU should install.
    pub idt_ptr: *mut idt::Idt,
}

/// Naked entry point for a secondary CPU, analogous to [`kstart`].
///
/// The SMP trampoline jumps here once an AP is in 64-bit mode. It loads the
/// AP's stack from its [`KernelArgsAp`] and tail calls [`start_ap`].
// FIXME use extern "custom"
#[unsafe(naked)]
pub extern "C" fn kstart_ap() {
    naked_asm!(
        #[cfg(target_arch = "x86")]
        "
        mov esp, dword ptr [edi + {args_stack}]
        mov [esp + 4], edi
        mov [esp + 8], esp

        jmp {start_ap}
    ",

        #[cfg(target_arch = "x86_64")]
        "
        // Note: The System V ABI requires the stack to be aligned to 16 bytes
        // before the call instruction. As we jump rather than call it has to
        // be offset by 8 bytes. Additionally reserve a bit more space at the
        // end of the stack to ensure that the start function returns to
        // address 0.
        mov rax, qword ptr [rdi + {args_stack}]
        lea rsp, [rax - 24]

        jmp {start_ap}
    ",

        args_stack = const offset_of!(KernelArgsAp, stack_end),
        start_ap = sym start_ap,
    );
}

/// First real Rust function on a secondary CPU; the AP counterpart of [`start`].
///
/// Does the per-CPU subset of boot setup (install this CPU's GDT/IDT, enable
/// paging and the syscall instruction, init local devices) and then joins the
/// shared scheduler via [`crate::startup::kmain_ap`]. Never returns.
///
/// # Safety
///
/// `args_ptr` must point to a valid [`KernelArgsAp`] prepared by the BSP for
/// exactly this CPU.
/// Entry to rust for an AP
unsafe extern "C" fn start_ap(args_ptr: *const KernelArgsAp) -> ! {
    unsafe {
        let cpu_id = {
            let args = &*args_ptr;

            // Set up GDT
            gdt::install_pcr(args.pcr_ptr);

            // Set up IDT
            idt::install_idt(args.idt_ptr);

            // Initialize paging
            paging::init();

            crate::profiling::init();

            #[cfg(target_arch = "x86_64")]
            crate::arch::alternative::early_init(false);

            // Set up syscall instruction
            interrupt::syscall::init();

            // Initialize miscellaneous processor features
            #[cfg(target_arch = "x86_64")]
            crate::arch::misc::init(args.cpu_id);

            // Initialize devices (for AP)
            device::init_ap();

            args.cpu_id
        };

        crate::startup::kmain_ap(cpu_id);
    }
}
