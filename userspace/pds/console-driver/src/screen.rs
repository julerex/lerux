//! Text cells for the console.
//!
//! The VGA boards are the 80×25 hardware page. The Swift panel is a 240×40 band
//! of 8×16 cells. Both store one Linux attribute byte in the high half of the cell.

use lerux_desk::{CellAttr, DEFAULT_ATTR};

/// Columns. The Swift band is 240; a VGA page is 80.
#[cfg(feature = "board-swift_sfx14_41g")]
pub(crate) const COLS: usize = lerux_desk::CONSOLE_COLS;
#[cfg(not(feature = "board-swift_sfx14_41g"))]
pub(crate) const COLS: usize = 80;
/// Rows. The Swift band matches [`lerux_desk::CONSOLE_ROWS`]. A VGA page is 25.
#[cfg(feature = "board-swift_sfx14_41g")]
pub(crate) const ROWS: usize = lerux_desk::CONSOLE_ROWS;
#[cfg(not(feature = "board-swift_sfx14_41g"))]
pub(crate) const ROWS: usize = 25;
/// Cells in the page.
pub const CELLS: usize = COLS * ROWS;

/// Light gray on black, in the high byte of a VGA text cell.
pub const ATTR: u16 = (DEFAULT_ATTR as u16) << 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ansi {
    Ground,
    Esc,
    Csi,
}

/// What [`Screen::apply_byte`] changed in the cell buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Damage {
    None,
    /// Half-open cell range `[start, end)`.
    Cells {
        start: usize,
        end: usize,
    },
    All,
}

/// Cursor plus the visible cells. The shell writes bytes; this applies them.
///
/// `cols` and `rows` are the page the machine is showing. The Swift panel is
/// [`COLS`] by [`ROWS`]. A VGA text page is 80 by 25. The cell buffer is the
/// larger of those, and [`Self::page`] is the visible prefix.
pub struct Screen {
    cells: [u16; CELLS],
    cols: usize,
    rows: usize,
    row: usize,
    col: usize,
    ansi: Ansi,
    attr: CellAttr,
    params: [u16; 8],
    nparams: u8,
    acc: u16,
    have_digits: bool,
}

impl Screen {
    /// Blank gray-on-black page, cursor at the origin.
    ///
    /// On the Swift build this is the panel band. [`Self::vga`] is the 80×25
    /// text page used when the probe does not see that panel.
    pub fn blank() -> Self {
        Self::with_geometry(COLS, ROWS)
    }

    /// VGA text page. The Swift image uses this when the firmware buffer is
    /// not the panel above 4 GiB.
    #[cfg(feature = "board-swift_sfx14_41g")]
    pub fn vga() -> Self {
        Self::with_geometry(80, 25)
    }

    pub(crate) fn with_geometry(cols: usize, rows: usize) -> Self {
        let used = cols.checked_mul(rows).expect("geometry fits");
        assert!(used <= CELLS, "geometry fits the cell buffer");
        Self {
            cells: blank_cells(),
            cols,
            rows,
            row: 0,
            col: 0,
            ansi: Ansi::Ground,
            attr: CellAttr::DEFAULT,
            params: [0; 8],
            nparams: 0,
            acc: 0,
            have_digits: false,
        }
    }

    /// Visible cells. The stride is the page width.
    pub fn used(&self) -> usize {
        self.cols * self.rows
    }

    #[cfg(test)]
    pub fn cells(&self) -> &[u16; CELLS] {
        &self.cells
    }

    /// The visible page. The stride is the page width.
    pub fn page(&self) -> &[u16] {
        &self.cells[..self.used()]
    }

    #[cfg(test)]
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Hardware cursor index, clamped to the last visible cell.
    pub fn cursor_index(&self) -> usize {
        (self.row * self.cols + self.col).min(self.used() - 1)
    }

    pub fn apply_byte(&mut self, byte: u8) -> Damage {
        match self.ansi {
            Ansi::Ground => self.ground(byte),
            Ansi::Esc => {
                if byte == b'[' {
                    self.begin_csi();
                } else {
                    self.ansi = Ansi::Ground;
                }
                Damage::None
            }
            Ansi::Csi => self.csi(byte),
        }
    }

