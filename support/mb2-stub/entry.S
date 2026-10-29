/* Multiboot 2 header and 32-bit entry. */

.section .mbh, "a"
.align 8
mbh_start:
    .long 0xe85250d6
    .long 0
    .long mbh_end - mbh_start
    .long -(0xe85250d6 + (mbh_end - mbh_start))
    /* Framebuffer request, no preferred size. GRUB's default console is
       EGA text. EFI cannot set that mode, so it prints
       "WARNING: no console will be available to OS" and passes no
       framebuffer tag. Width, height, and depth 0 leave the firmware GOP
       mode in place. The size is 20; the following align pads to 8 bytes
       so the end tag stays aligned. */
    .word 5
    .word 0
    .long 20
    .long 0
    .long 0
    .long 0
    .align 8
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
