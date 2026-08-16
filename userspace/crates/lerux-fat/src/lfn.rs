//! VFAT long file names and 8.3 aliases.
//!
//! LFN directory entries (attr `0x0F`) store up to 13 UTF-16 units each and
//! immediately precede the 8.3 short entry they name. We only persist
//! ASCII / Latin-1 bytes from IPC paths (zero-extended to UTF-16).

use lerux_interface_types::SECTOR_SIZE;

use crate::{
    path_to_short_name, read_u16, write_u16, ShortName, DIR_ENTRY_SIZE, ENTRIES_PER_SECTOR,
};

/// VFAT long-name attribute (read-only | hidden | system | volume).
pub const ATTR_LFN: u8 = 0x0F;

/// Last-LFN-entry flag in the sequence byte.
pub const LFN_LAST: u8 = 0x40;

/// UTF-16 units stored in one LFN directory entry.
pub const LFN_CHARS_PER_ENTRY: usize = 13;

/// Long names we encode (IPC component cap is 22; 24 fills two LFN slots).
pub const MAX_LFN_CHARS: usize = 24;

/// LFN entries needed for [`MAX_LFN_CHARS`].
pub const MAX_LFN_ENTRIES: usize = 2;

/// Directory (subdirectory) attribute bit.
pub const ATTR_DIR: u8 = 0x10;

/// Archive attribute (regular file).
pub const ATTR_ARCHIVE: u8 = 0x20;

/// Volume-label attribute bit.
pub const ATTR_VOLUME: u8 = 0x08;

/// Byte offsets of the 13 UTF-16 units inside a 32-byte LFN entry.
const LFN_CHAR_OFFSETS: [usize; LFN_CHARS_PER_ENTRY] =
    [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];

/// One decoded LFN directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LfnPiece {
    /// 1-based fragment index (without [`LFN_LAST`]).
    pub seq: u8,
    /// True when this is the last (highest) fragment — stored first on disk.
    pub last: bool,
    /// Checksum of the following 8.3 name.
    pub checksum: u8,
    pub chars: [u16; LFN_CHARS_PER_ENTRY],
}

/// Windows-style short-name checksum used by LFN entries.
pub fn lfn_checksum(name: &ShortName) -> u8 {
    let mut sum: u8 = 0;
    for &b in name {
        sum = ((sum & 1) << 7).wrapping_add(sum >> 1).wrapping_add(b);
    }
    sum
}

