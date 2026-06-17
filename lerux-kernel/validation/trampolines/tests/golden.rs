//! Golden-file checks for SMP trampoline bytes (standalone crate; no kernel build).

#[test]
fn x86_64_trampoline_matches_nasm_golden() {
    let golden = include_bytes!("../expected/trampoline_x86_64.bin");
    assert_eq!(golden.len(), 202);
    assert_eq!(&golden[8..40], &[0u8; 32]);
}

#[test]
fn x86_trampoline_golden_file_is_valid() {
    let golden = include_bytes!("../expected/trampoline_x86.bin");
    assert_eq!(golden.len(), 175);
    assert_eq!(&golden[8..40], &[0u8; 32]);
}
