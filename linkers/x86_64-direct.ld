/* Linker script for x86_64 direct-boot / QEMU -kernel (PVH note + low stub). */
ENTRY(kstart)
OUTPUT_FORMAT(elf64-x86-64)

KERNEL_OFFSET = 0xFFFFFFFF80000000;
/* Load main kernel at 2 MiB (QEMU -kernel skips most of PT_LOAD below ~1 MiB). */
KERNEL_PHYS_BASE = 0x200000;

SECTIONS
{
    .note.Xen 0x100000 : AT(0x100000) {
        KEEP(*(.note.Xen))
    }

    .pvh.boot 0x100020 : AT(0x100020) {
        KEEP(*(.pvh.text))
    }

    .pvh.gdt 0x10e000 : AT(0x10e000) {
        KEEP(*(.pvh.gdt))
    }

    . = KERNEL_OFFSET;

    /* No SIZEOF_HEADERS gap: virt 0xFFFFFFFF80000000 maps to phys KERNEL_PHYS_BASE. */
    .dummy : AT(KERNEL_PHYS_BASE) {}

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_OFFSET + KERNEL_PHYS_BASE) {
        __text_start = .;
        *(.text*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_OFFSET + KERNEL_PHYS_BASE) {
        __text_end = .;
        __rodata_start = .;
        *(.rodata*)
        __altcode_start = .;
        KEEP(*(.altcode*))
        __altcode_end = .;
        . = ALIGN(8);
        __altrelocs_start = .;
        KEEP(*(.altrelocs*))
        __altrelocs_end = .;
        __altfeatures_start = .;
        KEEP(*(.altfeatures*))
        __altfeatures_end = .;
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_OFFSET + KERNEL_PHYS_BASE) {
        __rodata_end = .;
        *(.data*)
        . = ALIGN(4K);
        *(.bss*)
    }

    __end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note.gnu*)
        *(.note.GNU*)
        *(.rel.eh_frame*)
    }
}
