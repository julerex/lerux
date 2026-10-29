/*
 * Multiboot 2 trampoline for UEFI.
 *
 * sel4.elf is linked at 1 MiB. A loader that checks that address before
 * ExitBootServices cannot place it: firmware still owns the range. This stub
 * is linked at 32 MiB so the loader can place it. After ExitBootServices it
 * ELF-loads the 64-bit sel4.elf to 1 MiB, parks loader.img above the kernel,
 * rebuilds a Multiboot 2 info block, and jumps to the kernel entry (still
 * Multiboot 2 / i386 machine state).
 *
 * Modules (order matters):
 *   0: sel4.elf   (x86-64 ELF file bytes)
 *   1: loader.img (Microkit initial task)
 */

#include <stdint.h>

#include "fb_paint.h"
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

/* Swift panel driver maps this page. The linker places it at 0x01FE0000.
   A 4096-byte alignment here raises the segment alignment and pushes the
   multiboot header to file offset 8192, past GRUB's scan. */
__attribute__((section(".swift_mode"))) static uint8_t mode_page[4096];

#define MODE_MAGIC 0x4D4F44454C525558ull
#define PAE_WINDOW 0x10000000u

static uint64_t pdpt[4] __attribute__((aligned(32)));
static uint64_t pd[512] __attribute__((aligned(4096)));

struct __attribute__((packed)) fb_tag {
    uint32_t type;
    uint32_t size;
    uint64_t addr;
    uint32_t pitch;
    uint32_t width;
    uint32_t height;
    uint8_t bpp;
    uint8_t fb_type;
    uint16_t reserved;
    uint8_t red_shift;
    uint8_t red_width;
    uint8_t green_shift;
    uint8_t green_width;
    uint8_t blue_shift;
    uint8_t blue_width;
};

static void *memcpy(void *dst, const void *src, uint32_t n) {
    uint8_t *d = dst;
    const uint8_t *s = src;
    while (n--) {
        *d++ = *s++;
    }
    return dst;
}

