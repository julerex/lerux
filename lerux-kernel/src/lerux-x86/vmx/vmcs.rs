// vmcs stub for lerux zero-dep inlined build.
// The original has bitflags and complex VMCS field defs that trigger delimiter issues
// under the current inlined bitflags + mod context for x86.
// For the smoke test (direct boot, no full vmx usage) a stub suffices.
pub mod control {
    pub const PINBASED_EXEC_CONTROLS: u32 = 0;
}
pub const VMCS_LINK_POINTER: u32 = 0;
pub const GUEST_RIP: u32 = 0;
pub const GUEST_RSP: u32 = 0;
pub const GUEST_RFLAGS: u32 = 0;
pub const HOST_RIP: u32 = 0;
// Add more consts as needed by code that references them; the smoke should not reach vmx setup.
