//! Table text from iwd 3.12 `client/display.c`.
//!
//! `LINE_LEN` is 81, so the rule is 80 dashes. The caption is centered with
//! that file's integer formula. Color escapes are left out: the console is a
//! cell grid.

pub(crate) struct Out<'a> {
    buf: &'a mut [u8],
    at: usize,
    pub truncated: bool,
}

impl<'a> Out<'a> {
    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            at: 0,
            truncated: false,
        }
    }

    pub(crate) fn written(&self) -> usize {
        self.at
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) {
        let room = self.buf.len().saturating_sub(self.at);
        let n = bytes.len().min(room);
        self.buf[self.at..self.at + n].copy_from_slice(&bytes[..n]);
        self.at += n;
        if n < bytes.len() {
            self.truncated = true;
        }
    }

    pub(crate) fn byte(&mut self, byte: u8) {
        self.push(&[byte]);
    }

    pub(crate) fn spaces(&mut self, n: usize) {
        for _ in 0..n {
            self.byte(b' ');
        }
    }

    pub(crate) fn nl(&mut self) {
        self.byte(b'\n');
    }
}

pub(crate) fn caption(out: &mut Out<'_>, title: &[u8]) {
    let pos = 40 + title.len() / 2;
    let before = pos.saturating_sub(title.len());
    out.spaces(before);
    out.push(title);
    out.spaces(79usize.saturating_sub(pos));
    out.nl();
    rule(out);
}

pub(crate) fn rule(out: &mut Out<'_>) {
    for _ in 0..80 {
        out.byte(b'-');
    }
    out.nl();
}

pub(crate) fn footer(out: &mut Out<'_>) {
    out.nl();
}

/// Header row. The format string in iwd puts the separator between fields.
pub(crate) fn header(out: &mut Out<'_>, cols: &[Col<'_>]) {
    out.push(b"  ");
    for (index, col) in cols.iter().enumerate() {
        field(out, col.text, col.width, col.align);
        if index + 1 != cols.len() {
            out.push(b"  ");
        }
    }
    out.nl();
}

pub(crate) struct Col<'a> {
    pub text: &'a [u8],
    pub width: usize,
    pub align: Align,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Left,
    Right,
}

/// One data row. `display_table_row` pads every column and then writes two spaces.
pub(crate) fn row(out: &mut Out<'_>, cols: &[(&[u8], usize)]) {
    out.push(b"  ");
    for (text, width) in cols {
        field(out, text, *width, Align::Left);
        out.push(b"  ");
    }
    out.nl();
}

fn field(out: &mut Out<'_>, text: &[u8], width: usize, align: Align) {
    let pad = width.saturating_sub(text.len());
    if align == Align::Right {
        out.spaces(pad);
    }
    out.push(text);
    if align == Align::Left && text.len() < width {
        out.spaces(pad);
    }
}

pub(crate) fn mac(out: &mut Out<'_>, addr: [u8; 6]) {
    for (index, byte) in addr.iter().enumerate() {
        if index > 0 {
            out.byte(b':');
        }
        out.byte(hex(byte >> 4));
        out.byte(hex(*byte));
    }
}

fn hex(nibble: u8) -> u8 {
    let n = nibble & 0xf;
    if n < 10 {
        b'0' + n
    } else {
        b'a' + (n - 10)
    }
}

pub(crate) fn mac_text(addr: [u8; 6]) -> [u8; 17] {
    let mut dest = [0u8; 17];
    let mut out = Out::new(&mut dest);
    mac(&mut out, addr);
    dest
}
