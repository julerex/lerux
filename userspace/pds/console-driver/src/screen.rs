//! 80×25 VGA text cells. Hardware stores stay outside this module.

/// Columns in the VGA text page Limine leaves behind.
pub const COLS: usize = 80;
/// Rows in that page.
pub const ROWS: usize = 25;
/// Cells in the page. 80×25 occupies 4000 bytes of the 4 KiB map.
pub const CELLS: usize = COLS * ROWS;

/// Bright white on blue, in the high byte of a VGA text cell.
pub const ATTR: u16 = 0x1f00;

/// Painted on row 0 before the shell prompt.
pub const BANNER: &[u8] = b"hello lerux";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ansi {
    Ground,
    Esc,
    Bracket,
    Param(u8),
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
pub struct Screen {
    cells: [u16; CELLS],
    row: usize,
    col: usize,
    ansi: Ansi,
}

impl Screen {
    /// Blue field, `hello lerux` on the first line, cursor on the next line.
    pub fn with_banner() -> Self {
        let mut screen = Self::blank();
        for (index, &byte) in BANNER.iter().enumerate() {
            screen.cells[index] = cell(byte);
        }
        screen.row = 1;
        screen
    }

    fn blank() -> Self {
        Self {
            cells: [blank(); CELLS],
            row: 0,
            col: 0,
            ansi: Ansi::Ground,
        }
    }

    pub fn cells(&self) -> &[u16; CELLS] {
        &self.cells
    }

    #[cfg(test)]
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Hardware cursor index, clamped to the last cell.
    pub fn cursor_index(&self) -> usize {
        (self.row * COLS + self.col).min(CELLS - 1)
    }

    pub fn apply_byte(&mut self, byte: u8) -> Damage {
        match self.ansi {
            Ansi::Ground => self.ground(byte),
            Ansi::Esc => {
                self.ansi = if byte == b'[' {
                    Ansi::Bracket
                } else {
                    Ansi::Ground
                };
                Damage::None
            }
            Ansi::Bracket => self.bracket(byte),
            Ansi::Param(n) => self.param(n, byte),
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

    fn bracket(&mut self, byte: u8) -> Damage {
        if byte.is_ascii_digit() {
            self.ansi = Ansi::Param(byte - b'0');
            return Damage::None;
        }
        self.ansi = Ansi::Ground;
        if byte == b'H' {
            self.home();
        }
        Damage::None
    }

    fn param(&mut self, n: u8, byte: u8) -> Damage {
        if byte.is_ascii_digit() {
            self.ansi = Ansi::Param(byte - b'0');
            return Damage::None;
        }
        self.ansi = Ansi::Ground;
        if byte == b'H' {
            self.home();
            return Damage::None;
        }
        if byte == b'J' && n == 2 {
            self.clear();
            return Damage::All;
        }
        Damage::None
    }

    fn put(&mut self, byte: u8) -> Damage {
        let scrolled = if self.col >= COLS {
            self.newline()
        } else {
            false
        };
        let index = self.row * COLS + self.col;
        self.cells[index] = cell(byte);
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
        if self.row + 1 >= ROWS {
            self.scroll();
            true
        } else {
            self.row += 1;
            false
        }
    }

    fn scroll(&mut self) {
        self.cells.copy_within(COLS.., 0);
        for slot in &mut self.cells[CELLS - COLS..] {
            *slot = blank();
        }
        self.row = ROWS - 1;
        self.col = 0;
    }

    fn clear(&mut self) {
        self.cells = [blank(); CELLS];
        self.home();
    }

    fn home(&mut self) {
        self.row = 0;
        self.col = 0;
    }
}

fn cell(byte: u8) -> u16 {
    ATTR | u16::from(byte)
}

fn blank() -> u16 {
    cell(b' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_sits_above_the_prompt_line() {
        let screen = Screen::with_banner();
        for (index, &byte) in BANNER.iter().enumerate() {
            assert_eq!(screen.cells()[index], cell(byte));
        }
        assert_eq!(screen.cells()[BANNER.len()], blank());
        assert_eq!(screen.cursor(), (1, 0));
        assert_eq!(screen.cursor_index(), COLS);
    }

    #[test]
    fn bytes_land_on_the_second_line() {
        let mut screen = Screen::with_banner();
        for &byte in b"lerux> " {
            screen.apply_byte(byte);
        }
        let start = COLS;
        for (offset, &byte) in b"lerux> ".iter().enumerate() {
            assert_eq!(screen.cells()[start + offset], cell(byte));
        }
        assert_eq!(screen.cursor(), (1, 7));
    }

    #[test]
    fn backspace_sequence_erases_a_character() {
        let mut screen = Screen::with_banner();
        screen.apply_byte(b'e');
        // The shell sends backspace, space, backspace.
        screen.apply_byte(0x08);
        screen.apply_byte(b' ');
        screen.apply_byte(0x08);
        assert_eq!(screen.cells()[COLS], blank());
        assert_eq!(screen.cursor(), (1, 0));
    }

    #[test]
    fn clear_sequence_homes_and_wipes_the_banner() {
        let mut screen = Screen::with_banner();
        for &byte in b"\x1b[2J\x1b[H" {
            screen.apply_byte(byte);
        }
        assert!(screen.cells().iter().all(|cell| *cell == blank()));
        assert_eq!(screen.cursor(), (0, 0));
    }

    #[test]
    fn newline_off_the_last_row_scrolls_the_banner_away() {
        let mut screen = Screen::with_banner();
        // Cursor starts on row 1. Twenty-four newlines reach row 24 and scroll.
        for _ in 0..24 {
            screen.apply_byte(b'\n');
        }
        assert_eq!(screen.cursor(), (ROWS - 1, 0));
        assert_eq!(screen.cells()[0], blank());
        assert_eq!(screen.cells()[CELLS - 1], blank());
    }
}
