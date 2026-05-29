//! PVH 32->64-bit boot stub for direct-boot (QEMU `-kernel`), in pure Rust via
//! `core::arch::global_asm!` (assembled by rustc/LLVM — no C toolchain).
//!
//! This replaces the former `pvh_boot.S` that was compiled by `cc`/`clang`. The
//! section placement (`.note.Xen`, `.pvh.text`, `.pvh.gdt`) and the fixed physical
//! addresses below are matched by `linkers/x86_64-direct.ld`. QEMU reads the Xen
//! ELF note to find the 32-bit entry point, sets up an initial environment, and
//! jumps to `pvh_start32`, which builds preliminary page tables, enables long
//! mode, and tail-calls the Rust entry point `kstart`.
//!
//! All addresses are physical constants so this stub needs no relocations (apart
//! from the absolute reference to `kstart`).

// Assembled with AT&T syntax to mirror the original stub verbatim. `.set` is used
// instead of C `#define` since global_asm! is not run through the C preprocessor.
core::arch::global_asm!(
    r#"
    .set PVH_ENTRY,        0x00100020
    .set PVH_PML4,         0x00108000
    .set PVH_PDPT_LOW,     0x00109000
    .set PVH_PD_LOW,       0x0010a000
    .set PVH_PDPT_HIGH,    0x0010b000
    .set PVH_PD_HIGH,      0x0010f000
    .set KERNEL_PHYS_BASE, 0x00200000
    .set PVH_STACK32,      0x0010c000
    .set PVH_STACK64,      0x0010d000
    .set PVH_GDT,          0x0010e000

    .section .note.Xen, "a", @note
    .align 4
    .long 4
    .long 4
    .long 18
    .asciz "Xen"
    .align 4
    .long PVH_ENTRY
    .align 4

    .section .pvh.text, "ax"
    .code32
    .globl pvh_start32
pvh_start32:
    cli
    cld
    mov $PVH_STACK32, %esp

    mov $PVH_PDPT_LOW, %eax
    or  $0x3, %eax
    mov $PVH_PML4, %edi
    mov %eax, (%edi)
    /* Mirror low PDPT at PML4[256] for PHYS_OFFSET linear map during early init */
    mov %eax, 256 * 8(%edi)

    mov $PVH_PDPT_HIGH, %eax
    or  $0x3, %eax
    mov %eax, 511 * 8(%edi)

    mov $PVH_PD_LOW, %eax
    or  $0x3, %eax
    mov $PVH_PDPT_LOW, %edi
    mov %eax, (%edi)

    xor %ecx, %ecx
    /* 0x183 = present | writable | huge (2 MiB pages) */
    mov $0x183, %eax
    mov $PVH_PD_LOW, %edi
1:
    mov %eax, (%edi, %ecx, 8)
    add $0x200000, %eax
    inc %ecx
    cmp $512, %ecx
    jb 1b

    mov $KERNEL_PHYS_BASE, %eax
    or  $0x183, %eax
    mov $PVH_PD_HIGH, %edi
    xor %ecx, %ecx
2:
    mov %eax, (%edi, %ecx, 8)
    add $0x200000, %eax
    inc %ecx
    cmp $512, %ecx
    jb 2b

    mov $PVH_PD_HIGH, %eax
    or  $0x3, %eax
    mov $PVH_PDPT_HIGH, %edi
    mov %eax, 510 * 8(%edi)

    mov $PVH_PML4, %eax
    mov %eax, %cr3

    mov %cr4, %eax
    or  $(1 << 5) | (1 << 7), %eax
    mov %eax, %cr4

    mov $0xC0000080, %ecx
    rdmsr
    /* EFER.LME (1<<8) enables long mode; EFER.NXE (1<<11) is required because the
     * kernel's page tables set the NX bit on data pages. Without NXE those bits are
     * reserved and the first NX-page access raises a reserved-bit page fault. */
    or  $(1 << 8) | (1 << 11), %eax
    wrmsr

    mov $PVH_GDT, %eax
    lgdt (%eax)

    mov %cr0, %eax
    or  $(1 << 31) | (1 << 16) | 1, %eax
    mov %eax, %cr0

    ljmp $0x08, $long_mode_entry

    .code64
long_mode_entry:
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss
    mov $PVH_STACK64, %rsp
    .extern kstart
    movabs $kstart, %rax
    jmp *%rax

    .section .pvh.gdt, "a"
    .align 16
    /* lgdt loads limit then base; base points at null then code then data */
    .word 0x17
    .quad gdt_table
gdt_table:
    .quad 0x0000000000000000
    .quad 0x00AF9A000000FFFF
    .quad 0x00AF92000000FFFF
"#,
    options(att_syntax)
);
