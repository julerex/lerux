//! DEFLATE (RFC 1951) inside a zlib wrapper (RFC 1950).
//!
//! Inflate lives in this crate because a protection domain is `no_std` and the
//! Ladybird plan does not take `image`, `png`, or `flate2`.

use alloc::vec::Vec;

use crate::{checksum::adler32, error::DecodeError};

/// Inflated scanlines stay under this size. A larger claim is an error so a
/// corrupt length cannot abort the allocator.
pub(crate) const MAX_INFLATED_BYTES: usize = 1024 * 1024;

/// Code-length symbol order from RFC 1951.
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Length bases for symbols 257 through 285.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];

const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// Distance bases for symbols 0 through 29.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

const _: () = assert!(LENGTH_BASE.len() == LENGTH_EXTRA.len());
const _: () = assert!(DISTANCE_BASE.len() == DISTANCE_EXTRA.len());

pub(crate) fn inflate(input: &[u8], exact_len: usize) -> Result<Vec<u8>, DecodeError> {
    if exact_len > MAX_INFLATED_BYTES {
        return Err(DecodeError::new());
    }
    let mut input = input;
    read_zlib_header(&mut input)?;
    let mut bits = Bits::new(input);
    let mut output = Vec::new();
    inflate_blocks(&mut bits, &mut output, exact_len)?;
    if output.len() != exact_len {
        return Err(DecodeError::new());
    }
    bits.discard_to_byte()?;
    check_adler(&output, bits.rest())?;
    Ok(output)
}

fn read_zlib_header(input: &mut &[u8]) -> Result<(), DecodeError> {
    let cmf = take_byte(input)?;
    let flags = take_byte(input)?;
    let combined = u16::from(cmf) << 8 | u16::from(flags);
    let method = cmf & 0x0f;
    let window = cmf >> 4;
    let dictionary = flags & 0x20;
    if combined % 31 != 0 || method != 8 || window > 7 || dictionary != 0 {
        return Err(DecodeError::new());
    }
    Ok(())
}

fn take_byte(input: &mut &[u8]) -> Result<u8, DecodeError> {
    let Some((byte, rest)) = input.split_first() else {
        return Err(DecodeError::new());
    };
    *input = rest;
    Ok(*byte)
}

fn check_adler(output: &[u8], rest: &[u8]) -> Result<(), DecodeError> {
    let Some(bytes) = rest.first_chunk::<4>() else {
        return Err(DecodeError::new());
    };
    if rest.len() != 4 || u32::from_be_bytes(*bytes) != adler32(output) {
        return Err(DecodeError::new());
    }
    Ok(())
}

fn inflate_blocks(
    bits: &mut Bits<'_>,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), DecodeError> {
    loop {
        let last = bits.pull(1)? == 1;
        match block_kind(bits.pull(2)?)? {
            BlockKind::Stored => stored_block(bits, output, limit)?,
            BlockKind::Fixed => fixed_block(bits, output, limit)?,
            BlockKind::Dynamic => dynamic_block(bits, output, limit)?,
        }
        if last {
            return Ok(());
        }
    }
}

enum BlockKind {
    Stored,
    Fixed,
    Dynamic,
}

fn block_kind(bits: u32) -> Result<BlockKind, DecodeError> {
    match bits {
        0 => Ok(BlockKind::Stored),
        1 => Ok(BlockKind::Fixed),
        2 => Ok(BlockKind::Dynamic),
        _ => Err(DecodeError::new()),
    }
}

fn stored_block(
    bits: &mut Bits<'_>,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), DecodeError> {
    bits.discard_to_byte()?;
    let len = bits.pull(16)?;
    let nlen = bits.pull(16)?;
    if len ^ nlen != 0xffff {
        return Err(DecodeError::new());
    }
    let data = bits.take_aligned(usize_from(len)?)?;
    append(output, data, limit)
}

