//! Paint one text row on the mapped Swift panel.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use lerux_desk::{paint_text_row, panel_mode, Pen, FRAMEBUFFER_BYTES};

/// Register polls between dot updates. A million MMIO reads is about a second.
pub const WORKING_EVERY: u32 = 1_000_000;

const WORKING_DOTS: usize = 4;

/// A moving mark for a long bring-up poll. The status line replaces it.
pub struct WaitMark {
    armed: AtomicBool,
    tick: AtomicU32,
}

impl Default for WaitMark {
    fn default() -> Self {
        Self::new()
    }
}

impl WaitMark {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            armed: AtomicBool::new(false),
            tick: AtomicU32::new(0),
        }
    }

    pub fn arm(&self) {
        self.tick.store(0, Ordering::Relaxed);
        self.armed.store(true, Ordering::Relaxed);
    }

    pub fn disarm(&self) {
        self.armed.store(false, Ordering::Relaxed);
    }

    pub fn is_armed(&self) -> bool {
        self.armed.load(Ordering::Relaxed)
    }

    /// `Some(tick)` when the row should advance. Quiet until [`arm`](Self::arm).
    pub fn beat(&self) -> Option<u32> {
        if !self.armed.load(Ordering::Relaxed) {
            return None;
        }
        let tick = self.tick.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        tick.is_multiple_of(WORKING_EVERY).then_some(tick)
    }
}

/// `name .` through `name ....`, padded so a shorter count erases the old dots.
#[must_use]
pub fn working_line(name: &[u8], tick: u32, out: &mut [u8]) -> usize {
    let dots = (tick / WORKING_EVERY) % 4 + 1;
    let width = name.len() + 1 + WORKING_DOTS;
    if out.len() < width {
        return 0;
    }
    out[..name.len()].copy_from_slice(name);
    out[name.len()] = b' ';
    out[name.len() + 1] = b'.';
    out[name.len() + 2] = if dots > 1 { b'.' } else { b' ' };
    out[name.len() + 3] = if dots > 2 { b'.' } else { b' ' };
    out[name.len() + 4] = if dots > 3 { b'.' } else { b' ' };
    width
}

pub fn paint_row(record: &[u8], frame: &mut [u8], row: usize, text: &[u8]) {
    let Some(mode) = panel_mode(record) else {
        return;
    };
    let bytes = u64::from(mode.pitch()).saturating_mul(u64::from(mode.height()));
    if bytes > FRAMEBUFFER_BYTES {
        return;
    }
    let bytes = fit_panel_bytes(bytes);
    if frame.len() < bytes {
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

fn fit_panel_bytes(bytes: u64) -> usize {
    usize::try_from(bytes).expect("panel bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_line_cycles_four_dots() {
        let mut out = [0u8; 16];
        let n = working_line(b"mt7921", 0, &mut out);
        assert_eq!(&out[..n], b"mt7921 .   ");
        let n = working_line(b"mt7921", WORKING_EVERY, &mut out);
        assert_eq!(&out[..n], b"mt7921 ..  ");
        let n = working_line(b"mt7921", WORKING_EVERY * 3, &mut out);
        assert_eq!(&out[..n], b"mt7921 ....");
        let n = working_line(b"mt7921", WORKING_EVERY * 4, &mut out);
        assert_eq!(&out[..n], b"mt7921 .   ");
    }

    #[test]
    fn wait_mark_advances_on_the_interval() {
        let mark = WaitMark::new();
        assert!(mark.beat().is_none());
        mark.arm();
        for _ in 1..WORKING_EVERY {
            assert!(mark.beat().is_none());
        }
        assert_eq!(mark.beat(), Some(WORKING_EVERY));
        mark.disarm();
        assert!(mark.beat().is_none());
    }
}
