//! Minimal `LERUXFS1` on-disk layout for lerux smoke and workstation profiles.
//!
//! Uses LBAs 1+ on `disk.img`, leaving LBA 0 (MBR) untouched. One directory sector
//! holds up to 16 files; each file occupies a single 512-byte data sector.

#![no_std]

use lerux_interface_types::SECTOR_SIZE;

/// Eight-byte superblock magic.
pub const MAGIC: &[u8; 8] = b"LERUXFS1";

/// On-disk format version.
pub const VERSION: u32 = 1;

/// Superblock sector (after MBR).
pub const SUPERBLOCK_LBA: u32 = 1;

/// Directory sector.
pub const DIR_LBA: u32 = 2;

/// First LBA available for file payloads.
pub const DATA_START_LBA: u32 = 3;

/// Maximum directory entries per sector.
pub const MAX_ENTRIES: usize = 16;

/// Maximum file name bytes stored in a directory entry.
pub const NAME_LEN: usize = 24;

/// Serialized directory entry size in bytes.
pub const ENTRY_SIZE: usize = 32;

/// Parsed superblock fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Superblock {
    pub file_count: u32,
    pub next_data_lba: u32,
}

impl Default for Superblock {
    fn default() -> Self {
        Self {
            file_count: 0,
            next_data_lba: DATA_START_LBA,
        }
    }
}

impl Superblock {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parsed directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirEntry {
    pub name: [u8; NAME_LEN],
    pub name_len: u8,
    pub data_lba: u32,
    pub size: u32,
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            name: [0; NAME_LEN],
            name_len: 0,
            data_lba: 0,
            size: 0,
        }
    }

    pub fn name_slice(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    pub fn is_free(&self) -> bool {
        self.data_lba == 0
    }
}

/// Returns true when `sector` contains a valid `LERUXFS1` superblock.
pub fn is_formatted(sector: &[u8; SECTOR_SIZE]) -> bool {
    sector.starts_with(MAGIC) && read_u32(&sector[8..12]) == VERSION
}

/// Decode superblock fields from `sector`.
pub fn decode_superblock(sector: &[u8; SECTOR_SIZE]) -> Option<Superblock> {
    if !is_formatted(sector) {
        return None;
    }
    Some(Superblock {
        file_count: read_u32(&sector[12..16]),
        next_data_lba: read_u32(&sector[16..20]),
    })
}

/// Encode superblock into `sector`.
pub fn encode_superblock(sector: &mut [u8; SECTOR_SIZE], sb: &Superblock) {
    sector.fill(0);
    sector[..8].copy_from_slice(MAGIC);
    write_u32(&mut sector[8..12], VERSION);
    write_u32(&mut sector[12..16], sb.file_count);
    write_u32(&mut sector[16..20], sb.next_data_lba);
}

/// Decode one directory entry from `dir_sector` at `index`.
pub fn decode_dir_entry(dir_sector: &[u8; SECTOR_SIZE], index: usize) -> DirEntry {
    let mut entry = DirEntry::empty();
    if index >= MAX_ENTRIES {
        return entry;
    }
    let off = index * ENTRY_SIZE;
    let slice = &dir_sector[off..off + ENTRY_SIZE];
    entry.name.copy_from_slice(&slice[..NAME_LEN]);
    entry.name_len = entry.name.iter().position(|&b| b == 0).unwrap_or(NAME_LEN) as u8;
    entry.data_lba = read_u32(&slice[24..28]);
    entry.size = read_u32(&slice[28..32]);
    entry
}

/// Encode one directory entry into `dir_sector` at `index`.
pub fn encode_dir_entry(dir_sector: &mut [u8; SECTOR_SIZE], index: usize, entry: &DirEntry) {
    if index >= MAX_ENTRIES {
        return;
    }
    let off = index * ENTRY_SIZE;
    let slice = &mut dir_sector[off..off + ENTRY_SIZE];
    slice.fill(0);
    let name_len = entry.name_len as usize;
    slice[..name_len].copy_from_slice(&entry.name[..name_len]);
    write_u32(&mut slice[24..28], entry.data_lba);
    write_u32(&mut slice[28..32], entry.size);
}

/// Find a directory index by file name, if present.
pub fn find_by_name(dir_sector: &[u8; SECTOR_SIZE], name: &[u8]) -> Option<usize> {
    for index in 0..MAX_ENTRIES {
        let entry = decode_dir_entry(dir_sector, index);
        if !entry.is_free() && entry.name_slice() == name {
            return Some(index);
        }
    }
    None
}

/// Find the first free directory slot.
pub fn find_free_slot(dir_sector: &[u8; SECTOR_SIZE]) -> Option<usize> {
    (0..MAX_ENTRIES).find(|&index| decode_dir_entry(dir_sector, index).is_free())
}

/// Count allocated directory entries.
pub fn count_entries(dir_sector: &[u8; SECTOR_SIZE]) -> u8 {
    let mut count = 0u8;
    for index in 0..MAX_ENTRIES {
        if !decode_dir_entry(dir_sector, index).is_free() {
            count = count.saturating_add(1);
        }
    }
    count
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn write_u32(bytes: &mut [u8], value: u32) {
    let le = value.to_le_bytes();
    bytes[..4].copy_from_slice(&le);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_superblock() {
        let sb = Superblock {
            file_count: 2,
            next_data_lba: 5,
        };
        let mut sector = [0u8; SECTOR_SIZE];
        encode_superblock(&mut sector, &sb);
        assert!(is_formatted(&sector));
        assert_eq!(decode_superblock(&sector), Some(sb));
    }

    #[test]
    fn dir_entry_round_trip() {
        let mut dir = [0u8; SECTOR_SIZE];
        let mut entry = DirEntry::empty();
        entry.name[..5].copy_from_slice(b"hello");
        entry.name_len = 5;
        entry.data_lba = 9;
        entry.size = 42;
        encode_dir_entry(&mut dir, 1, &entry);
        let decoded = decode_dir_entry(&dir, 1);
        assert_eq!(decoded.name_slice(), b"hello");
        assert_eq!(decoded.data_lba, 9);
        assert_eq!(decoded.size, 42);
    }
}