fn fixed_block(bits: &mut Bits<'_>, output: &mut Vec<u8>, limit: usize) -> Result<(), DecodeError> {
    let literal = fixed_literal()?;
    let distance = fixed_distance()?;
    coded_block(bits, output, limit, &literal, &distance)
}

fn dynamic_block(
    bits: &mut Bits<'_>,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), DecodeError> {
    let (literal, distance) = dynamic_trees(bits)?;
    coded_block(bits, output, limit, &literal, &distance)
}

fn fixed_literal() -> Result<Huffman, DecodeError> {
    let mut lengths = [0u8; 288];
    fill(&mut lengths[..=143], 8);
    fill(&mut lengths[144..=255], 9);
    fill(&mut lengths[256..=279], 7);
    fill(&mut lengths[280..=287], 8);
    Huffman::from_lengths(&lengths)
}

fn fixed_distance() -> Result<Huffman, DecodeError> {
    Huffman::from_lengths(&[5u8; 32])
}

fn fill(slot: &mut [u8], value: u8) {
    for byte in slot {
        *byte = value;
    }
}

fn dynamic_trees(bits: &mut Bits<'_>) -> Result<(Huffman, Huffman), DecodeError> {
    let literal_count = usize_from(bits.pull(5)? + 257)?;
    let distance_count = usize_from(bits.pull(5)? + 1)?;
    let code_count = usize_from(bits.pull(4)? + 4)?;
    let alphabet = code_length_tree(bits, code_count)?;
    let total = literal_count
        .checked_add(distance_count)
        .ok_or(DecodeError::new())?;
    let lengths = read_lengths(bits, &alphabet, total)?;
    let literal_lens = lengths.get(..literal_count).ok_or(DecodeError::new())?;
    let distance_lens = lengths.get(literal_count..).ok_or(DecodeError::new())?;
    Ok((
        Huffman::from_lengths(literal_lens)?,
        Huffman::from_lengths(distance_lens)?,
    ))
}

fn code_length_tree(bits: &mut Bits<'_>, code_count: usize) -> Result<Huffman, DecodeError> {
    let mut lengths = [0u8; 19];
    for index in 0..code_count {
        let symbol = CODE_LENGTH_ORDER.get(index).ok_or(DecodeError::new())?;
        let slot = lengths.get_mut(*symbol).ok_or(DecodeError::new())?;
        *slot = narrow_u8(bits.pull(3)?)?;
    }
    Huffman::from_lengths(&lengths)
}

fn read_lengths(bits: &mut Bits<'_>, tree: &Huffman, total: usize) -> Result<Vec<u8>, DecodeError> {
    let mut lengths = Vec::new();
    while lengths.len() < total {
        append_length(bits, tree, &mut lengths, total)?;
    }
    Ok(lengths)
}

fn append_length(
    bits: &mut Bits<'_>,
    tree: &Huffman,
    lengths: &mut Vec<u8>,
    total: usize,
) -> Result<(), DecodeError> {
    let symbol = tree.decode(bits)?;
    match symbol {
        0..=15 => push_repeat(lengths, narrow_u16(symbol)?, 1, total),
        16 => repeat_previous(bits, lengths, total),
        17 => repeat_zero(bits, lengths, total, 3, 3),
        18 => repeat_zero(bits, lengths, total, 11, 7),
        _ => Err(DecodeError::new()),
    }
}

fn repeat_previous(
    bits: &mut Bits<'_>,
    lengths: &mut Vec<u8>,
    total: usize,
) -> Result<(), DecodeError> {
    let previous = lengths.last().copied().ok_or(DecodeError::new())?;
    let extra = usize_from(bits.pull(2)?)?;
    let count = 3usize.checked_add(extra).ok_or(DecodeError::new())?;
    push_repeat(lengths, previous, count, total)
}

fn repeat_zero(
    bits: &mut Bits<'_>,
    lengths: &mut Vec<u8>,
    total: usize,
    base: usize,
    extra_bits: u32,
) -> Result<(), DecodeError> {
    let extra = usize_from(bits.pull(extra_bits)?)?;
    let count = base.checked_add(extra).ok_or(DecodeError::new())?;
    push_repeat(lengths, 0, count, total)
}

