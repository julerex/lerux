//! Inter-processor interrupts (IPIs): one CPU poking another.
//!
//! Sometimes a CPU needs to make another CPU do something *now* — wake it to run
//! a newly-runnable context, force it to flush stale TLB entries after a mapping
//! change ("TLB shootdown"), or trigger a reschedule. An **IPI** is the hardware
//! mechanism for that: a CPU sends an interrupt to one or more other CPUs via
//! the APIC. [`IpiKind`] enumerates the reasons, and [`IpiTarget`] selects the
//! recipients.
//!
//! See also: [`docs/kernel/architecture.md`] section 9 ("SMP").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

/// Why an inter-processor interrupt is being sent.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum IpiKind {
    Wakeup = 0x40,
    Tlb = 0x41,
    Switch = 0x42,
    Pit = 0x43,

    #[cfg(feature = "profiling")]
    Profile = 0x44,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum IpiTarget {
    Current = 1,
    All = 2,
    Other = 3,
}

#[inline(always)]
pub fn ipi(kind: IpiKind, target: IpiTarget) {
    use crate::arch::device::local_apic::the_local_apic;

    if cfg!(not(feature = "multi_core")) {
        return;
    }

    #[cfg(feature = "profiling")]
    if matches!(kind, IpiKind::Profile) {
        let icr = ((target as u64) << 18) | (1 << 14) | (0b100 << 8);
        unsafe { the_local_apic().set_icr(icr) };
        return;
    }

    let icr = ((target as u64) << 18) | (1 << 14) | (kind as u64);
    unsafe { the_local_apic().set_icr(icr) };
}

#[inline(always)]
pub fn ipi_single(kind: IpiKind, target: &crate::percpu::PercpuBlock) {
    use crate::arch::device::local_apic::the_local_apic;

    if cfg!(not(feature = "multi_core")) {
        return;
    }

    if let Some(apic_id) = target.misc_arch_info.apic_id_opt.get() {
        unsafe {
            the_local_apic().ipi(apic_id, kind);
        }
    }
}
