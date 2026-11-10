//! zlib (RFC 1950) around DEFLATE (RFC 1951): stored, fixed Huffman, and
//! dynamic Huffman blocks, then the Adler-32 trailer.

use alloc::vec::Vec;

use crate::PngError;

const MAX_BITS: usize = 15;
const MAX_SYMBOLS: usize = 288;
const END_OF_BLOCK: u16 = 256;
const MAX_LITERALS: usize = 286;
const MAX_DISTANCES: usize = 30;

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u16; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u16; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Inflate a zlib stream whose output must be exactly `len` bytes.
pub(crate) fn inflate_zlib(input: &[u8], len: usize) -> Result<Vec<u8>, PngError> {
    let &[cmf, flg, ref stream @ ..] = input else {
        return Err(PngError::Zlib);
    };
    let check = ((u16::from(cmf) << 8) | u16::from(flg)) % 31;
    if cmf & 0x0f != 8 || cmf >> 4 > 7 || flg & 0x20 != 0 || check != 0 {
        return Err(PngError::Zlib);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| PngError::Unsupported)?;
    let mut inflater = Inflater {
        input: stream,
        pos: 0,
        bit_buf: 0,
        bit_count: 0,
        out,
        limit: len,
    };
    inflater.blocks()?;
    if inflater.out.len() != len {
        return Err(PngError::Scanline);
    }
    if inflater.trailer()? != adler32(&inflater.out) {
        return Err(PngError::Adler);
    }
    Ok(inflater.out)
}

/// Canonical Huffman code, stored as the number of codes of each bit length
/// and then the symbols in code order.
struct Huffman {
    counts: [u16; MAX_BITS + 1],
    symbols: [u16; MAX_SYMBOLS],
}

impl Huffman {
    /// An over-subscribed set of lengths is rejected. An incomplete set is
    /// accepted, and a code it leaves unassigned fails to decode.
    fn new(lengths: &[u16]) -> Result<Self, PngError> {
        let mut counts = [0u16; MAX_BITS + 1];
        for &len in lengths {
            counts[usize::from(len)] += 1;
        }
        let mut left = 1i32;
        for &count in &counts[1..] {
            left = (left << 1) - i32::from(count);
            if left < 0 {
                return Err(PngError::Zlib);
            }
        }
        let mut offsets = [0u16; MAX_BITS + 1];
        let mut next = 0;
        for (offset, &count) in offsets.iter_mut().zip(&counts).skip(1) {
            *offset = next;
            next += count;
        }
        let mut symbols = [0u16; MAX_SYMBOLS];
        for (symbol, &len) in lengths.iter().enumerate() {
            if len == 0 {
                continue;
            }
            let slot = &mut offsets[usize::from(len)];
            symbols[usize::from(*slot)] = u16::try_from(symbol).map_err(|_| PngError::Zlib)?;
            *slot += 1;
        }
        Ok(Self { counts, symbols })
    }
}

fn fixed_tables() -> Result<(Huffman, Huffman), PngError> {
    let mut lengths = [0u16; MAX_SYMBOLS];
    lengths[..144].fill(8);
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    lengths[280..].fill(8);
    Ok((Huffman::new(&lengths)?, Huffman::new(&[5; MAX_DISTANCES])?))
}

struct Inflater<'a> {
    input: &'a [u8],
    pos: usize,
    bit_buf: u32,
    bit_count: u32,
    out: Vec<u8>,
    limit: usize,
}

impl<'a> Inflater<'a> {
    /// Read `n` bits, least significant first. Callers read at most 13.
    fn bits(&mut self, n: u32) -> Result<u16, PngError> {
        while self.bit_count < n {
            let byte = *self.input.get(self.pos).ok_or(PngError::Zlib)?;
            self.pos += 1;
            self.bit_buf |= u32::from(byte) << self.bit_count;
            self.bit_count += 8;
        }
        let value = self.bit_buf & ((1 << n) - 1);
        self.bit_buf >>= n;
        self.bit_count -= n;
        u16::try_from(value).map_err(|_| PngError::Zlib)
    }

    /// Fewer than eight bits stay buffered after a read, so dropping them
    /// lands on the next whole byte.
    fn align(&mut self) {
        self.bit_buf = 0;
        self.bit_count = 0;
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8], PngError> {
        let end = self.pos.checked_add(n).ok_or(PngError::Zlib)?;
        let bytes = self.input.get(self.pos..end).ok_or(PngError::Zlib)?;
        self.pos = end;
        Ok(bytes)
    }

    fn room(&self, n: usize) -> Result<(), PngError> {
        if self.limit - self.out.len() < n {
            return Err(PngError::Scanline);
        }
        Ok(())
    }

    fn blocks(&mut self) -> Result<(), PngError> {
        loop {
            let last = self.bits(1)? == 1;
            match self.bits(2)? {
                0 => self.stored()?,
                1 => {
                    let (literals, distances) = fixed_tables()?;
                    self.codes(&literals, &distances)?;
                }
                2 => {
                    let (literals, distances) = self.dynamic_tables()?;
                    self.codes(&literals, &distances)?;
                }
                _ => return Err(PngError::Zlib),
            }
            if last {
                return Ok(());
            }
        }
    }