fn push_repeat(
    lengths: &mut Vec<u8>,
    value: u8,
    count: usize,
    total: usize,
) -> Result<(), DecodeError> {
    let end = lengths.len().checked_add(count).ok_or(DecodeError::new())?;
    if end > total {
        return Err(DecodeError::new());
    }
    lengths.resize(end, value);
    Ok(())
}

fn coded_block(
    bits: &mut Bits<'_>,
    output: &mut Vec<u8>,
    limit: usize,
    literal: &Huffman,
    distance: &Huffman,
) -> Result<(), DecodeError> {
    loop {
        let symbol = literal.decode(bits)?;
        if symbol < 256 {
            push_byte(output, narrow_u16(symbol)?, limit)?;
            continue;
        }
        if symbol == 256 {
            return Ok(());
        }
        copy_length(bits, output, limit, distance, symbol)?;
    }
}

fn push_byte(output: &mut Vec<u8>, byte: u8, limit: usize) -> Result<(), DecodeError> {
    if output.len() >= limit {
        return Err(DecodeError::new());
    }
    output.push(byte);
    Ok(())
}

fn copy_length(
    bits: &mut Bits<'_>,
    output: &mut Vec<u8>,
    limit: usize,
    distance_tree: &Huffman,
    symbol: u16,
) -> Result<(), DecodeError> {
    let length = decode_length(symbol, bits)?;
    let distance = decode_distance(distance_tree.decode(bits)?, bits)?;
    copy_match(output, distance, length, limit)
}

fn decode_length(symbol: u16, bits: &mut Bits<'_>) -> Result<usize, DecodeError> {
    let index = symbol.checked_sub(257).ok_or(DecodeError::new())?;
    let base = LENGTH_BASE
        .get(usize::from(index))
        .copied()
        .ok_or(DecodeError::new())?;
    let extra = LENGTH_EXTRA
        .get(usize::from(index))
        .copied()
        .ok_or(DecodeError::new())?;
    add_base(base, bits.pull(u32::from(extra))?)
}

fn decode_distance(symbol: u16, bits: &mut Bits<'_>) -> Result<usize, DecodeError> {
    let index = usize::from(symbol);
    let base = DISTANCE_BASE
        .get(index)
        .copied()
        .ok_or(DecodeError::new())?;
    let extra = DISTANCE_EXTRA
        .get(index)
        .copied()
        .ok_or(DecodeError::new())?;
    add_base(base, bits.pull(u32::from(extra))?)
}

fn add_base(base: u16, extra: u32) -> Result<usize, DecodeError> {
    let sum = u32::from(base)
        .checked_add(extra)
        .ok_or(DecodeError::new())?;
    usize_from(sum)
}

fn copy_match(
    output: &mut Vec<u8>,
    distance: usize,
    length: usize,
    limit: usize,
) -> Result<(), DecodeError> {
    if distance == 0 || distance > output.len() {
        return Err(DecodeError::new());
    }
    let end = output.len().checked_add(length).ok_or(DecodeError::new())?;
    if end > limit {
        return Err(DecodeError::new());
    }
    for _ in 0..length {
        let index = output.len() - distance;
        let byte = output.get(index).copied().ok_or(DecodeError::new())?;
        output.push(byte);
    }
    Ok(())
}

fn append(output: &mut Vec<u8>, data: &[u8], limit: usize) -> Result<(), DecodeError> {
    let end = output
        .len()
        .checked_add(data.len())
        .ok_or(DecodeError::new())?;
    if end > limit {
        return Err(DecodeError::new());
    }
    output.extend_from_slice(data);
    Ok(())
}

struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn from_lengths(lengths: &[u8]) -> Result<Self, DecodeError> {
        let counts = count_lengths(lengths)?;
        check_oversubscribed(&counts)?;
        Ok(Self {
            symbols: pack_symbols(lengths, &counts)?,
            counts,
        })
    }

    fn decode(&self, bits: &mut Bits<'_>) -> Result<u16, DecodeError> {
        let mut code = 0u32;
        let mut first = 0u32;
        let mut index = 0usize;
        for len in 1..self.counts.len() {
            code = shift_code(code, bits.pull(1)?)?;
            let count = u32::from(self.counts[len]);
            if let Some(symbol) = self.symbol_at(index, code, first, count)? {
                return Ok(symbol);
            }
            index = step_index(index, count)?;
            first = step_first(first, count)?;
        }
        Err(DecodeError::new())
    }

    fn symbol_at(
        &self,
        index: usize,
        code: u32,
        first: u32,
        count: u32,
    ) -> Result<Option<u16>, DecodeError> {
        if code < first {
            return Err(DecodeError::new());
        }
        let Some(limit) = first.checked_add(count) else {
            return Err(DecodeError::new());
        };
        if code >= limit {
            return Ok(None);
        }
        let offset = usize_from(code - first)?;
        let slot = index.checked_add(offset).ok_or(DecodeError::new())?;
        self.symbols
            .get(slot)
            .copied()
            .map(Some)
            .ok_or(DecodeError::new())
    }
}

fn count_lengths(lengths: &[u8]) -> Result<[u16; 16], DecodeError> {
    let mut counts = [0u16; 16];
    for &len in lengths {
        let slot = counts.get_mut(usize::from(len)).ok_or(DecodeError::new())?;
        *slot = slot.checked_add(1).ok_or(DecodeError::new())?;
    }
    counts[0] = 0;
    Ok(counts)
}

fn check_oversubscribed(counts: &[u16; 16]) -> Result<(), DecodeError> {
    let mut left = 1i32;
    for count in counts.iter().skip(1) {
        left = left.checked_mul(2).ok_or(DecodeError::new())?;
        left = left
            .checked_sub(i32::from(*count))
            .ok_or(DecodeError::new())?;
    }
    Ok(())
}

fn pack_symbols(lengths: &[u8], counts: &[u16; 16]) -> Result<Vec<u16>, DecodeError> {
    let mut offsets = [0usize; 16];
    let mut sum = 0usize;
    for len in 1..offsets.len() {
        offsets[len] = sum;
        sum = sum
            .checked_add(usize::from(counts[len]))
            .ok_or(DecodeError::new())?;
    }
    let mut symbols = alloc::vec![0; sum];
    for (symbol, &len) in lengths.iter().enumerate() {
        place_symbol(&mut symbols, &mut offsets, symbol, len)?;
    }
    Ok(symbols)
}

fn place_symbol(
    symbols: &mut [u16],
    offsets: &mut [usize; 16],
    symbol: usize,
    len: u8,
) -> Result<(), DecodeError> {
    if len == 0 {
        return Ok(());
    }
    let at = usize::from(len);
    let slot = offsets.get(at).copied().ok_or(DecodeError::new())?;
    let next = slot.checked_add(1).ok_or(DecodeError::new())?;
    let Some(offset) = offsets.get_mut(at) else {
        return Err(DecodeError::new());
    };
    *offset = next;
    let id = u16::try_from(symbol).map_err(|_| DecodeError::new())?;
    let Some(place) = symbols.get_mut(slot) else {
        return Err(DecodeError::new());
    };
    *place = id;
    Ok(())
}

fn shift_code(code: u32, bit: u32) -> Result<u32, DecodeError> {
    let shifted = code.checked_shl(1).ok_or(DecodeError::new())?;
    Ok(shifted | bit)
}

fn step_first(first: u32, count: u32) -> Result<u32, DecodeError> {
    let sum = first.checked_add(count).ok_or(DecodeError::new())?;
    sum.checked_shl(1).ok_or(DecodeError::new())
}

