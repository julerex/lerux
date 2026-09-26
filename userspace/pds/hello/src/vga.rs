//! VGA 80×25 text banner for the PC hello board.
//!
//! Limine's legacy entry leaves color text mode at physical `0xb8000`. The
//! system file maps that page uncached. Serial output is unchanged.

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{memory_region_symbol, var};

/// Cells in the 80×25 text buffer.
pub const CELLS: usize = 80 * 25;

/// `hello lerux`, painted at the top left.
pub const BANNER: &[u8] = b"hello lerux";

/// Bright white on blue, in the high byte of a VGA text cell.
const ATTR: u16 = 0x1f00;

/// First IOPort cap slot assigned by Microkit (same base as the serial driver).
const BASE_IOPORT_CAP: u64 = 394;

/// CRTC cursor-start register. Bit 5 hides the hardware cursor.
const CURSOR_START: u8 = 0x0a;
const CURSOR_DISABLE: u8 = 0x20;

/// Fill `cells` with a blue background and write `text` from the first cell.
pub fn paint_cells(cells: &mut [u16], text: &[u8]) {
    let blank = ATTR | u16::from(b' ');
    for cell in cells.iter_mut() {
        *cell = blank;
    }
    for (cell, &byte) in cells.iter_mut().zip(text) {
        *cell = ATTR | u16::from(byte);
    }
}

pub fn paint_hello() {
    let mut cells = [0u16; CELLS];
    paint_cells(&mut cells, BANNER);
    unsafe {
        // The map is one 4KiB page; 80×25 cells occupy the first 4000 bytes.
        let ptr = memory_region_symbol!(vga_text_vaddr: *mut u16).as_ptr();
        for (i, cell) in cells.iter().enumerate() {
            ptr.add(i).write_volatile(*cell);
        }
    }
    hide_cursor();
}

fn hide_cursor() {
    let ioport_id = *var!(vga_ioport_id: usize = usize::MAX);
    let index_port = *var!(vga_ioport_addr: usize = usize::MAX) as u16;
    let cap = BASE_IOPORT_CAP + ioport_id as u64;
    out8(cap, index_port, CURSOR_START);
    out8(cap, index_port + 1, CURSOR_DISABLE);
}

fn out8(cap: u64, port: u16, value: u8) {
    with_ipc_buffer_mut(|ipc| {
        ipc.inner_mut()
            .seL4_X86_IOPort_Out8(cap, u64::from(port), u64::from(value));
    });
}

#[cfg(test)]
mod tests {
    use super::{paint_cells, ATTR, BANNER, CELLS};

    #[test]
    fn banner_is_white_on_blue_at_the_origin() {
        let mut cells = [0u16; CELLS];
        paint_cells(&mut cells, BANNER);
        for (cell, &byte) in cells.iter().zip(BANNER) {
            assert_eq!(*cell, ATTR | u16::from(byte));
        }
        assert_eq!(cells[BANNER.len()], ATTR | u16::from(b' '));
        assert_eq!(cells[CELLS - 1], ATTR | u16::from(b' '));
    }
}
