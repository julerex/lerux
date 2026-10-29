/* Linked at 32 MiB so Limine finds a free load address under UEFI.
   seL4 itself stays linked at 1 MiB; this trampoline copies it there
   after Limine has already called ExitBootServices. */

ENTRY(_start)

SECTIONS {
    /* Mode record the Swift panel driver reads. One page, below the stub. */
    . = 0x01FE0000;
    .swift_mode : {
        KEEP(*(.swift_mode))
    }

    . = 0x02000000;

    .mbh : {
        KEEP(*(.mbh))
    }

    .text : {
        *(.text.boot)
        *(.text*)
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(.bss*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.note*)
        *(.comment*)
        *(.eh_frame*)
    }
}
