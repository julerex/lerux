//! Handlers for inter-processor interrupts received by this CPU.
//!
//! The counterpart to [`crate::arch::ipi`] (which *sends* IPIs): these run when
//! another CPU pokes this one — to wake it, force a reschedule, or flush stale
//! TLB entries after a mapping change.
//!
//! See also: [`docs/kernel/architecture.md`] section 9.
//!
//! [`docs/kernel/architecture.md`]: ../../../../../docs/kernel/architecture.md

use crate::{
    arch::device::local_apic::the_local_apic, context, percpu::PercpuBlock, sync::CleanLockToken,
};

interrupt!(wakeup, || {
    unsafe { the_local_apic().eoi() };
});

interrupt!(tlb, || {
    PercpuBlock::current().maybe_handle_tlb_shootdown();

    unsafe { the_local_apic().eoi() };
});

interrupt!(switch, || {
    unsafe { the_local_apic().eoi() };

    let mut token = unsafe { CleanLockToken::new() };
    let _ = context::switch(&mut token);
});

interrupt!(pit, || {
    unsafe { the_local_apic().eoi() };

    // Switch after a sufficient amount of time since the last switch.
    let mut token = unsafe { CleanLockToken::new() };
    context::switch::tick(&mut token);
});
