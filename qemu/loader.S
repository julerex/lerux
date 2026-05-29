/* Minimal QEMU loader for lerux (x86_64) using GNU as (gas).
 * This version can be built in environments without nasm.
 *
 * It prefers loading the kernel from physical address 0x200000
 * (placed there via QEMU -device loader).
 *
 * Assemble + link example:
 *   as --64 -o loader.o loader.S
 *   ld -T loader.ld -o loader.elf loader.o
 *   objcopy -O binary loader.elf loader.bin
 */

#define PAGE_SIZE 0x1000

/* When using QEMU -device loader, we place the kernel at this fixed physical address.
 * This is the most reliable method during bring-up. */
.equ KERNEL_PHYS_ADDR, 0x200000

/* Multiboot v1 header — much more reliable with QEMU -kernel */
.section .multiboot, "a"
    .long 0x1BADB002
    .long 0x00010003          /* flags: want modules + meminfo */
    .long -(0x1BADB002 + 0x00010003)
    .long 0, 0, 0, 0, 0       /* header_addr, load_addr, etc (0 = use ELF) */
    .long 0                   /* entry (use ELF entry) */

.section .text32, "ax"
.code32
.global _start
_start:
    cli
    cld
    mov %ebx, %edi               /* save multiboot2 info */

    mov $stack32_top, %esp

    /* Build page tables */
    mov $pdpt_low, %eax
    or  $0x3, %eax
    mov %eax, pml4

    mov $pdpt_high, %eax
    or  $0x3, %eax
    mov %eax, pml4 + 511*8

    mov $pd_low, %eax
    or  $0x3, %eax
    mov %eax, pdpt_low

    /* 512 huge 2MiB pages for identity 0-1GiB */
    xor %ecx, %ecx
    mov $0x83, %eax
1:
    mov %eax, pd_low(,%ecx,8)
    add $0x200000, %eax
    inc %ecx
    cmp $512, %ecx
    jb 1b

    /* Higher half mapping (same physical at 0xffffffff80000000) */
    mov $pd_low, %eax
    or  $0x3, %eax
    mov %eax, pdpt_high + 510*8

    mov $pml4, %eax
    mov %eax, %cr3

    mov %cr4, %eax
    or  $(1<<5) | (1<<7), %eax
    mov %eax, %cr4

    mov $0xC0000080, %ecx
    rdmsr
    or  $(1<<8), %eax
    wrmsr

    lgdt gdt64_ptr

    mov %cr0, %eax
    or  $(1<<31) | (1<<16) | 1, %eax
    mov %eax, %cr0

    ljmp $0x08, $long_mode_entry

.section .text64, "ax"
.code64
long_mode_entry:
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    mov $stack64_top, %rsp

    /* === Try to find and load the kernel === */
    /* Preferred path: kernel placed at fixed physical address via -device loader */
    mov $KERNEL_PHYS_ADDR, %rdi
    call try_load_kernel_at
    test %rax, %rax
    jnz have_kernel

    /* Fallback: look via multiboot module */
    call find_kernel_module
    test %rax, %rax
    jz no_kernel

    mov %rax, %rdi
    call load_elf64_segments
    jmp have_kernel

have_kernel:
    call build_minimal_kernel_args
    /* RDI = &KernelArgs */

    mov kernel_entry_va(%rip), %rax
    jmp *%rax

no_kernel:
    mov $'N', %al
    mov $0x3f8, %dx
    out %al, (%dx)
    cli
    hlt

/* rdi = ELF phys */
load_elf64_segments:
    cmpl $0x464C457F, (%rdi)
    jne bad_elf

    mov 0x18(%rdi), %rax
    mov %rax, kernel_entry_va(%rip)

    mov 0x20(%rdi), %r8          /* phoff */
    movzwl 0x38(%rdi), %r9d      /* phnum */
    movzwl 0x36(%rdi), %r10d     /* phentsize */
    add %rdi, %r8

ph_loop:
    test %r9, %r9
    jz 1f

    cmpl $1, (%r8)               /* PT_LOAD */
    jne next_ph

    mov %rdi, %rsi
    add 8(%r8), %rsi             /* p_offset */

    mov 0x10(%r8), %rdx          /* p_paddr */
    mov 0x20(%r8), %rcx          /* filesz */

    push %rdi
    mov %rdx, %rdi
    rep movsb
    pop %rdi

    /* zero bss */
    mov 0x28(%r8), %rcx
    sub 0x20(%r8), %rcx
    mov %rdx, %rdi
    add 0x20(%r8), %rdi
    xor %al, %al
    rep stosb

next_ph:
    add %r10, %r8
    dec %r9
    jmp ph_loop
1:
    ret

bad_elf:
    mov $'E', %al
    mov $0x3f8, %dx
    out %al, (%dx)
    cli
    hlt

find_kernel_module:
    mov %rdi, %rbx
    mov (%rbx), %ecx
    add %rbx, %rcx
    add $8, %rbx

.tag_loop:
    cmp %rcx, %rbx
    jae .not_found
    mov (%rbx), %eax
    mov 4(%rbx), %edx
    cmp $3, %eax
    je .found
    add %rdx, %rbx
    add $7, %rbx
    and $-8, %rbx
    jmp .tag_loop

.found:
    mov 8(%rbx), %eax
    ret
.not_found:
    xor %eax, %eax
    ret

/* Try to load a kernel ELF from a known physical address.
 * Input:  %rdi = physical address to check
 * Output: %rax = 0 on failure, the address on success
 */
try_load_kernel_at:
    cmpl $0x464C457F, (%rdi)          /* \x7FELF magic? */
    jne .fail

    push %rdi
    call load_elf64_segments
    pop %rax
    ret
.fail:
    xor %eax, %eax
    ret

build_minimal_kernel_args:
    mov $0x20000, %rdi
    xor %eax, %eax
    mov $32, %ecx
    rep stosl
    mov $0x20000, %rdi

    movq $0x100000,   0(%rdi)
    movq $0x1000000,  8(%rdi)
    movq $0x80000,   16(%rdi)
    movq $0x40000,   24(%rdi)
    movq $0,         32(%rdi)
    movq $0,         40(%rdi)
    movq $0,         48(%rdi)
    movq $0,         56(%rdi)
    movq $memory_map,64(%rdi)
    movq $128,       72(%rdi)
    movq $0x40000,   80(%rdi)
    movq $0x10000,   88(%rdi)

    /* minimal memory map entry */
    mov $memory_map, %rax
    movq $0x1000,    0(%rax)
    movq $0x7F00000, 8(%rax)
    movq $1,        16(%rax)

    ret

.section .data
.align 8
kernel_entry_va:
    .quad 0

/* GDT */
.section .rodata
.align 16
gdt64:
    .quad 0
    .quad 0x00AF9A000000FFFF   /* code64 */
    .quad 0x00AF92000000FFFF   /* data64 */
gdt64_ptr:
    .word . - gdt64 - 1
    .quad gdt64

.section .bss
.align 4096
pml4:       .skip 4096
pdpt_low:   .skip 4096
pd_low:     .skip 4096
pdpt_high:  .skip 4096

memory_map: .skip 4096

stack32_bottom: .skip 4096
stack32_top:

stack64_bottom: .skip 4096*4
stack64_top:
