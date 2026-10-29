//! Paint one text row when the mode record matches the mapped framebuffer.

use lerux_desk::{
    paint_text_row, panel_ready, Pen, ScanoutMode, FRAMEBUFFER_BYTES, FRAMEBUFFER_PHYS,
};

pub fn paint_row(record: &[u8], frame: &mut [u8], row: usize, text: &[u8]) {
    let Some(mode) = ScanoutMode::parse(record) else {
        return;
    };
    let bytes = u64::from(mode.pitch()).saturating_mul(u64::from(mode.height()));
    if !panel_ready(mode.address(), FRAMEBUFFER_PHYS) || bytes > FRAMEBUFFER_BYTES {
        return;
    }
    if frame.len() < bytes as usize {
        return;
    }
    paint_text_row(
        frame,
        Pen {
            pitch: mode.pitch() as usize,
            width: mode.width() as usize,
            height: mode.height() as usize,
            red: mode.red_shift(),
            green: mode.green_shift(),
            blue: mode.blue_shift(),
        },
        row,
        text,
    );
}