    fn stored(&mut self) -> Result<(), PngError> {
        self.align();
        let &[len_lo, len_hi, nlen_lo, nlen_hi] = self.bytes(4)? else {
            return Err(PngError::Zlib);
        };
        let len = u16::from_le_bytes([len_lo, len_hi]);
        if len != !u16::from_le_bytes([nlen_lo, nlen_hi]) {
            return Err(PngError::Zlib);
        }
        let data = self.bytes(usize::from(len))?;
        self.room(data.len())?;
        self.out.extend_from_slice(data);
        Ok(())
    }

    fn dynamic_tables(&mut self) -> Result<(Huffman, Huffman), PngError> {
        let literals = usize::from(self.bits(5)?) + 257;
        let distances = usize::from(self.bits(5)?) + 1;
        let code_lengths = usize::from(self.bits(4)?) + 4;
        if literals > MAX_LITERALS || distances > MAX_DISTANCES {
            return Err(PngError::Zlib);
        }
        let mut lengths = [0u16; 19];
        for &slot in &CODE_LENGTH_ORDER[..code_lengths] {
            lengths[slot] = self.bits(3)?;
        }
        let code_length_table = Huffman::new(&lengths)?;
        let mut lengths = [0u16; MAX_LITERALS + MAX_DISTANCES];
        let lengths = &mut lengths[..literals + distances];
        self.code_lengths(&code_length_table, lengths)?;
        if lengths[usize::from(END_OF_BLOCK)] == 0 {
            return Err(PngError::Zlib);
        }
        let (literal_lengths, distance_lengths) = lengths.split_at(literals);
        Ok((
            Huffman::new(literal_lengths)?,
            Huffman::new(distance_lengths)?,
        ))
    }

    fn code_lengths(&mut self, table: &Huffman, lengths: &mut [u16]) -> Result<(), PngError> {
        let mut index = 0;
        while index < lengths.len() {
            let symbol = self.symbol(table)?;
            let (value, repeat) = match symbol {
                0..=15 => (symbol, 1),
                16 => {
                    let previous = *lengths[..index].last().ok_or(PngError::Zlib)?;
                    (previous, 3 + self.bits(2)?)
                }
                17 => (0, 3 + self.bits(3)?),
                18 => (0, 11 + self.bits(7)?),
                _ => return Err(PngError::Zlib),
            };
            let end = index + usize::from(repeat);
            lengths
                .get_mut(index..end)
                .ok_or(PngError::Zlib)?
                .fill(value);
            index = end;
        }
        Ok(())
    }

    fn symbol(&mut self, table: &Huffman) -> Result<u16, PngError> {
        let mut code = 0usize;
        let mut first = 0usize;
        let mut index = 0usize;
        for &count in &table.counts[1..] {
            code |= usize::from(self.bits(1)?);
            let count = usize::from(count);
            if code < first + count {
                return Ok(table.symbols[index + code - first]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(PngError::Zlib)
    }

    fn codes(&mut self, literals: &Huffman, distances: &Huffman) -> Result<(), PngError> {
        loop {
            let symbol = self.symbol(literals)?;
            if let Ok(byte) = u8::try_from(symbol) {
                self.room(1)?;
                self.out.push(byte);
                continue;
            }
            if symbol == END_OF_BLOCK {
                return Ok(());
            }
            let length = self.extra(symbol - 257, &LENGTH_BASE, &LENGTH_EXTRA)?;
            let code = self.symbol(distances)?;
            let distance = self.extra(code, &DISTANCE_BASE, &DISTANCE_EXTRA)?;
            self.copy(distance, length)?;
        }
    }

    fn extra(&mut self, code: u16, base: &[u16], extra: &[u16]) -> Result<usize, PngError> {
        let index = usize::from(code);
        let (Some(&base), Some(&extra)) = (base.get(index), extra.get(index)) else {
            return Err(PngError::Zlib);
        };
        Ok(usize::from(base) + usize::from(self.bits(u32::from(extra))?))
    }

    /// The source may overlap the bytes this copy appends.
    fn copy(&mut self, distance: usize, length: usize) -> Result<(), PngError> {
        let start = self.out.len().checked_sub(distance).ok_or(PngError::Zlib)?;
        self.room(length)?;
        for index in start..start + length {
            let byte = self.out[index];
            self.out.push(byte);
        }
        Ok(())
    }

    fn trailer(&mut self) -> Result<u32, PngError> {
        self.align();
        let bytes = self.bytes(4)?.try_into().map_err(|_| PngError::Zlib)?;
        Ok(u32::from_be_bytes(bytes))
    }
}

fn adler32(bytes: &[u8]) -> u32 {
    const MODULUS: u32 = 65_521;
    // zlib's NMAX. A longer run can overflow the second sum in a u32.
    const RUN: usize = 5552;
    let (mut low, mut high) = (1u32, 0u32);
    for run in bytes.chunks(RUN) {
        for &byte in run {
            low += u32::from(byte);
            high += low;
        }
        low %= MODULUS;
        high %= MODULUS;
    }
    (high << 16) | low
}
