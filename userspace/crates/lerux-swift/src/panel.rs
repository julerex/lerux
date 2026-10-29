//! Paint one text row on the mapped Swift panel.

use lerux_desk::{paint_text_row, panel_mode, Pen, FRAMEBUFFER_BYTES};

pub fn paint_row(record: &[u8], frame: &mut [u8], row: usize, text: &[u8]) {
    let Some(mode) = panel_mode(record) else {
        return;
    };
    let bytes = u64::from(mode.pitch()).saturating_mul(u64::from(mode.height()));
    if bytes > FRAMEBUFFER_BYTES {
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
