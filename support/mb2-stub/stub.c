/*
 * Multiboot 2 trampoline for UEFI Limine.
 *
 * Limine refuses to place sel4.elf at its 1 MiB link address while UEFI boot
 * services still own that range. This stub is linked at 32 MiB so Limine can
 * load it. After ExitBootServices it ELF-loads the 64-bit sel4.elf to 1 MiB,
 * parks loader.img above the kernel, rebuilds a Multiboot 2 info block, and
 * jumps to the kernel entry (still Multiboot 2 / i386 machine state).
 * CR4 is 0 on entry. support/mb2-stub/build.sh passes -mgeneral-regs-only so
 * the clear of new_mbi cannot become an SSE store.
 *
 * Limine modules (order matters):
 *   0: sel4.elf   (x86-64 ELF file bytes)
 *   1: loader.img (Microkit initial task)
 */

#include <stdint.h>

#include "multiboot2.h"

#define PAGE_SIZE 0x1000u
#define KERNEL_LINK_FLOOR 0x00100000u
#define STUB_LINK_BASE 0x02000000u

#define ELF_MAGIC 0x464c457fu
#define PT_LOAD 1u
#define EI_CLASS 4
#define ELFCLASS64 2
#define EM_X86_64 62

struct elf64_ehdr {
    uint8_t e_ident[16];
    uint16_t e_type;
    uint16_t e_machine;
    uint32_t e_version;
    uint64_t e_entry;
    uint64_t e_phoff;
    uint64_t e_shoff;
    uint32_t e_flags;
    uint16_t e_ehsize;
    uint16_t e_phentsize;
    uint16_t e_phnum;
    uint16_t e_shentsize;
    uint16_t e_shnum;
    uint16_t e_shstrndx;
};

struct elf64_phdr {
    uint32_t p_type;
    uint32_t p_flags;
    uint64_t p_offset;
    uint64_t p_vaddr;
    uint64_t p_paddr;
    uint64_t p_filesz;
    uint64_t p_memsz;
    uint64_t p_align;
};

/* Fresh Multiboot 2 info for seL4. Kept out of the 1 MiB kernel window. */
static uint8_t new_mbi[8192] __attribute__((aligned(8)));

static void *memcpy(void *dst, const void *src, uint32_t n) {
    uint8_t *d = dst;
    const uint8_t *s = src;
    while (n--) {
        *d++ = *s++;
    }
    return dst;
}

static void *memset(void *dst, int c, uint32_t n) {
    uint8_t *d = dst;
    while (n--) {
        *d++ = (uint8_t)c;
    }
    return dst;
}

static uint32_t strlen_local(const char *s) {
    uint32_t n = 0;
    while (s[n]) {
        n++;
    }
    return n;
}

static uint32_t align_up(uint32_t v, uint32_t a) {
    return (v + a - 1u) & ~(a - 1u);
}

static void com1_write(const char *s) {
    while (*s) {
        __asm__ volatile("outb %0, %1" : : "a"((uint8_t)*s), "Nd"((uint16_t)0x3f8));
        s++;
    }
}

static void fail(const char *msg) {
    com1_write("mb2-stub: ");
    com1_write(msg);
    com1_write("\r\n");
    for (;;) {
        __asm__ volatile("hlt");
    }
}

static int find_modules(
    struct multiboot2_header *mbi,
    uint32_t *kernel_start,
    uint32_t *kernel_end,
    uint32_t *loader_start,
    uint32_t *loader_end
) {
    uint8_t *p = (uint8_t *)(mbi + 1);
    uint8_t *end = (uint8_t *)mbi + mbi->total_size;
    int count = 0;

    while (p + sizeof(struct multiboot2_tag) <= end) {
        struct multiboot2_tag *tag = (struct multiboot2_tag *)p;
        if (tag->type == MULTIBOOT2_TAG_END) {
            break;
        }
        if (tag->type == MULTIBOOT2_TAG_MODULE) {
            struct multiboot2_tag_module *mod = (struct multiboot2_tag_module *)tag;
            if (count == 0) {
                *kernel_start = mod->mod_start;
                *kernel_end = mod->mod_end;
            } else if (count == 1) {
                *loader_start = mod->mod_start;
                *loader_end = mod->mod_end;
            }
            count++;
        }
        p += align_up(tag->size, 8);
    }
    return count;
}

static uint32_t elf_load_kernel(const uint8_t *file, uint32_t file_size, uint32_t *entry_out) {
    if (file_size < sizeof(struct elf64_ehdr)) {
        fail("kernel ELF truncated");
    }
    const struct elf64_ehdr *eh = (const struct elf64_ehdr *)file;
    if (*(const uint32_t *)eh->e_ident != ELF_MAGIC) {
        fail("kernel not ELF");
    }
    if (eh->e_ident[EI_CLASS] != ELFCLASS64 || eh->e_machine != EM_X86_64) {
        fail("kernel not x86-64 ELF64");
    }
    if (eh->e_phoff == 0 || eh->e_phnum == 0) {
        fail("kernel has no program headers");
    }
    if (eh->e_entry > 0xffffffffull) {
        fail("kernel entry not 32-bit Multiboot reachable");
    }
    if (eh->e_phoff > file_size) {
        fail("kernel phoff past file end");
    }

    uint32_t kernel_end = 0;
    const uint8_t *ph_base = file + (uint32_t)eh->e_phoff;
    for (uint16_t i = 0; i < eh->e_phnum; i++) {
        const struct elf64_phdr *ph =
            (const struct elf64_phdr *)(ph_base + (uint32_t)i * eh->e_phentsize);
        if (ph->p_type != PT_LOAD) {
            continue;
        }
        if (ph->p_paddr > 0xffffffffull || ph->p_filesz > 0xffffffffull
            || ph->p_memsz > 0xffffffffull || ph->p_offset > 0xffffffffull) {
            fail("kernel PHDR not in 32-bit physical range");
        }
        uint32_t paddr = (uint32_t)ph->p_paddr;
        uint32_t filesz = (uint32_t)ph->p_filesz;
        uint32_t memsz = (uint32_t)ph->p_memsz;
        uint32_t offset = (uint32_t)ph->p_offset;
        if (offset + filesz > file_size) {
            fail("kernel PHDR past file end");
        }
        if (paddr < KERNEL_LINK_FLOOR) {
            fail("kernel PHDR below 1MiB");
        }
        if (paddr < STUB_LINK_BASE && paddr + memsz > STUB_LINK_BASE) {
            fail("kernel overlaps stub");
        }
        uint8_t *dest = (uint8_t *)(uintptr_t)paddr;
        memcpy(dest, file + offset, filesz);
        if (memsz > filesz) {
            memset(dest + filesz, 0, memsz - filesz);
        }
        uint32_t seg_end = paddr + memsz;
        if (seg_end > kernel_end) {
            kernel_end = seg_end;
        }
    }
    if (kernel_end == 0) {
        fail("kernel has no PT_LOAD");
    }
    *entry_out = (uint32_t)eh->e_entry;
    return align_up(kernel_end, PAGE_SIZE);
}

