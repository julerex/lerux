//! Copy each boot log line into the Z97 firmware buffer before the five-second pause.
//!
//! The shell later replaces those rows with the sign-on. A fault during
//! bring-up leaves the last line on the monitor for that pause. The hook is
//! installed only after the frame exists, and it does not touch `0xb8000`.

use core::sync::atomic::{AtomicPtr, AtomicU8, AtomicUsize, Ordering};

use lerux_desk::{paint_text_row, Pen, DESK_FB_BYTES};

use crate::screen::{COLS, ROWS};

static FRAME: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static PITCH: AtomicUsize = AtomicUsize::new(0);
static WIDTH: AtomicUsize = AtomicUsize::new(0);
static HEIGHT: AtomicUsize = AtomicUsize::new(0);
static RED: AtomicU8 = AtomicU8::new(0);
static GREEN: AtomicU8 = AtomicU8::new(0);
static BLUE: AtomicU8 = AtomicU8::new(0);
static ROW: AtomicUsize = AtomicUsize::new(0);

pub fn install(frame: *mut u8, pen: Pen) {
    FRAME.store(frame, Ordering::Relaxed);
    PITCH.store(pen.pitch, Ordering::Relaxed);
    WIDTH.store(pen.width, Ordering::Relaxed);
    HEIGHT.store(pen.height, Ordering::Relaxed);
    RED.store(pen.red, Ordering::Relaxed);
    GREEN.store(pen.green, Ordering::Relaxed);
    BLUE.store(pen.blue, Ordering::Relaxed);
    lerux_logging::debug::set_boot_line_hook(paint);
}

fn paint(line: &str) {
    let frame = FRAME.load(Ordering::Relaxed);
    if frame.is_null() {
        return;
    }
    let row = ROW.fetch_add(1, Ordering::Relaxed);
    if row >= ROWS {
        return;
    }
    let bytes = row_bytes(line);
    let pen = Pen {
        pitch: PITCH.load(Ordering::Relaxed),
        width: WIDTH.load(Ordering::Relaxed),
        height: HEIGHT.load(Ordering::Relaxed),
        red: RED.load(Ordering::Relaxed),
        green: GREEN.load(Ordering::Relaxed),
        blue: BLUE.load(Ordering::Relaxed),
    };
    let len = usize::try_from(DESK_FB_BYTES).expect("desk frame fits");
    // SAFETY: `install` stored the `desk_fb` mapping, and this hook runs on
    // the root after that store. The slice is that mapping.
    let pixels = unsafe { core::slice::from_raw_parts_mut(frame, len) };
    paint_text_row(pixels, pen, row, &bytes);
}

fn row_bytes(line: &str) -> [u8; COLS] {
    let mut bytes = [b' '; COLS];
    for (slot, byte) in bytes.iter_mut().zip(line.as_bytes()) {
        *slot = if byte.is_ascii_graphic() || *byte == b' ' {
            *byte
        } else {
            b' '
        };
    }
    bytes
}
