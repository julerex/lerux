//! Token split for one `iwctl` line.
//!
//! Double quotes keep a space inside one word. Options that take a value match
//! iwd 3.12 `client/iwctl.rst`: `--passphrase` / `-P`, `--username` / `-u`,
//! `--password` / `-p`, `--dont-ask` / `-v`, and `--help` / `-h`.

const TOKEN_MAX: usize = 80;
const TOKEN_COUNT: usize = 12;

pub(crate) struct Parsed {
    pub help: bool,
    pub dont_ask: bool,
    pub pass: Option<Pass>,
    pub words: [[u8; TOKEN_MAX]; TOKEN_COUNT],
    pub lens: [u8; TOKEN_COUNT],
    pub count: u8,
}

#[derive(Clone, Copy)]
pub(crate) struct Pass {
    pub bytes: [u8; 64],
    pub len: u8,
}

impl Parsed {
    pub(crate) fn word(&self, index: usize) -> &[u8] {
        let n = usize::from(self.lens[index]);
        &self.words[index][..n]
    }
}

pub(crate) fn words_only(line: &[u8]) -> Option<Parsed> {
    let mut raw = [[0u8; TOKEN_MAX]; TOKEN_COUNT];
    let mut raw_len = [0u8; TOKEN_COUNT];
    let n = split(line, &mut raw, &mut raw_len)?;
    let mut out = empty();
    copy_words(&raw, &raw_len, 0, n, &mut out);
    Some(out)
}

pub(crate) fn parse(line: &[u8]) -> Option<Parsed> {
    let mut raw = [[0u8; TOKEN_MAX]; TOKEN_COUNT];
    let mut raw_len = [0u8; TOKEN_COUNT];
    let n = split(line, &mut raw, &mut raw_len)?;
    let mut out = empty();
    let mut i = 0;
    while i < n {
        if take_option(&raw, &raw_len, n, &mut i, &mut out)? {
            continue;
        }
        break;
    }
    copy_words(&raw, &raw_len, i, n, &mut out);
    Some(out)
}

fn take_option(
    raw: &[[u8; TOKEN_MAX]; TOKEN_COUNT],
    raw_len: &[u8; TOKEN_COUNT],
    n: usize,
    i: &mut usize,
    out: &mut Parsed,
) -> Option<bool> {
    let word = token(raw, raw_len, *i);
    if word == b"--" {
        *i += 1;
        return Some(false);
    }
    if word == b"--help" || word == b"-h" {
        out.help = true;
        *i += 1;
        return Some(true);
    }
    if word == b"--dont-ask" || word == b"-v" {
        out.dont_ask = true;
        *i += 1;
        return Some(true);
    }
    if let Some(value) = word.strip_prefix(b"--passphrase=") {
        out.pass = Some(pass_from(value)?);
        *i += 1;
        return Some(true);
    }
    if word == b"--passphrase" || word == b"-P" {
        *i += 1;
        if *i >= n {
            return None;
        }
        out.pass = Some(pass_from(token(raw, raw_len, *i))?);
        *i += 1;
        return Some(true);
    }
    if word == b"--username" || word == b"-u" || word == b"--password" || word == b"-p" {
        *i += 1;
        if *i >= n {
            return None;
        }
        *i += 1;
        return Some(true);
    }
    if word.first() == Some(&b'-') {
        return None;
    }
    Some(false)
}

fn empty() -> Parsed {
    Parsed {
        help: false,
        dont_ask: false,
        pass: None,
        words: [[0u8; TOKEN_MAX]; TOKEN_COUNT],
        lens: [0u8; TOKEN_COUNT],
        count: 0,
    }
}

fn pass_from(value: &[u8]) -> Option<Pass> {
    if value.len() > 64 {
        return None;
    }
    let mut bytes = [0u8; 64];
    bytes[..value.len()].copy_from_slice(value);
    let len = u8::try_from(value.len()).ok()?;
    Some(Pass { bytes, len })
}

fn copy_words(
    raw: &[[u8; TOKEN_MAX]; TOKEN_COUNT],
    raw_len: &[u8; TOKEN_COUNT],
    from: usize,
    n: usize,
    out: &mut Parsed,
) {
    let mut at = from;
    while at < n {
        let len = raw_len[at];
        let nlen = usize::from(len);
        out.words[out.count as usize][..nlen].copy_from_slice(&raw[at][..nlen]);
        out.lens[out.count as usize] = len;
        out.count += 1;
        at += 1;
    }
}

fn split(line: &[u8], words: &mut [[u8; TOKEN_MAX]], lens: &mut [u8]) -> Option<usize> {
    let mut i = 0;
    let mut n = 0;
    while i < line.len() {
        while i < line.len() && line[i] == b' ' {
            i += 1;
        }
        if i == line.len() {
            break;
        }
        if n == words.len() {
            return None;
        }
        i = take_word(line, i, &mut words[n], &mut lens[n])?;
        n += 1;
    }
    Some(n)
}

fn take_word(line: &[u8], mut i: usize, word: &mut [u8], len: &mut u8) -> Option<usize> {
    let quoted = line[i] == b'"';
    if quoted {
        i += 1;
    }
    let mut n = 0;
    while i < line.len() {
        let byte = line[i];
        if quoted && byte == b'"' {
            *len = u8::try_from(n).ok()?;
            return Some(i + 1);
        }
        if !quoted && byte == b' ' {
            break;
        }
        if n == word.len() {
            return None;
        }
        word[n] = byte;
        n += 1;
        i += 1;
    }
    if quoted {
        return None;
    }
    *len = u8::try_from(n).ok()?;
    Some(i)
}

fn token<'a>(raw: &'a [[u8; TOKEN_MAX]; TOKEN_COUNT], lens: &[u8], index: usize) -> &'a [u8] {
    &raw[index][..usize::from(lens[index])]
}