static void memmove_bytes(void *dst, const void *src, uint32_t n) {
    uint8_t *d = dst;
    const uint8_t *s = src;
    if (d == s || n == 0) {
        return;
    }
    if ((uintptr_t)d < (uintptr_t)s) {
        while (n--) {
            *d++ = *s++;
        }
        return;
    }
    d += n;
    s += n;
    while (n--) {
        *--d = *--s;
    }
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

/* Highest physical address the kernel image will occupy, page aligned.
   Used to park the module files above that image before any segment is
   copied. GRUB places those files at 1 MiB, which is also seL4's load
   address. Zeroing the first segment then erases the file, and the higher
   segments are copied as zeros. */
static uint32_t kernel_image_end(const uint8_t *file, uint32_t file_size) {
    if (file_size < sizeof(struct elf64_ehdr)) {
        fail("kernel ELF truncated");
    }
    const struct elf64_ehdr *eh = (const struct elf64_ehdr *)file;
    if (*(const uint32_t *)eh->e_ident != ELF_MAGIC) {
        fail("kernel not ELF");
    }
    if (eh->e_phoff > file_size || eh->e_phnum == 0) {
        fail("kernel has no program headers");
    }
    uint32_t kernel_end = 0;
    const uint8_t *ph_base = file + (uint32_t)eh->e_phoff;
    for (uint16_t i = 0; i < eh->e_phnum; i++) {
        const struct elf64_phdr *ph =
            (const struct elf64_phdr *)(ph_base + (uint32_t)i * eh->e_phentsize);
        if (ph->p_type != PT_LOAD) {
            continue;
        }
        if (ph->p_paddr > 0xffffffffull || ph->p_memsz > 0xffffffffull) {
            fail("kernel PHDR not in 32-bit physical range");
        }
        uint32_t paddr = (uint32_t)ph->p_paddr;
        uint32_t memsz = (uint32_t)ph->p_memsz;
        if (memsz > 0xffffffffu - paddr) {
            fail("kernel image end wrap");
        }
        uint32_t seg_end = paddr + memsz;
        if (seg_end > kernel_end) {
            kernel_end = seg_end;
        }
    }
    if (kernel_end == 0) {
        fail("kernel has no PT_LOAD");
    }
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

static void store_u32(uint8_t *out, uint32_t value) {
    out[0] = (uint8_t)value;
    out[1] = (uint8_t)(value >> 8);
    out[2] = (uint8_t)(value >> 16);
    out[3] = (uint8_t)(value >> 24);
}

static void store_u64(uint8_t *out, uint64_t value) {
    store_u32(out, (uint32_t)value);
    store_u32(out + 4, (uint32_t)(value >> 32));
}

static void write_mode_record(const struct fb_tag *tag) {
    memset(mode_page, 0, 64);
    store_u64(mode_page, MODE_MAGIC);
    store_u64(mode_page + 8, tag->addr);
    store_u32(mode_page + 16, tag->pitch);
    store_u32(mode_page + 20, tag->width);
    store_u32(mode_page + 24, tag->height);
    mode_page[28] = tag->bpp;
    mode_page[29] = tag->red_shift;
    mode_page[30] = tag->red_width;
    mode_page[31] = tag->green_shift;
    mode_page[32] = tag->green_width;
    mode_page[33] = tag->blue_shift;
    mode_page[34] = tag->blue_width;
}

static uint32_t read_cr(const char *which) {
    uint32_t value;
    if (which[2] == '0') {
        __asm__ volatile("mov %%cr0, %0" : "=r"(value));
    } else if (which[2] == '3') {
        __asm__ volatile("mov %%cr3, %0" : "=r"(value));
    } else {
        __asm__ volatile("mov %%cr4, %0" : "=r"(value));
    }
    return value;
}

static void write_cr0(uint32_t value) {
    __asm__ volatile("mov %0, %%cr0" : : "r"(value) : "memory");
}

static void write_cr3(uint32_t value) {
    __asm__ volatile("mov %0, %%cr3" : : "r"(value) : "memory");
}

static void write_cr4(uint32_t value) {
    __asm__ volatile("mov %0, %%cr4" : : "r"(value) : "memory");
}

static void paint_high(const struct fb_tag *tag, const char *line) {
    uint64_t base = tag->addr & ~0x1fffffull;
    uint32_t offset = (uint32_t)(tag->addr - base);
    uint32_t i;
    uint32_t cr0 = read_cr("cr0");
    uint32_t cr3 = read_cr("cr3");
    uint32_t cr4 = read_cr("cr4");
    uint32_t cr0_off = cr0 & ~(1u << 31);
    memset(pdpt, 0, sizeof(pdpt));
    for (i = 0; i < 512; i++) {
        pd[i] = ((uint64_t)i << 21) | 0x83u;
    }
    for (i = 0; i < 8; i++) {
        pd[128 + i] = (base + ((uint64_t)i << 21)) | 0x83u;
    }
    pdpt[0] = ((uint64_t)(uint32_t)(uintptr_t)pd) | 0x1u;
    write_cr0(cr0_off);
    write_cr4(cr4 | (1u << 5));
    write_cr3((uint32_t)(uintptr_t)pdpt);
    write_cr0(cr0);
    fb_paint_line(
        (uint8_t *)(uintptr_t)(PAE_WINDOW + offset),
        tag->pitch,
        tag->width,
        tag->height,
        tag->red_shift,
        tag->green_shift,
        tag->blue_shift,
        line
    );
    write_cr0(cr0_off);
    write_cr4(cr4);
    write_cr3(cr3);
    write_cr0(cr0);
}

static const struct fb_tag *framebuffer_tag(struct multiboot2_header *mbi) {
    uint8_t *p = (uint8_t *)(mbi + 1);
    uint8_t *end = (uint8_t *)mbi + mbi->total_size;
    while (p + sizeof(struct multiboot2_tag) <= end) {
        struct multiboot2_tag *tag = (struct multiboot2_tag *)p;
        if (tag->type == MULTIBOOT2_TAG_END) {
            return 0;
        }
        if (tag->type == MULTIBOOT2_TAG_FRAMEBUFFER && tag->size >= sizeof(struct fb_tag)) {
            return (const struct fb_tag *)tag;
        }
        p += align_up(tag->size, 8);
    }
    return 0;
}

static void paint_firmware_mode(struct multiboot2_header *mbi) {
    const struct fb_tag *tag = framebuffer_tag(mbi);
    struct fb_mode mode;
    char line[80];
    uint64_t bytes;
    if (tag == 0 || tag->fb_type != 1 || tag->bpp != 32 || tag->width < 16 || tag->height < 16
        || tag->pitch < tag->width * 4u || tag->pitch > 16384u) {
        return;
    }
    bytes = (uint64_t)tag->pitch * tag->height;
    if (bytes < tag->pitch || (tag->addr & 0xfff) != 0) {
        return;
    }
    write_mode_record(tag);
    mode.addr = tag->addr;
    mode.pitch = tag->pitch;
    mode.width = tag->width;
    mode.height = tag->height;
    mode.red_shift = tag->red_shift;
    mode.green_shift = tag->green_shift;
    mode.blue_shift = tag->blue_shift;
    if (fb_mode_line(&mode, line, sizeof(line)) == 0) {
        return;
    }
    if (tag->addr <= 0xffffffffull && tag->addr + 16ull * tag->pitch <= 0x100000000ull) {
        fb_paint_line(
            (uint8_t *)(uintptr_t)tag->addr,
            tag->pitch,
            tag->width,
            tag->height,
            tag->red_shift,
            tag->green_shift,
            tag->blue_shift,
            line
        );
        return;
    }
    if (tag->addr + 16ull * tag->pitch < tag->addr) {
        return;
    }
    paint_high(tag, line);
}

void stub_main(uint32_t magic, struct multiboot2_header *mbi) {
    if (magic != MULTIBOOT2_BOOTLOADER_MAGIC) {
        fail("bad multiboot magic");
    }
    if (mbi == 0) {
        fail("null mbi");
    }
    paint_firmware_mode(mbi);

    uint32_t k_start = 0, k_end = 0, l_start = 0, l_end = 0;
    if (find_modules(mbi, &k_start, &k_end, &l_start, &l_end) < 2) {
        fail("need sel4.elf and loader.img modules");
    }
    if (k_end <= k_start || l_end <= l_start) {
        fail("empty module");
    }

    uint32_t kernel_size = k_end - k_start;
    uint32_t loader_size = l_end - l_start;
    uint32_t image_end = kernel_image_end((const uint8_t *)(uintptr_t)k_start, kernel_size);
    uint32_t top = image_end;
    if (k_end > top) {
        top = align_up(k_end, PAGE_SIZE);
    }
    if (l_end > top) {
        top = align_up(l_end, PAGE_SIZE);
    }
    uint32_t loader_dest = top;
    if (loader_size > 0xffffffffu - loader_dest) {
        fail("loader dest wrap");
    }
    uint32_t kernel_park = align_up(loader_dest + loader_size, PAGE_SIZE);
    if (kernel_size > 0xffffffffu - kernel_park || kernel_park + kernel_size > STUB_LINK_BASE) {
        fail("kernel park");
    }
    memmove_bytes(
        (void *)(uintptr_t)loader_dest, (const void *)(uintptr_t)l_start, loader_size
    );
    memmove_bytes(
        (void *)(uintptr_t)kernel_park, (const void *)(uintptr_t)k_start, kernel_size
    );

    uint32_t entry = 0;
    elf_load_kernel((const uint8_t *)(uintptr_t)kernel_park, kernel_size, &entry);

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
