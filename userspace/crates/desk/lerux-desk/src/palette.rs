//! Linux 6.12 console colours.
//!
//! `color_table` turns an ANSI index into a VGA hardware index. `rgb_foreground`
//! writes that hardware index directly, then bold flips bit 3. The DAC colours
//! are the ones `vgacon` programs: `default_red`/`grn`/`blu` stored at
//! `color_table[i]`. One attribute byte is that hardware index, so a VGA cell
//! and a Swift pixel name the same colour.

/// ANSI 0–7 to the VGA hardware index. Bright colours continue the same table.
const COLOR_TABLE: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];

/// VGA hardware index to red, green, blue. Index 7 is `#aaaaaa`. Index 11 is `#55ffff`.
const VGA_RGB: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xaa],
    [0x00, 0xaa, 0x00],
    [0x00, 0xaa, 0xaa],
    [0xaa, 0x00, 0x00],
    [0xaa, 0x00, 0xaa],
    [0xaa, 0x55, 0x00],
    [0xaa, 0xaa, 0xaa],
    [0x55, 0x55, 0x55],
    [0x55, 0x55, 0xff],
    [0x55, 0xff, 0x55],
    [0x55, 0xff, 0xff],
    [0xff, 0x55, 0x55],
    [0xff, 0x55, 0xff],
    [0xff, 0xff, 0x55],
    [0xff, 0xff, 0xff],
];

/// Light gray on black. The high nibble is the background hardware index.
pub const DEFAULT_ATTR: u8 = 0x07;

/// Foreground and background before bold is folded into the attribute byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellAttr {
    fg: u8,
    bg: u8,
    bold: bool,
}

impl CellAttr {
    pub const DEFAULT: Self = Self {
        fg: 7,
        bg: 0,
        bold: false,
    };

    /// VGA attribute byte. Bold flips foreground bit 3, the same xor as `build_attr`.
    #[must_use]
    pub fn bits(self) -> u8 {
        let mut fg = self.fg & 0x0f;
        if self.bold {
            fg ^= 0x08;
        }
        ((self.bg & 0x0f) << 4) | (fg & 0x0f)
    }

    /// Apply one CSI `m` parameter list. An empty list is reset.
    pub fn apply_sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            *self = Self::DEFAULT;
            return;
        }
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => *self = Self::DEFAULT,
                1 => self.bold = true,
                22 => self.bold = false,
                30..=37 => self.fg = COLOR_TABLE[(params[i] - 30) as usize],
                40..=47 => self.bg = COLOR_TABLE[(params[i] - 40) as usize],
                90..=97 => {
                    self.bold = true;
                    self.fg = COLOR_TABLE[(params[i] - 90) as usize];
                }
                100..=107 => self.bg = COLOR_TABLE[(params[i] - 100) as usize],
                39 => self.fg = 7,
                49 => self.bg = 0,
                38 | 48 => {
                    let foreground = params[i] == 38;
                    i = self.truecolor(params, i, foreground);
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// ITU T.416. Returns the index of the last parameter this sequence consumed.
    fn truecolor(&mut self, params: &[u16], i: usize, foreground: bool) -> usize {
        let next = i + 1;
        if next >= params.len() {
            return i;
        }
        if params[next] == 5 && next + 1 < params.len() {
            let mut rgb = [0u8; 3];
            rgb_from_256(params[next + 1], &mut rgb);
            self.set_rgb(rgb, foreground);
            return next + 1;
        }
        if params[next] == 2 && next + 3 < params.len() {
            let rgb = [
                channel(params[next + 1]),
                channel(params[next + 2]),
                channel(params[next + 3]),
            ];
            self.set_rgb(rgb, foreground);
            return next + 3;
        }
        next
    }

    fn set_rgb(&mut self, rgb: [u8; 3], foreground: bool) {
        if foreground {
            let (hue, bold) = rgb_foreground(rgb);
            self.fg = hue;
            self.bold = bold;
        } else {
            self.bg = rgb_background(rgb);
        }
    }
}

