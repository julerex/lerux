/* Mode line painted into a linear 32-bit framebuffer. Host tests compile this
 * file without the trampoline. */
#pragma once

#include <stdint.h>

struct fb_mode {
    uint64_t addr;
    uint32_t pitch;
    uint32_t width;
    uint32_t height;
    uint8_t red_shift;
    uint8_t green_shift;
    uint8_t blue_shift;
};

/* `fb 0xADDR pitch PITCH WIDTHxHEIGHT rR gG bB`. Returns the length, or 0. */
unsigned fb_mode_line(const struct fb_mode *mode, char *dst, unsigned cap);

/* White glyphs on a blue bar, two pixels per font bit, at the top of `pixels`. */
void fb_paint_line(
    uint8_t *pixels,
    uint32_t pitch,
    uint32_t width,
    uint32_t height,
    uint8_t red_shift,
    uint8_t green_shift,
    uint8_t blue_shift,
    const char *text
);
