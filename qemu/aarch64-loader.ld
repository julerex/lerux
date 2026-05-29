/* Linker script for the minimal aarch64 QEMU loader (lerux bring-up).
 *
 * The loader runs at a low physical address (placed by QEMU -device loader).
 * It performs early MMU setup, loads the real kernel ELF, builds KernelArgs,
 * and jumps to the kernel's high virtual entry point (kstart) with x0 = &args.
 */

OUTPUT_FORMAT("elf64-littleaarch64")
ENTRY(_start)

SECTIONS {
    /* Safe low address inside guest RAM for the loader itself.
     * Virt machine typically has RAM starting at 0x40000000; 0x100000 is fine
     * as long as it does not overlap the kernel placement.
     */
    . = 0x100000;

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(COMMON)
        *(.bss .bss.*)
    }

    /* Boot-time page tables and small buffers live here so the loader
     * can carve them with known LMA for early phys writes.
     */
    .boot : ALIGN(4096) {
        __boot_tables_start = .;
        . += 8 * 4096;   /* 8 pages for L0/L1 tables + scratch */
        __boot_tables_end = .;
    }

    __end = .;

    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}
