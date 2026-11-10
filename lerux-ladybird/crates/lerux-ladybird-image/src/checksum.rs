//! PNG chunk CRC-32 and the zlib Adler-32.

const CRC_POLY: u32 = 0xedb8_8320;
const ADLER_BASE: u32 = 65521;

pub(crate) fn crc32_pair(first: &[u8], second: &[u8]) -> u32 {
    !fold(fold(0xffff_ffff, first), second)
}

pub(crate) fn adler32(bytes: &[u8]) -> u32 {
    let mut sum1 = 1u32;
    let mut sum2 = 0u32;
    for &byte in bytes {
        sum1 = (sum1 + u32::from(byte)) % ADLER_BASE;
        sum2 = (sum2 + sum1) % ADLER_BASE;
    }
    (sum2 << 16) | sum1
}

fn fold(mut crc: u32, bytes: &[u8]) -> u32 {
    for &byte in bytes {
        crc = step(crc, byte);
    }
    crc
}

fn step(mut crc: u32, byte: u8) -> u32 {
    crc ^= u32::from(byte);
    for _ in 0..8 {
        let mix = 0u32.wrapping_sub(crc & 1);
        crc = (crc >> 1) ^ (CRC_POLY & mix);
    }
    crc
}
