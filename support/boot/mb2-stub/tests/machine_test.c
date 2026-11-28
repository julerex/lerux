#include <stdio.h>

#include "machine.h"

static int fail(const char *msg) {
    fprintf(stderr, "machine probe: %s\n", msg);
    return 1;
}

static int test_kind(void) {
    if (console_kind_for_framebuffer(SWIFT_FB_PHYS) != CONSOLE_PANEL) {
        return fail("swift framebuffer is the panel");
    }
    if (console_kind_for_framebuffer(0x80000000ull) != CONSOLE_VGA) {
        return fail("ovmf framebuffer is vga");
    }
    if (console_kind_for_framebuffer(0) != CONSOLE_VGA) {
        return fail("no framebuffer is vga");
    }
    if (CONSOLE_KIND_OFF != 36 || CONSOLE_PANEL != 1 || CONSOLE_VGA != 2) {
        return fail("kind constants");
    }
    return 0;
}

static int test_carve(void) {
    mem_span in[2];
    mem_span out[8];
    size_t n;
    in[0].addr = 0x100000ull;
    in[0].len = 0x1f00000ull;
    in[0].type = MMAP_RAM;
    in[1].addr = 0xb8000ull;
    in[1].len = 0x8000ull;
    in[1].type = MMAP_RESERVED;
    n = carve_mode_page(in, 2, out, 8);
    if (n != 4) {
        return fail("carve split count");
    }
    if (out[0].addr != 0x100000ull || out[0].type != MMAP_RAM) {
        return fail("ram below the mode page");
    }
    if (out[0].addr + out[0].len != MODE_PAGE_PHYS) {
        return fail("ram ends at the mode page");
    }
    if (out[1].addr != MODE_PAGE_PHYS || out[1].len != MODE_PAGE_LEN || out[1].type != MMAP_RESERVED) {
        return fail("mode page is reserved");
    }
    if (out[2].addr != MODE_PAGE_PHYS + MODE_PAGE_LEN || out[2].type != MMAP_RAM) {
        return fail("ram above the mode page");
    }
    if (out[3].addr != 0xb8000ull || out[3].type != MMAP_RESERVED) {
        return fail("vga span kept");
    }
    if (!span_covers(out, n, MODE_PAGE_PHYS, MODE_PAGE_LEN)) {
        return fail("mode page covered");
    }
    if (span_overlaps_ram(out, n, MODE_PAGE_PHYS, MODE_PAGE_LEN)) {
        return fail("mode page still ram");
    }
    return 0;
}

static int test_windows(void) {
    mem_span in[2];
    mem_span spans[16];
    size_t n;
    size_t i;
    in[0].addr = 0x100000ull;
    in[0].len = 0x10000000ull;
    in[0].type = MMAP_RAM;
    in[1].addr = 0xa0000ull;
    in[1].len = 0x60000ull;
    in[1].type = MMAP_RESERVED;
    n = carve_mode_page(in, 2, spans, 16);
    if (n == 0) {
        return fail("carve before windows");
    }
    n = reserve_missing_windows(spans, n, 16, 48);
    if (n == 0) {
        return fail("reserve windows");
    }
    if (!span_covers(spans, n, SWIFT_FB_PHYS, 0x7e9000ull)) {
        return fail("panel window reserved");
    }
    if (!span_covers(spans, n, 0xfc82000000ull, 0x200000ull)) {
        return fail("wifi window reserved");
    }
    if (!span_covers(spans, n, 0xb8000ull, 0x1000ull)) {
        return fail("vga window covered");
    }
    if (span_overlaps_ram(spans, n, SWIFT_FB_PHYS, 0x1000ull)) {
        return fail("panel window stolen from ram");
    }
    /* A window that sits in RAM is not turned into a device hole. */
    spans[0].addr = SWIFT_FB_PHYS;
    spans[0].len = 0x1000000ull;
    spans[0].type = MMAP_RAM;
    n = reserve_missing_windows(spans, 1, 16, 48);
    for (i = 1; i < n; i++) {
        if (spans[i].addr == SWIFT_FB_PHYS) {
            return fail("ram panel was reserved again");
        }
    }
    if (spans[0].type != MMAP_RAM || spans[0].len != 0x1000000ull) {
        return fail("overlapping ram kept");
    }
    return 0;
}

static int high_covered(unsigned phys_bits, uint64_t addr) {
    mem_span spans[16];
    size_t n;

    spans[0].addr = 0x100000ull;
    spans[0].len = 0x100000ull;
    spans[0].type = MMAP_RAM;
    n = reserve_missing_windows(spans, 1, 16, phys_bits);
    if (n == 0) {
        return -1;
    }
    return span_covers(spans, n, addr, 0x1000ull);
}

static int test_phys_limit(void) {
    int panel36 = high_covered(36, SWIFT_FB_PHYS);
    int wifi36 = high_covered(36, 0x000000fc82000000ull);
    int panel39 = high_covered(39, SWIFT_FB_PHYS);
    int panel40 = high_covered(40, SWIFT_FB_PHYS);
    int wifi40 = high_covered(40, 0x000000fc82000000ull);
    int vga36 = high_covered(36, 0xb8000ull);

    if (panel36 < 0 || wifi36 < 0 || panel39 < 0 || panel40 < 0 || wifi40 < 0 || vga36 < 0) {
        return fail("phys reserve");
    }
    printf("phys36-panel %d\n", panel36);
    printf("phys36-wifi %d\n", wifi36);
    printf("phys39-panel %d\n", panel39);
    printf("phys40-panel %d\n", panel40);
    printf("phys40-wifi %d\n", wifi40);
    printf("phys36-vga %d\n", vga36);
    if (panel36 != 0 || wifi36 != 0 || panel39 != 0) {
        return fail("narrow cpu reserved a high window");
    }
    if (panel40 != 1 || wifi40 != 1 || vga36 != 1) {
        return fail("wide cpu dropped an addressable window");
    }
    return 0;
}

int main(void) {
    if (test_kind() || test_carve() || test_windows() || test_phys_limit()) {
        return 1;
    }
    printf("kind-swift %u\n", (unsigned)console_kind_for_framebuffer(SWIFT_FB_PHYS));
    printf("kind-ovmf %u\n", (unsigned)console_kind_for_framebuffer(0x80000000ull));
    printf("kind-none %u\n", (unsigned)console_kind_for_framebuffer(0));
    printf("carve-ok\n");
    return 0;
}
