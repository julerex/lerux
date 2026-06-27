//! NASM assembly checks for SMP trampoline sources (standalone crate; no kernel build).
//! Sizes match the binaries produced by build.rs via nasm.

const X86_64_LEN: usize = 202;
const X86_LEN: usize = 175;

#[test]
fn x86_64_trampoline_asm_exists() {
    let asm = include_str!("../../../src/asm/x86_64/trampoline.asm");
    assert!(asm.contains("ORG 0x8000"));
    assert!(asm.contains("trampoline:"));
}

#[test]
fn x86_trampoline_asm_exists() {
    let asm = include_str!("../../../src/asm/x86/trampoline.asm");
    assert!(asm.contains("ORG 0x8000"));
    assert!(asm.contains("trampoline:"));
}

#[test]
fn trampoline_sizes_documented() {
    // Keep in sync with compare_trampoline_bytes.py EXPECTED_SIZES and build.rs output.
    assert_eq!(X86_64_LEN, 202);
    assert_eq!(X86_LEN, 175);
}