    fn ground(&mut self, byte: u8) -> Damage {
        match byte {
            0x1b => {
                self.ansi = Ansi::Esc;
                Damage::None
            }
            b'\r' => {
                self.col = 0;
                Damage::None
            }
            b'\n' => {
                if self.newline() {
                    Damage::All
                } else {
                    Damage::None
                }
            }
            0x08 => {
                if self.col > 0 {
                    self.col -= 1;
                }
                Damage::None
            }
            32..=126 => self.put(byte),
            _ => Damage::None,
        }
    }

    fn begin_csi(&mut self) {
        self.ansi = Ansi::Csi;
        self.params = [0; 8];
        self.nparams = 0;
        self.acc = 0;
        self.have_digits = false;
    }

    fn push_param(&mut self) {
        if self.nparams < 8 {
            self.params[usize::from(self.nparams)] = self.acc;
            self.nparams += 1;
        }
        self.acc = 0;
        self.have_digits = false;
    }

    fn csi(&mut self, byte: u8) -> Damage {
        if byte.is_ascii_digit() {
            self.have_digits = true;
            self.acc = self
                .acc
                .saturating_mul(10)
                .saturating_add(u16::from(byte - b'0'));
            return Damage::None;
        }
        if byte == b';' {
            self.push_param();
            return Damage::None;
        }
        if self.have_digits {
            self.push_param();
        }
        let n = usize::from(self.nparams);
        let params = self.params;
        self.ansi = Ansi::Ground;
        match byte {
            b'H' => {
                self.home();
                Damage::None
            }
            b'J' if n == 1 && params[0] == 2 => {
                self.clear();
                Damage::All
            }
            b'm' => {
                if n == 0 {
                    self.attr.apply_sgr(&[]);
                } else {
                    self.attr.apply_sgr(&params[..n]);
                }
                Damage::None
            }
            _ => Damage::None,
        }
    }

    fn put(&mut self, byte: u8) -> Damage {
        let scrolled = if self.col >= self.cols {
            self.newline()
        } else {
            false
        };
        let index = self.row * self.cols + self.col;
        self.cells[index] = cell(self.attr, byte);
        self.col += 1;
        if scrolled {
            Damage::All
        } else {
            Damage::Cells {
                start: index,
                end: index + 1,
            }
        }
    }

    /// Advance a line. Returns whether the page scrolled.
    fn newline(&mut self) -> bool {
        self.col = 0;
        if self.row + 1 >= self.rows {
            self.scroll();
            true
        } else {
            self.row += 1;
            false
        }
    }

    fn scroll(&mut self) {
        let cols = self.cols;
        let used = self.used();
        self.cells.copy_within(cols..used, 0);
        let fill = cell(self.attr, b' ');
        for slot in &mut self.cells[used - cols..used] {
            *slot = fill;
        }
        self.row = self.rows - 1;
        self.col = 0;
    }

    fn clear(&mut self) {
        self.cells.fill(cell(self.attr, b' '));
        self.home();
    }

    fn home(&mut self) {
        self.row = 0;
        self.col = 0;
    }
}

fn cell(attr: CellAttr, byte: u8) -> u16 {
    (u16::from(attr.bits()) << 8) | u16::from(byte)
}

fn blank() -> u16 {
    ATTR | u16::from(b' ')
}

