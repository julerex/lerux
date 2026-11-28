#include <stdio.h>

#include "fb_paint.h"

int main(void) {
    struct fb_mode mode = {
        .addr = 0xd0000000ull,
        .pitch = 4096,
        .width = 1024,
        .height = 768,
        .red_shift = 16,
        .green_shift = 8,
        .blue_shift = 0,
    };
    char line[80];
    uint8_t pixels[16 * 64];
    unsigned len = fb_mode_line(&mode, line, sizeof(line));
    if (len == 0) {
        return 1;
    }
    puts(line);
    fb_paint_line(pixels, 64, 16, 16, 16, 8, 0, "f");
    /* Glyph 0x1C has ink at column 3. Red sits at bit 16, so byte 26. */
    if (pixels[26] != 0xff) {
        return 2;
    }
    return 0;
}
