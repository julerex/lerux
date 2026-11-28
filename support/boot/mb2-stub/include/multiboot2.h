/* Multiboot 2 structures for the UEFI trampoline. Spec: multiboot2.h from GRUB. */
#pragma once

#include <stdint.h>

#define MULTIBOOT2_BOOTLOADER_MAGIC 0x36d76289u

#define MULTIBOOT2_TAG_END 0
#define MULTIBOOT2_TAG_CMDLINE 1
#define MULTIBOOT2_TAG_BOOT_LOADER_NAME 2
#define MULTIBOOT2_TAG_MODULE 3
#define MULTIBOOT2_TAG_BASIC_MEMINFO 4
#define MULTIBOOT2_TAG_MMAP 6
#define MULTIBOOT2_TAG_FRAMEBUFFER 8
#define MULTIBOOT2_TAG_ACPI_OLD 14
#define MULTIBOOT2_TAG_ACPI_NEW 15

struct multiboot2_header {
    uint32_t total_size;
    uint32_t reserved;
};

struct multiboot2_tag {
    uint32_t type;
    uint32_t size;
};

struct multiboot2_tag_module {
    uint32_t type;
    uint32_t size;
    uint32_t mod_start;
    uint32_t mod_end;
    char cmdline[];
};

struct multiboot2_mmap_entry {
    uint64_t addr;
    uint64_t len;
    uint32_t type;
    uint32_t zero;
};

struct multiboot2_tag_mmap {
    uint32_t type;
    uint32_t size;
    uint32_t entry_size;
    uint32_t entry_version;
    struct multiboot2_mmap_entry entries[];
};
