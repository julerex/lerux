/* Machine probe for the one x86 ISO.
   The Swift panel is the GOP buffer at SWIFT_FB_PHYS. Every other machine,
   including one whose firmware has no framebuffer, uses the VGA text page.
   The mode page is reserved so seL4 does not clear the kind byte. Device
   windows that are already RAM are left alone: this image does not steal
   them, and it does not invent a window for the Inspiron AHCI disk. */
#pragma once

#include <stddef.h>
#include <stdint.h>

#define CONSOLE_KIND_OFF 36u
#define CONSOLE_PANEL 1u
#define CONSOLE_VGA 2u

#define MMAP_RAM 1u
#define MMAP_RESERVED 2u

#define MODE_PAGE_PHYS 0x01FE0000ull
#define MODE_PAGE_LEN 0x1000ull
#define SWIFT_FB_PHYS 0x00FC90000000ull

typedef struct {
    uint64_t addr;
    uint64_t len;
    uint32_t type;
} mem_span;

/* PANEL when the firmware buffer is the Swift panel. Anything else is VGA. */
uint8_t console_kind_for_framebuffer(uint64_t addr);

/* 1 when one span contains the whole range. */
int span_covers(const mem_span *spans, size_t count, uint64_t addr, uint64_t len);

/* 1 when a RAM span overlaps the range. */
int span_overlaps_ram(const mem_span *spans, size_t count, uint64_t addr, uint64_t len);

/* Copy spans and turn the mode page into a reserved span.
   Returns 0 when `out_cap` is too small. */
size_t carve_mode_page(const mem_span *in, size_t count, mem_span *out, size_t out_cap);

/* 1 when the last byte of [addr, addr + len) sits below 2^phys_bits. */
int span_fits_phys(uint64_t addr, uint64_t len, unsigned phys_bits);

/* CPUID leaf 0x80000008 EAX[7:0]. A missing leaf is 36, the Intel default. */
unsigned host_phys_bits(void);

/* Append a device window only when it is not already RAM and the CPU can
 * address it. A 36-bit machine does not get the Swift panel or the MT7921
 * window. Returns the new count, or 0 when the list cannot grow. */
size_t reserve_missing_windows(mem_span *spans, size_t count, size_t cap, unsigned phys_bits);

/* Write 0xA5 and then 0x5A. Returns 1 when both patterns read back.
   A null or empty window returns 0. */
int memtest_window(uint8_t *window, size_t len);