static uint32_t append_tag(uint32_t idx, uint32_t size) {
    return align_up(idx + size, 8);
}

static void copy_forwarded_tags(struct multiboot2_header *src, uint32_t *idx) {
    uint8_t *p = (uint8_t *)(src + 1);
    uint8_t *end = (uint8_t *)src + src->total_size;

    while (p + sizeof(struct multiboot2_tag) <= end) {
        struct multiboot2_tag *tag = (struct multiboot2_tag *)p;
        if (tag->type == MULTIBOOT2_TAG_END) {
            break;
        }
        int forward = tag->type == MULTIBOOT2_TAG_CMDLINE
            || tag->type == MULTIBOOT2_TAG_BOOT_LOADER_NAME
            || tag->type == MULTIBOOT2_TAG_BASIC_MEMINFO
            || tag->type == MULTIBOOT2_TAG_MMAP
            || tag->type == MULTIBOOT2_TAG_FRAMEBUFFER
            || tag->type == MULTIBOOT2_TAG_ACPI_OLD
            || tag->type == MULTIBOOT2_TAG_ACPI_NEW;
        if (forward) {
            if (*idx + tag->size > sizeof(new_mbi)) {
                fail("mbi overflow");
            }
            memcpy(new_mbi + *idx, tag, tag->size);
            *idx = append_tag(*idx, tag->size);
        }
        p += align_up(tag->size, 8);
    }
}

void stub_main(uint32_t magic, struct multiboot2_header *mbi) {
    if (magic != MULTIBOOT2_BOOTLOADER_MAGIC) {
        fail("bad multiboot magic");
    }
    if (mbi == 0) {
        fail("null mbi");
    }

    uint32_t k_start = 0, k_end = 0, l_start = 0, l_end = 0;
    if (find_modules(mbi, &k_start, &k_end, &l_start, &l_end) < 2) {
        fail("need sel4.elf and loader.img modules");
    }
    if (k_end <= k_start || l_end <= l_start) {
        fail("empty module");
    }

    uint32_t entry = 0;
    uint32_t kernel_phys_end =
        elf_load_kernel((const uint8_t *)(uintptr_t)k_start, k_end - k_start, &entry);

    uint32_t loader_size = l_end - l_start;
    uint32_t loader_dest = kernel_phys_end;
    if (loader_size > 0xffffffffu - loader_dest) {
        fail("loader dest wrap");
    }
    if (loader_dest + loader_size > STUB_LINK_BASE) {
        loader_dest = align_up(STUB_LINK_BASE + 0x100000u, PAGE_SIZE);
    }
    memcpy((void *)(uintptr_t)loader_dest, (const void *)(uintptr_t)l_start, loader_size);

    memset(new_mbi, 0, sizeof(new_mbi));
    uint32_t idx = sizeof(struct multiboot2_header);

    {
        const char *cmd = "";
        uint32_t cmd_len = strlen_local(cmd) + 1;
        uint32_t size = sizeof(struct multiboot2_tag_module) + cmd_len;
        if (idx + size > sizeof(new_mbi)) {
            fail("module tag overflow");
        }
        struct multiboot2_tag_module *mod = (struct multiboot2_tag_module *)(new_mbi + idx);
        mod->type = MULTIBOOT2_TAG_MODULE;
        mod->size = size;
        mod->mod_start = loader_dest;
        mod->mod_end = loader_dest + loader_size;
        memcpy(mod->cmdline, cmd, cmd_len);
        idx = append_tag(idx, size);
    }

    copy_forwarded_tags(mbi, &idx);

    {
        struct multiboot2_tag *end = (struct multiboot2_tag *)(new_mbi + idx);
        end->type = MULTIBOOT2_TAG_END;
        end->size = sizeof(struct multiboot2_tag);
        idx = append_tag(idx, sizeof(struct multiboot2_tag));
    }

    struct multiboot2_header *out = (struct multiboot2_header *)new_mbi;
    out->total_size = idx;
    out->reserved = 0;

    com1_write("mb2-stub: jumping to seL4\r\n");

    uint32_t mb_magic = MULTIBOOT2_BOOTLOADER_MAGIC;
    __asm__ volatile(
        "movl %0, %%eax\n\t"
        "movl %1, %%ebx\n\t"
        "jmp *%2"
        :
        : "r"(mb_magic), "r"(out), "r"(entry)
        : "eax", "ebx", "memory"
    );

    fail("seL4 returned");
}
