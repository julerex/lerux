#include "machine.h"

uint8_t console_kind_for_framebuffer(uint64_t addr) {
    if (addr == SWIFT_FB_PHYS) {
        return CONSOLE_PANEL;
    }
    return CONSOLE_VGA;
}

static int range_ok(uint64_t addr, uint64_t len) {
    return len != 0 && addr <= UINT64_MAX - len;
}

static int ranges_overlap(uint64_t a, uint64_t a_len, uint64_t b, uint64_t b_len) {
    uint64_t a_end;
    uint64_t b_end;
    if (!range_ok(a, a_len) || !range_ok(b, b_len)) {
        return 0;
    }
    a_end = a + a_len;
    b_end = b + b_len;
    return a < b_end && b < a_end;
}

int span_covers(const mem_span *spans, size_t count, uint64_t addr, uint64_t len) {
    size_t i;
    if (!range_ok(addr, len)) {
        return 0;
    }
    for (i = 0; i < count; i++) {
        uint64_t end;
        if (!range_ok(spans[i].addr, spans[i].len)) {
            continue;
        }
        if (spans[i].addr > addr) {
            continue;
        }
        end = spans[i].addr + spans[i].len;
        if (end >= addr + len) {
            return 1;
        }
    }
    return 0;
}

int span_overlaps_ram(const mem_span *spans, size_t count, uint64_t addr, uint64_t len) {
    size_t i;
    for (i = 0; i < count; i++) {
        if (spans[i].type == MMAP_RAM && ranges_overlap(spans[i].addr, spans[i].len, addr, len)) {
            return 1;
        }
    }
    return 0;
}

static size_t emit(mem_span *out, size_t count, size_t cap, uint64_t addr, uint64_t len, uint32_t type) {
    if (len == 0) {
        return count;
    }
    if (count >= cap) {
        return 0;
    }
    out[count].addr = addr;
    out[count].len = len;
    out[count].type = type;
    return count + 1;
}

size_t carve_mode_page(const mem_span *in, size_t count, mem_span *out, size_t out_cap) {
    size_t i;
    size_t written = 0;
    int covered = 0;
    const uint64_t page = MODE_PAGE_PHYS;
    const uint64_t page_end = MODE_PAGE_PHYS + MODE_PAGE_LEN;

    for (i = 0; i < count; i++) {
        uint64_t start = in[i].addr;
        uint64_t len = in[i].len;
        uint64_t end;
        if (!range_ok(start, len)) {
            written = emit(out, written, out_cap, start, len, in[i].type);
            if (written == 0 && len != 0) {
                return 0;
            }
            continue;
        }
        end = start + len;
        if (!ranges_overlap(start, len, page, MODE_PAGE_LEN)) {
            written = emit(out, written, out_cap, start, len, in[i].type);
            if (written == 0) {
                return 0;
            }
            continue;
        }
        covered = 1;
        if (start < page) {
            written = emit(out, written, out_cap, start, page - start, in[i].type);
            if (written == 0) {
                return 0;
            }
        }
        written = emit(
            out,
            written,
            out_cap,
            page > start ? page : start,
            (end < page_end ? end : page_end) - (page > start ? page : start),
            MMAP_RESERVED
        );
        if (written == 0) {
            return 0;
        }
        if (end > page_end) {
            written = emit(out, written, out_cap, page_end, end - page_end, in[i].type);
            if (written == 0) {
                return 0;
            }
        }
    }
    if (!covered) {
        written = emit(out, written, out_cap, page, MODE_PAGE_LEN, MMAP_RESERVED);
    }
    return written;
}

/* Windows the one image maps. The Inspiron AHCI disk is not here.
   The last two need 40 physical bits. A 36-bit or 39-bit CPU does not
   receive them. */
static const mem_span DEVICE_WINDOWS[] = {
    {0x00000000000b8000ull, 0x1000ull, MMAP_RESERVED},
    {0x00000000d1100000ull, 0x10000ull, MMAP_RESERVED},
    {0x00000000d1200000ull, 0x10000ull, MMAP_RESERVED},
    {0x00000000d1500000ull, 0x4000ull, MMAP_RESERVED},
    {0x00000000fdf00000ull, 0x4000ull, MMAP_RESERVED},
    {0x000000fc82000000ull, 0x200000ull, MMAP_RESERVED},
    {SWIFT_FB_PHYS, 0x7e9000ull, MMAP_RESERVED},
};

int span_fits_phys(uint64_t addr, uint64_t len, unsigned phys_bits) {
    if (phys_bits < 32 || phys_bits > 52 || !range_ok(addr, len)) {
        return 0;
    }
    if ((addr >> phys_bits) != 0) {
        return 0;
    }
    if (((addr + len - 1ull) >> phys_bits) != 0) {
        return 0;
    }
    return 1;
}

unsigned host_phys_bits(void) {
    uint32_t eax;
    uint32_t ebx;
    uint32_t ecx;
    uint32_t edx;

    __asm__ volatile("cpuid" : "=a"(eax), "=b"(ebx), "=c"(ecx), "=d"(edx) : "0"(0x80000000u));
    if (eax < 0x80000008u) {
        return 36u;
    }
    __asm__ volatile("cpuid" : "=a"(eax), "=b"(ebx), "=c"(ecx), "=d"(edx) : "0"(0x80000008u));
    eax &= 0xffu;
    if (eax < 32u || eax > 52u) {
        return 36u;
    }
    return eax;
}

size_t reserve_missing_windows(mem_span *spans, size_t count, size_t cap, unsigned phys_bits) {
    size_t i;
    for (i = 0; i < sizeof(DEVICE_WINDOWS) / sizeof(DEVICE_WINDOWS[0]); i++) {
        uint64_t addr = DEVICE_WINDOWS[i].addr;
        uint64_t len = DEVICE_WINDOWS[i].len;
        if (!span_fits_phys(addr, len, phys_bits)) {
            continue;
        }
        if (span_covers(spans, count, addr, len)) {
            continue;
        }
        if (span_overlaps_ram(spans, count, addr, len)) {
            continue;
        }
        if (count >= cap) {
            return 0;
        }
        spans[count].addr = addr;
        spans[count].len = len;
        spans[count].type = MMAP_RESERVED;
        count++;
    }
    return count;
}

static int paint_check(uint8_t *window, size_t len, uint8_t byte) {
    size_t i;
    for (i = 0; i < len; i++) {
        window[i] = byte;
    }
    for (i = 0; i < len; i++) {
        if (window[i] != byte) {
            return 0;
        }
    }
    return 1;
}

int memtest_window(uint8_t *window, size_t len) {
    if (window == 0 || len == 0) {
        return 0;
    }
    if (!paint_check(window, len, 0xA5)) {
        return 0;
    }
    return paint_check(window, len, 0x5A);
}
