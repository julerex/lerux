/* Linked at 32 MiB. seL4 stays at 1 MiB; this trampoline copies it there
   after the loader has called ExitBootServices.
   The mode page is the lower address, so it is stored first in the file.
   A 4096-byte segment alignment ends that page at file offset 8192, and
   GRUB's header scan stops before the multiboot header. build.sh sets
   -z max-page-size=8 so the header stays inside the scan. Load addresses
   stay equal to these virtual addresses. */

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
