//! Thin wrapper over the `cpuid` instruction.
//!
//! `cpuid` is how x86 software asks the CPU what it is and what features it
//! supports. This module wraps the inlined `raw_cpuid` crate to give the rest of
//! the kernel convenient feature queries (used during boot to decide which code
//! paths and CPU features to enable).
//!
//! See also: [`docs/kernel/architecture.md`] section 3.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::raw_cpuid::{CpuId, CpuIdResult, ExtendedFeatures, FeatureInfo};

pub fn cpuid() -> CpuId {
    // FIXME check for cpuid availability during early boot and error out if it doesn't exist.
    CpuId::with_cpuid_fn(|a, c| {
        #[cfg(target_arch = "x86")]
        let result = core::arch::x86::__cpuid_count(a, c);
        #[cfg(target_arch = "x86_64")]
        let result = core::arch::x86_64::__cpuid_count(a, c);
        CpuIdResult {
            eax: result.eax,
            ebx: result.ebx,
            ecx: result.ecx,
            edx: result.edx,
        }
    })
}

#[cfg_attr(not(target_arch = "x86_64"), expect(dead_code))]
pub fn feature_info() -> FeatureInfo {
    cpuid()
        .get_feature_info()
        .expect("x86_64 requires CPUID leaf=0x01 to be present")
}

#[cfg_attr(not(target_arch = "x86_64"), expect(dead_code))]
pub fn has_ext_feat(feat: impl FnOnce(ExtendedFeatures) -> bool) -> bool {
    cpuid().get_extended_feature_info().is_some_and(feat)
}
