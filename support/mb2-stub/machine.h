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

/* Append reserved holes for device windows the map does not already cover.
   A window that overlaps RAM is not added. Returns 0 when `cap` is too small. */
size_t reserve_missing_windows(mem_span *spans, size_t count, size_t cap);