fn blank_cells() -> [u16; CELLS] {
    let fill = blank();
    // `from_fn` rather than `[fill; CELLS]`: the Swift band is 19 KiB, and a
    // repeat expression of that size is a stack temporary clippy rejects.
    core::array::from_fn(|_| fill)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_land_on_the_first_line() {
        let mut screen = Screen::blank();
        for &byte in b"lerux> " {
            screen.apply_byte(byte);
        }
        for (offset, &byte) in b"lerux> ".iter().enumerate() {
            assert_eq!(screen.cells()[offset], blank_with(byte));
        }
        assert_eq!(screen.cursor(), (0, 7));
    }

    #[test]
    fn backspace_sequence_erases_a_character() {
        let mut screen = Screen::blank();
        screen.apply_byte(b'e');
        screen.apply_byte(0x08);
        screen.apply_byte(b' ');
        screen.apply_byte(0x08);
        assert_eq!(screen.cells()[0], blank());
        assert_eq!(screen.cursor(), (0, 0));
    }

    #[test]
    fn clear_sequence_homes_and_wipes() {
        let mut screen = Screen::blank();
        screen.apply_byte(b'e');
        for &byte in b"\x1b[2J\x1b[H" {
            screen.apply_byte(byte);
        }
        assert!(screen.cells().iter().all(|cell| *cell == blank()));
        assert_eq!(screen.cursor(), (0, 0));
    }

    #[test]
    fn newline_off_the_last_row_scrolls() {
        let mut screen = Screen::blank();
        screen.apply_byte(b'X');
        for _ in 0..ROWS {
            screen.apply_byte(b'\n');
        }
        assert_eq!(screen.cursor(), (ROWS - 1, 0));
        assert_eq!(screen.cells()[0], blank());
        assert_eq!(screen.cells()[CELLS - 1], blank());
    }

    #[test]
    fn truecolor_title_is_nibble_11() {
        let mut screen = Screen::blank();
        for &byte in b"\x1b[38;2;23;147;209mA" {
            screen.apply_byte(byte);
        }
        assert_eq!(screen.cells()[0] >> 8, 0x0b);
        assert_eq!(screen.cells()[0] & 0xff, u16::from(b'A'));
    }

    #[test]
    fn sgr_35_and_logo_red_share_one_attribute_byte() {
        let mut screen = Screen::blank();
        for &byte in b"\x1b[35mi\x1b[0m\x1b[41m " {
            screen.apply_byte(byte);
        }
        assert_eq!(screen.cells()[0] >> 8, 0x05);
        assert_eq!(screen.cells()[1] >> 8, 0x47);
    }

    #[test]
    fn vga_width_wraps_at_eighty() {
        let mut screen = Screen::with_geometry(80, 25);
        for _ in 0..80 {
            screen.apply_byte(b'a');
        }
        screen.apply_byte(b'b');
        assert_eq!(screen.cursor(), (1, 1));
        assert_eq!(screen.page()[80] & 0xff, u16::from(b'b'));
        assert_eq!(screen.used(), 80 * 25);
    }

    #[test]
    fn preamble_leaves_the_cursor_on_the_prompt() {
        let mut screen = Screen::blank();
        lerux_signon::for_each_preamble(b"test", |part| {
            for &byte in part {
                screen.apply_byte(byte);
            }
        });
        let (row, col) = screen.cursor();
        assert_eq!(col, lerux_signon::PROMPT.len());
        assert_eq!(screen.cursor_index(), row * COLS + col);
        let start = row * COLS;
        for (offset, &byte) in lerux_signon::PROMPT.iter().enumerate() {
            assert_eq!(screen.cells()[start + offset] & 0xff, u16::from(byte));
            assert_eq!(screen.cells()[start + offset] >> 8, u16::from(DEFAULT_ATTR));
        }
        let arch = find(&screen, b"Arch Linux").expect("title cells");
        assert_eq!(screen.cells()[arch] >> 8, 0x0b);
        let iwctl = find(&screen, b"iwctl").expect("utility cells");
        assert_eq!(screen.cells()[iwctl] >> 8, 0x05);
        assert!(
            screen
                .cells()
                .iter()
                .any(|cell| *cell >> 8 == 0x47 && *cell & 0xff == u16::from(b' ')),
            "a logo cell is red on the hardware index"
        );
    }

    fn blank_with(byte: u8) -> u16 {
        ATTR | u16::from(byte)
    }

    fn find(screen: &Screen, word: &[u8]) -> Option<usize> {
        screen.cells().windows(word.len()).position(|window| {
            window
                .iter()
                .zip(word)
                .all(|(cell, &byte)| (*cell & 0xff) as u8 == byte)
        })
    }
}
