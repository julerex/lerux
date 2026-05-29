/* PVH 32-bit entry for QEMU -kernel (direct-boot).
 * All addresses are physical constants so this file needs no relocations.
 */
#define PVH_ENTRY       0x00100000
#define PVH_PML4        0x00108000
#define PVH_PDPT_LOW    0x00109000
#define PVH_PD_LOW      0x0010a000
#define PVH_PDPT_HIGH   0x0010b000
#define PVH_STACK32     0x0010c000
#define PVH_STACK64     0x0010d000
#define PVH_GDT         0x0010e000

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

    mov $PVH_PDPT_HIGH, %eax
    or  $0x3, %eax
    mov %eax, 511 * 8(%edi)

    mov $PVH_PD_LOW, %eax
    or  $0x3, %eax
    mov $PVH_PDPT_LOW, %edi
    mov %eax, (%edi)

    xor %ecx, %ecx
    mov $0x83, %eax
    mov $PVH_PD_LOW, %edi
1:
    mov %eax, (%edi, %ecx, 8)
    add $0x200000, %eax
    inc %ecx
    cmp $512, %ecx
    jb 1b

    mov $PVH_PD_LOW, %eax
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
    or  $(1 << 8), %eax
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
    /* lgdt loads {limit, base} then base points at null + code + data */
    .word 0x17
    .quad gdt_table
gdt_table:
    .quad 0x0000000000000000
    .quad 0x00AF9A000000FFFF
    .quad 0x00AF92000000FFFF