fn channel(value: u16) -> u8 {
    u8::try_from(value.min(255)).unwrap_or(255)
}

/// `rgb_foreground`: hue bits are already the VGA order, and a bright sample sets bold.
fn rgb_foreground(rgb: [u8; 3]) -> (u8, bool) {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let half = max / 2;
    let mut hue = 0u8;
    if rgb[0] > half {
        hue |= 4;
    }
    if rgb[1] > half {
        hue |= 2;
    }
    if rgb[2] > half {
        hue |= 1;
    }
    if hue == 7 && max <= 0x55 {
        (0, true)
    } else {
        (hue, max > 0xaa)
    }
}

/// `rgb_background`: a channel at or above `0x80` sets the matching VGA bit.
fn rgb_background(rgb: [u8; 3]) -> u8 {
    let mut hue = 0u8;
    if rgb[0] & 0x80 != 0 {
        hue |= 4;
    }
    if rgb[1] & 0x80 != 0 {
        hue |= 2;
    }
    if rgb[2] & 0x80 != 0 {
        hue |= 1;
    }
    hue
}

fn rgb_from_256(index: u16, rgb: &mut [u8; 3]) {
    let index = usize::from(index.min(255));
    if index < 8 {
        rgb[0] = if index & 1 != 0 { 0xaa } else { 0 };
        rgb[1] = if index & 2 != 0 { 0xaa } else { 0 };
        rgb[2] = if index & 4 != 0 { 0xaa } else { 0 };
    } else if index < 16 {
        rgb[0] = if index & 1 != 0 { 0xff } else { 0x55 };
        rgb[1] = if index & 2 != 0 { 0xff } else { 0x55 };
        rgb[2] = if index & 4 != 0 { 0xff } else { 0x55 };
    } else if index < 232 {
        let mut i = index - 16;
        rgb[2] = u8::try_from(i % 6 * 255 / 6).unwrap_or(0);
        i /= 6;
        rgb[1] = u8::try_from(i % 6 * 255 / 6).unwrap_or(0);
        i /= 6;
        rgb[0] = u8::try_from(i * 255 / 6).unwrap_or(0);
    } else {
        let gray = u8::try_from(index * 10 - 2312).unwrap_or(0);
        *rgb = [gray, gray, gray];
    }
}

/// Red, green, blue for a VGA hardware index.
#[must_use]
pub fn vga_rgb(index: u8) -> [u8; 3] {
    VGA_RGB[usize::from(index & 0x0f)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_title_quantizes_to_light_cyan() {
        let mut attr = CellAttr::DEFAULT;
        attr.apply_sgr(&[38, 2, 23, 147, 209]);
        assert_eq!(attr.bits() & 0x0f, 11);
        assert_eq!(vga_rgb(attr.bits() & 0x0f), [0x55, 0xff, 0xff]);
    }

    #[test]
    fn sgr_35_is_magenta() {
        let mut attr = CellAttr::DEFAULT;
        attr.apply_sgr(&[35]);
        assert_eq!(vga_rgb(attr.bits() & 0x0f), [0xaa, 0x00, 0xaa]);
    }

    #[test]
    fn logo_red_is_hardware_red() {
        let mut attr = CellAttr::DEFAULT;
        attr.apply_sgr(&[41]);
        assert_eq!(attr.bits() >> 4, 4);
        assert_eq!(vga_rgb(4), [0xaa, 0x00, 0x00]);
    }

    #[test]
    fn logo_yellow_is_vga_brown() {
        let mut attr = CellAttr::DEFAULT;
        attr.apply_sgr(&[43]);
        assert_eq!(vga_rgb(attr.bits() >> 4), [0xaa, 0x55, 0x00]);
    }

    #[test]
    fn default_cell_is_gray_on_black() {
        assert_eq!(CellAttr::DEFAULT.bits(), DEFAULT_ATTR);
        assert_eq!(vga_rgb(DEFAULT_ATTR & 0x0f), [0xaa, 0xaa, 0xaa]);
        assert_eq!(vga_rgb(DEFAULT_ATTR >> 4), [0x00, 0x00, 0x00]);
    }
}