fn step_index(index: usize, count: u32) -> Result<usize, DecodeError> {
    index
        .checked_add(usize_from(count)?)
        .ok_or(DecodeError::new())
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    have: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            acc: 0,
            have: 0,
        }
    }

    fn pull(&mut self, count: u32) -> Result<u32, DecodeError> {
        if count > 16 {
            return Err(DecodeError::new());
        }
        self.fill(count)?;
        if self.have < count {
            return Err(DecodeError::new());
        }
        let value = self.acc & ((1u32 << count) - 1);
        self.acc >>= count;
        self.have -= count;
        Ok(value)
    }

    fn fill(&mut self, count: u32) -> Result<(), DecodeError> {
        while self.have < count {
            if self.have >= 24 {
                return Err(DecodeError::new());
            }
            let byte = self.data.get(self.pos).copied().ok_or(DecodeError::new())?;
            self.pos = self.pos.checked_add(1).ok_or(DecodeError::new())?;
            self.acc |= u32::from(byte) << self.have;
            self.have += 8;
        }
        Ok(())
    }

    /// Drop bits that belong to the byte already pulled. The next byte is `pos`.
    fn discard_to_byte(&mut self) -> Result<(), DecodeError> {
        if self.have >= 8 {
            return Err(DecodeError::new());
        }
        self.acc = 0;
        self.have = 0;
        Ok(())
    }

    fn take_aligned(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        if self.have != 0 {
            return Err(DecodeError::new());
        }
        let end = self.pos.checked_add(len).ok_or(DecodeError::new())?;
        let slice = self.data.get(self.pos..end).ok_or(DecodeError::new())?;
        self.pos = end;
        Ok(slice)
    }

    fn rest(&self) -> &'a [u8] {
        self.data.get(self.pos..).unwrap_or(&[])
    }
}

fn usize_from(value: u32) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| DecodeError::new())
}

fn narrow_u8(value: u32) -> Result<u8, DecodeError> {
    u8::try_from(value).map_err(|_| DecodeError::new())
}

fn narrow_u16(value: u16) -> Result<u8, DecodeError> {
    u8::try_from(value).map_err(|_| DecodeError::new())
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::inflate;

    fn hex(text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut index = 0;
        while index + 1 < bytes.len() {
            let high = nibble(bytes[index]);
            let low = nibble(bytes[index + 1]);
            out.push((high << 4) | low);
            index += 2;
        }
        out
    }

    fn nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("hex digit"),
        }
    }

    #[test]
    fn stored_block_round_trips() {
        let raw = hex("7801010300fcff616263024d0127");
        let out = inflate(&raw, 3).expect("stored");
        assert_eq!(out, b"abc");
    }

    #[test]
    fn fixed_block_copies_a_match() {
        let raw = hex("78dacb48cdc9c957c84090003a2e067d");
        let out = inflate(&raw, 17).expect("fixed");
        assert_eq!(out, b"hello hello hello");
    }

    #[test]
    fn dynamic_block_copies_a_run() {
        let raw = hex("780105c1810d00000082a099fe7f73c08c193366cc9831637605200051");
        let out = inflate(&raw, 32).expect("dynamic");
        assert_eq!(out, bytes_1_2_3_4());
    }

    #[test]
    fn empty_fixed_block_is_empty() {
        let raw = hex("7801030000000001");
        let out = inflate(&raw, 0).expect("empty");
        assert!(out.is_empty());
    }

    #[test]
    fn bad_adler_is_an_error() {
        let raw = hex("7801010300fcff616263024d0126");
        assert!(inflate(&raw, 3).is_err());
    }

    #[test]
    fn truncated_stream_is_an_error() {
        assert!(inflate(&hex("7801"), 1).is_err());
    }

    fn bytes_1_2_3_4() -> Vec<u8> {
        let mut out = Vec::new();
        for _ in 0..8 {
            out.extend_from_slice(&[1, 2, 3, 4]);
        }
        out
    }
}
