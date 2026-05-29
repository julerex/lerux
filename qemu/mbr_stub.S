/* 512-byte MBR stub that loads our loader from the same "disk"
 * and jumps to it at 0x100000.
 * Very small and reliable for QEMU -fda / -fdb bring-up.
 */

.code16
.global _start
_start:
    cli
    xor %ax, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %ss
    mov $0x7c00, %sp

    /* Load the next sectors (our loader) from floppy to 0x100000 */
    /* We use LBA-ish read via int 13h (simplified) */
    mov $0x1000, %ax      /* segment for load */
    mov %ax, %es
    xor %bx, %bx

    /* Read 4 sectors starting at LBA 1 (sector 2) */
    mov $0x02, %ah        /* read */
    mov $4, %al           /* sectors */
    mov $0x00, %ch        /* cylinder */
    mov $0x02, %cl        /* sector 2 */
    mov $0x00, %dh        /* head */
    mov $0x00, %dl        /* drive 0 (floppy) */
    int $0x13
    jc error

    /* Jump to loaded code at 0x100000 (as 16-bit far jump first) */
    ljmp $0x1000, $0x0000

error:
    mov $'E', %al
    mov $0x3f8, %dx
    out %al, (%dx)
    hlt

    .org 510
    .byte 0x55, 0xaa
