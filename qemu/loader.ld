/* Linker script for the minimal QEMU loader.
 * We want the loader itself to be loaded at a low address (1 MiB or so)
 * so it can run before the main kernel.
 */

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS {
    . = 0x100000;   /* 1 MiB - safe low address */

    .multiboot2 : {
        *(.multiboot2)
    }

    .text : {
        *(.text32)
        *(.text64)
        *(.text)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data)
    }

    .bss : {
        *(COMMON)
        *(.bss)
    }

    /DISCARD/ : { *(.comment) *(.note*) }
}
