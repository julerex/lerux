//! Miscellaneous per-CPU x86_64 feature setup.
//!
//! Enables assorted optional CPU features during boot (via control registers
//! like CR4) — things such as SSE/AVX state, user-mode instruction prevention,
//! and other security/performance bits. Run once per CPU early in startup.
//!
//! See also: [`docs/kernel/architecture.md`] section 3.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::x86::controlregs::Cr4;

use crate::{
    arch::cpuid::{cpuid, has_ext_feat},
    cpu_set::LogicalCpuId,
};

pub unsafe fn init(cpu_id: LogicalCpuId) {
    unsafe {
        if has_ext_feat(|feat| feat.has_umip()) {
            // UMIP (UserMode Instruction Prevention) forbids userspace from calling SGDT, SIDT, SLDT,
            // SMSW and STR. KASLR is currently not implemented, but this protects against leaking
            // addresses.
            crate::x86::controlregs::cr4_write(
                crate::x86::controlregs::cr4() | Cr4::CR4_ENABLE_UMIP,
            );
        }
        if has_ext_feat(|feat| feat.has_smep()) {
            // SMEP (Supervisor-Mode Execution Prevention) forbids the kernel from executing
            // instruction on any page marked "userspace-accessible". This improves security for
            // obvious reasons.
            crate::x86::controlregs::cr4_write(
                crate::x86::controlregs::cr4() | Cr4::CR4_ENABLE_SMEP,
            );
        }

        if let Some(feats) = cpuid().get_extended_processor_and_feature_identifiers()
            && feats.has_rdtscp()
        {
            crate::x86::msr::wrmsr(crate::x86::msr::IA32_TSC_AUX, cpu_id.get().into());
        }
    }
}