/// ASCII case-insensitive equality.
pub fn names_eq_ci(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

/// True when `name` can be stored as a single 8.3 entry (no LFN).
pub fn is_pure_short_name(name: &[u8]) -> bool {
    path_to_short_name(name).is_some()
}

/// Directory slots needed to store `name` (LFN fragments + short, or 1).
pub fn slots_for_name(name: &[u8]) -> u8 {
    if is_pure_short_name(name) {
        1
    } else {
        lfn_entry_count(name.len()).saturating_add(1)
    }
}

/// Number of LFN fragments for a long name of `name_len` bytes.
pub fn lfn_entry_count(name_len: usize) -> u8 {
    if name_len == 0 {
        return 0;
    }
    name_len.div_ceil(LFN_CHARS_PER_ENTRY) as u8
}

/// Build an 8.3 alias for `name`.
///
/// `seq` is the `~N` numeric suffix (1..=9) when the name is not already 8.3.
/// `seq == 0` still produces `~1` for lossy names.
pub fn make_short_alias(name: &[u8], seq: u8) -> Option<ShortName> {
    if let Some(short) = path_to_short_name(name) {
        return Some(short);
    }
    if name.is_empty() || name.len() > MAX_LFN_CHARS {
        return None;
    }
    let n = if seq == 0 { 1 } else { seq };
    if n > 9 {
        return None;
    }
    let (base, ext) = split_base_ext(name);
    let mut base_clean = [0u8; 8];
    let mut base_len = 0usize;
    for &b in base {
        let Some(u) = to_alias_char(b) else {
            continue;
        };
        if base_len < 8 {
            base_clean[base_len] = u;
            base_len += 1;
        }
    }
    if base_len == 0 {
        base_clean[0] = b'_';
        base_len = 1;
    }
    let mut ext_clean = [b' '; 3];
    let mut ext_i = 0usize;
    for &b in ext {
        let Some(u) = to_alias_char(b) else {
            continue;
        };
        if ext_i < 3 {
            ext_clean[ext_i] = u;
            ext_i += 1;
        }
    }
    let mut out = [b' '; 11];
    let keep = base_len.min(6);
    out[..keep].copy_from_slice(&base_clean[..keep]);
    out[keep] = b'~';
    out[keep + 1] = b'0' + n;
    out[8..11].copy_from_slice(&ext_clean);
    Some(out)
}

fn split_base_ext(name: &[u8]) -> (&[u8], &[u8]) {
    match name.iter().rposition(|&b| b == b'.') {
        Some(dot) if dot > 0 && dot + 1 < name.len() => (&name[..dot], &name[dot + 1..]),
        _ => (name, &name[0..0]),
    }
}

fn to_alias_char(b: u8) -> Option<u8> {
    match b {
        b'a'..=b'z' => Some(b - 32),
        b'A'..=b'Z'
        | b'0'..=b'9'
        | b'!'
        | b'#'
        | b'$'
        | b'%'
        | b'&'
        | b'\''
        | b'('
        | b')'
        | b'-'
        | b'@'
        | b'^'
        | b'_'
        | b'`'
        | b'{'
        | b'}'
        | b'~' => Some(b),
        _ => None,
    }
}

/// Decode an LFN directory entry at `index` within one sector.
pub fn decode_lfn_entry(sector: &[u8; SECTOR_SIZE], index: usize) -> Option<LfnPiece> {
    if index >= ENTRIES_PER_SECTOR {
        return None;
    }
    let off = index * DIR_ENTRY_SIZE;
    let s = &sector[off..off + DIR_ENTRY_SIZE];
    if s[11] != ATTR_LFN {
        return None;
    }
    let seq_raw = s[0];
    if seq_raw == 0x00 || seq_raw == 0xE5 {
        return None;
    }
    let last = seq_raw & LFN_LAST != 0;
    let seq = seq_raw & 0x1F;
    if seq == 0 {
        return None;
    }
    let mut chars = [0u16; LFN_CHARS_PER_ENTRY];
    for (i, &ch_off) in LFN_CHAR_OFFSETS.iter().enumerate() {
        chars[i] = read_u16(&s[ch_off..ch_off + 2]);
    }
    Some(LfnPiece {
        seq,
        last,
        checksum: s[13],
        chars,
    })
}

/// Encode an LFN directory entry at `index` within one sector.
pub fn encode_lfn_entry(sector: &mut [u8; SECTOR_SIZE], index: usize, piece: &LfnPiece) {
    if index >= ENTRIES_PER_SECTOR {
        return;
    }
    let off = index * DIR_ENTRY_SIZE;
    let s = &mut sector[off..off + DIR_ENTRY_SIZE];
    s.fill(0);
    s[0] = piece.seq | if piece.last { LFN_LAST } else { 0 };
    s[11] = ATTR_LFN;
    s[13] = piece.checksum;
    for (i, &ch_off) in LFN_CHAR_OFFSETS.iter().enumerate() {
        write_u16(&mut s[ch_off..ch_off + 2], piece.chars[i]);
    }
}

/// Pack `name` into on-disk LFN fragments (highest seq first).
///
/// Returns the number of fragments written into `out`.
pub fn encode_lfn_run(name: &[u8], short: &ShortName, out: &mut [LfnPiece; MAX_LFN_ENTRIES]) -> u8 {
    if name.is_empty() {
        return 0;
    }
    let nent = lfn_entry_count(name.len()).min(MAX_LFN_ENTRIES as u8);
    let sum = lfn_checksum(short);
    let mut units = [0xFFFFu16; MAX_LFN_ENTRIES * LFN_CHARS_PER_ENTRY];
    for (i, &b) in name.iter().enumerate() {
        if i >= units.len() {
            break;
        }
        units[i] = u16::from(b);
    }
    if name.len() < units.len() {
        units[name.len()] = 0;
    }
    for seq in 1..=nent {
        let mut chars = [0xFFFFu16; LFN_CHARS_PER_ENTRY];
        let src = (seq as usize - 1) * LFN_CHARS_PER_ENTRY;
        chars.copy_from_slice(&units[src..src + LFN_CHARS_PER_ENTRY]);
        // On-disk order is reverse: last fragment first.
        let dest = (nent - seq) as usize;
        out[dest] = LfnPiece {
            seq,
            last: seq == nent,
            checksum: sum,
            chars,
        };
    }
    nent
}

/// Collects LFN fragments while scanning a directory sector.
#[derive(Debug, Clone, Copy)]
pub struct LfnBuilder {
    buf: [u8; MAX_LFN_CHARS],
    len: u8,
    expected: u8,
    checksum: u8,
    start_slot: u16,
    active: bool,
    complete: bool,
}

impl Default for LfnBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LfnBuilder {
    pub const fn new() -> Self {
        Self {
            buf: [0; MAX_LFN_CHARS],
            len: 0,
            expected: 0,
            checksum: 0,
            start_slot: 0,
            active: false,
            complete: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Slot of the first (highest-seq) LFN fragment of the current run.
    pub fn start_slot(&self) -> u16 {
        self.start_slot
    }

    pub fn feed(&mut self, slot: u16, piece: &LfnPiece) {
        if piece.seq == 0 || usize::from(piece.seq) > MAX_LFN_ENTRIES {
            self.reset();
            return;
        }
        if piece.last {
            *self = Self::new();
            self.active = true;
            self.checksum = piece.checksum;
            self.start_slot = slot;
            self.expected = piece.seq;
        } else if !self.active || piece.seq != self.expected || piece.checksum != self.checksum {
            self.reset();
            return;
        }
        self.store_chars(piece);
        self.expected = piece.seq.saturating_sub(1);
        if self.expected == 0 {
            self.complete = true;
        }
    }

    fn store_chars(&mut self, piece: &LfnPiece) {
        let base = (piece.seq as usize - 1) * LFN_CHARS_PER_ENTRY;
        for (i, &unit) in piece.chars.iter().enumerate() {
            if unit == 0 {
                self.len = self.len.max((base + i) as u8);
                return;
            }
            if unit == 0xFFFF {
                continue;
            }
            let pos = base + i;
            if pos >= MAX_LFN_CHARS {
                return;
            }
            self.buf[pos] = if unit > 0xFF { b'_' } else { unit as u8 };
            self.len = self.len.max((pos + 1) as u8);
        }
    }

    /// If a complete LFN run matches `short`, return (name, first LFN slot).
    pub fn take(&mut self, short: &ShortName) -> Option<([u8; MAX_LFN_CHARS], u8, u16)> {
        if !self.complete || lfn_checksum(short) != self.checksum {
            self.reset();
            return None;
        }
        let name = self.buf;
        let len = self.len;
        let start = self.start_slot;
        self.reset();
        Some((name, len, start))
    }
}

/// True when `want` names this short entry, optionally via a collected LFN.
pub fn entry_matches(short: &ShortName, lfn: Option<&[u8]>, want: &[u8]) -> bool {
    if let Some(long) = lfn
        && names_eq_ci(long, want)
    {
        return true;
    }
    if let Some(s) = path_to_short_name(want) {
        return s == *short;
    }
    false
}
