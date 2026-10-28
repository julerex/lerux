//! Copy each boot log line onto the VGA text page before the five-second pause.
//!
//! `Screen::with_banner` later replaces this page with `hello lerux`. A fault
//! during bring-up leaves the last line on the monitor for that pause.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use sel4_microkit::memory_region_symbol;

use crate::screen::{ATTR, CELLS, COLS, ROWS};

static CLEARED: AtomicBool = AtomicBool::new(false);
static ROW: AtomicUsize = AtomicUsize::new(0);

pub fn install() {
    lerux_logging::debug::set_boot_line_hook(paint);
}

fn paint(line: &str) {
    let vga = memory_region_symbol!(vga_text_vaddr: *mut u16).as_ptr();
    if !CLEARED.swap(true, Ordering::Relaxed) {
        for index in 0..CELLS {
            // SAFETY: the system map is one uncached page at this symbol.
            unsafe {
                vga.add(index).write_volatile(ATTR | u16::from(b' '));
            }
        }
    }
    let row = ROW.fetch_add(1, Ordering::Relaxed);
    if row >= ROWS {
        return;
    }
    let bytes = line.as_bytes();
    for col in 0..COLS {
        let byte = bytes.get(col).copied().unwrap_or(b' ');
        let byte = if byte.is_ascii_graphic() || byte == b' ' {
            byte
        } else {
            b' '
        };
        // SAFETY: `row` is below `ROWS`, so the cell is inside the mapped page.
        unsafe {
            vga.add(row * COLS + col)
                .write_volatile(ATTR | u16::from(byte));
        }
    }
}
