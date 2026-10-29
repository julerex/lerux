#include "fb_paint.h"

struct glyph {
    char ch;
    uint8_t row[8];
};

/* 8x8, public-domain shapes. Bit 7 is the left pixel. */
static const struct glyph GLYPHS[] = {
    {'0', {0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00}},
    {'1', {0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00}},
    {'2', {0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00}},
    {'3', {0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00}},
    {'4', {0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0x00}},
    {'5', {0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00}},
    {'6', {0x3C, 0x66, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00}},
    {'7', {0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00}},
    {'8', {0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00}},
    {'9', {0x3C, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00}},
    {'a', {0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0x00}},
    {'b', {0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00}},
    {'c', {0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0x00}},
    {'d', {0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00}},
    {'e', {0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00}},
    {'f', {0x1C, 0x36, 0x30, 0x78, 0x30, 0x30, 0x30, 0x00}},
    {'g', {0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x7C}},
    {'h', {0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00}},
    {'i', {0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00}},
    {'p', {0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60}},
    {'r', {0x00, 0x00, 0x6C, 0x76, 0x60, 0x60, 0x60, 0x00}},
    {'t', {0x30, 0x30, 0x7C, 0x30, 0x30, 0x36, 0x1C, 0x00}},
    {'x', {0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00}},
    {' ', {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}},
};

static const uint8_t *glyph(char ch) {
    unsigned i;
    for (i = 0; i < sizeof(GLYPHS) / sizeof(GLYPHS[0]); i++) {
        if (GLYPHS[i].ch == ch) {
            return GLYPHS[i].row;
        }
    }
    return GLYPHS[sizeof(GLYPHS) / sizeof(GLYPHS[0]) - 1].row;
}

static int append(char *dst, unsigned cap, unsigned *len, const char *text) {
    while (*text) {
        if (*len + 1 >= cap) {
            return 0;
        }
        dst[(*len)++] = *text++;
    }
    return 1;
}

static int append_hex64(char *dst, unsigned cap, unsigned *len, uint64_t value) {
    static const char HEX[] = "0123456789abcdef";
    int shift;
    if (!append(dst, cap, len, "0x")) {
        return 0;
    }
    for (shift = 60; shift >= 0; shift -= 4) {
        char byte[2];
        byte[0] = HEX[(value >> shift) & 0xf];
        byte[1] = 0;
        if (!append(dst, cap, len, byte)) {
            return 0;
        }
    }
    return 1;
}

static int append_u32(char *dst, unsigned cap, unsigned *len, uint32_t value) {
    char tmp[10];
    unsigned n = 0;
    unsigned i;
    if (value == 0) {
        return append(dst, cap, len, "0");
    }
    while (value > 0 && n < sizeof(tmp)) {
        tmp[n++] = (char)('0' + (value % 10));
        value /= 10;
    }
    for (i = n; i > 0; i--) {
        char byte[2];
        byte[0] = tmp[i - 1];
        byte[1] = 0;
        if (!append(dst, cap, len, byte)) {
            return 0;
        }
    }
    return 1;
}

unsigned fb_mode_line(const struct fb_mode *mode, char *dst, unsigned cap) {
    unsigned len = 0;
    if (mode == 0 || dst == 0 || cap < 2) {
        return 0;
    }
    if (!append(dst, cap, &len, "fb ") || !append_hex64(dst, cap, &len, mode->addr)
        || !append(dst, cap, &len, " pitch ") || !append_u32(dst, cap, &len, mode->pitch)
        || !append(dst, cap, &len, " ") || !append_u32(dst, cap, &len, mode->width)
        || !append(dst, cap, &len, "x") || !append_u32(dst, cap, &len, mode->height)
        || !append(dst, cap, &len, " r") || !append_u32(dst, cap, &len, mode->red_shift)
        || !append(dst, cap, &len, " g") || !append_u32(dst, cap, &len, mode->green_shift)
        || !append(dst, cap, &len, " b") || !append_u32(dst, cap, &len, mode->blue_shift)
        || len >= cap) {
        return 0;
    }
    dst[len] = 0;
    return len;
}

static uint32_t pack(uint8_t red_shift, uint8_t green_shift, uint8_t blue_shift, int white) {
    uint32_t pixel = 0;
    if (white) {
        pixel |= 0xffu << red_shift;
        pixel |= 0xffu << green_shift;
        pixel |= 0xffu << blue_shift;
    } else {
        pixel |= 0xaau << blue_shift;
    }
    return pixel;
}

void fb_paint_line(
    uint8_t *pixels,
    uint32_t pitch,
    uint32_t width,
    uint32_t height,
    uint8_t red_shift,
    uint8_t green_shift,
    uint8_t blue_shift,
    const char *text
) {
    uint32_t fg = pack(red_shift, green_shift, blue_shift, 1);
    uint32_t bg = pack(red_shift, green_shift, blue_shift, 0);
    uint32_t x;
    uint32_t y;
    const char *cursor;
    if (pixels == 0 || text == 0 || pitch < 4 || width < 16 || height < 16) {
        return;
    }
    for (y = 0; y < 16 && y < height; y++) {
        uint8_t *row = pixels + (uint64_t)y * pitch;
        for (x = 0; x + 4 <= pitch && x / 4 < width; x += 4) {
            row[x] = (uint8_t)bg;
            row[x + 1] = (uint8_t)(bg >> 8);
            row[x + 2] = (uint8_t)(bg >> 16);
            row[x + 3] = (uint8_t)(bg >> 24);
        }
    }
    x = 0;
    for (cursor = text; *cursor && (x + 16) / 4 < width; cursor++) {
        const uint8_t *rows = glyph(*cursor);
        uint32_t gy;
        for (gy = 0; gy < 8; gy++) {
            uint32_t gx;
            for (gx = 0; gx < 8; gx++) {
                int on = (rows[gy] & (0x80u >> gx)) != 0;
                uint32_t color = on ? fg : bg;
                uint32_t px = x + gx * 2;
                uint32_t py = gy * 2;
                uint32_t sy;
                uint32_t sx;
                for (sy = 0; sy < 2; sy++) {
                    for (sx = 0; sx < 2; sx++) {
                        uint64_t byte =
                            (uint64_t)(py + sy) * pitch + (uint64_t)(px + sx) * 4;
                        if (byte + 4 <= (uint64_t)pitch * height) {
                            pixels[byte] = (uint8_t)color;
                            pixels[byte + 1] = (uint8_t)(color >> 8);
                            pixels[byte + 2] = (uint8_t)(color >> 16);
                            pixels[byte + 3] = (uint8_t)(color >> 24);
                        }
                    }
                }
            }
        }
        x += 16;
    }
}
