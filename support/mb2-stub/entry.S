/* Multiboot 2 header and 32-bit entry. */

.section .mbh, "a"
.align 8
mbh_start:
    .long 0xe85250d6
    .long 0
    .long mbh_end - mbh_start
    .long -(0xe85250d6 + (mbh_end - mbh_start))
    .word 0
    .word 0
    .long 8
mbh_end:

.section .text.boot, "ax"
.global _start
.code32
_start:
    /* Multiboot 2 machine state: 32-bit protected mode, eax=magic, ebx=mbi. */
    push %ebx
    push %eax
    call stub_main
1:
    hlt
    jmp 1b

.section .note.GNU-stack,"",@progbits
