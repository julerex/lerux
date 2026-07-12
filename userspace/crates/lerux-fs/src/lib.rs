//! `LERUXFS2` on-disk layout for lerux smoke and workstation profiles.
//!
//! Uses LBAs 1+ on `disk.img`, leaving LBA 0 (MBR) untouched.
//!
//! ## Layout
//!
//! | LBA | Content |
//! |-----|---------|
//! | 1 | Superblock (`LERUXFS2`) |
//! | 2 | Free-block bitmap (1 = free) |
//! | 3 | Root directory sector |
//! | 4+ | File data and subdirectory sectors |
//!
//! ## Directory entries
//!
//! Sixteen 32-byte entries per directory sector. Each entry is a file (contiguous
//! extent) or a subdirectory (points at another directory sector).
//!
//! ## Path grammar (IPC)
//!
//! See [`lerux_interface_types`] filesystem docs. This crate only handles
//! component names (no `/`); the server walks paths using these helpers.

#![no_std]

use lerux_interface_types::SECTOR_SIZE;

/// Eight-byte superblock magic (`LERUXFS2`).
pub const MAGIC: &[u8; 8] = b"LERUXFS2";

/// Legacy magic accepted only to detect pre-v2 volumes (reformat).
pub const MAGIC_V1: &[u8; 8] = b"LERUXFS1";

/// On-disk format version.
pub const VERSION: u32 = 2;

/// Superblock sector (after MBR).
pub const SUPERBLOCK_LBA: u32 = 1;

/// Free-map bitmap sector.
pub const FREE_MAP_LBA: u32 = 2;

/// Root directory sector.
pub const ROOT_DIR_LBA: u32 = 3;

/// First LBA available for dynamic allocation.
pub const DATA_START_LBA: u32 = 4;

/// Bitmap covers LBAs `0..TOTAL_LBAS` (one bit per LBA).
pub const TOTAL_LBAS: u32 = 2048;

/// Maximum directory entries per sector.
pub const MAX_ENTRIES: usize = 16;

/// Maximum component name bytes stored in a directory entry.
pub const NAME_LEN: usize = 22;

/// Serialized directory entry size in bytes.
pub const ENTRY_SIZE: usize = 32;

/// Directory entry flag: entry is a subdirectory.
pub const FLAG_DIR: u8 = 0x01;

/// Maximum contiguous sectors per file (16 KiB).
pub const MAX_FILE_SECTORS: u32 = 32;

/// Maximum path components when splitting an IPC path.
pub const MAX_COMPONENTS: usize = 8;

/// Parsed superblock fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Superblock {
    pub root_dir_lba: u32,
    pub free_map_lba: u32,
    pub data_start_lba: u32,
    pub total_lbas: u32,
}

impl Default for Superblock {
    fn default() -> Self {
        Self {
            root_dir_lba: ROOT_DIR_LBA,
            free_map_lba: FREE_MAP_LBA,
            data_start_lba: DATA_START_LBA,
            total_lbas: TOTAL_LBAS,
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
    pub flags: u8,
    pub first_lba: u32,
    pub size: u32,
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            name: [0; NAME_LEN],
            name_len: 0,
            flags: 0,
            first_lba: 0,
            size: 0,
        }
    }

    pub fn name_slice(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    pub fn is_free(&self) -> bool {
        self.first_lba == 0
    }

    pub fn is_dir(&self) -> bool {
        !self.is_free() && (self.flags & FLAG_DIR) != 0
    }

    pub fn is_file(&self) -> bool {
        !self.is_free() && (self.flags & FLAG_DIR) == 0
    }

    /// Contiguous sectors currently allocated for a file (0 if free).
    pub fn file_sectors(&self) -> u32 {
        if self.is_free() || self.is_dir() {
            return 0;
        }
        if self.size == 0 {
            return 1;
        }
        self.size
            .div_ceil(SECTOR_SIZE as u32)
            .clamp(1, MAX_FILE_SECTORS)
    }
}

/// Fixed-capacity split path (component names only).
#[derive(Debug, Clone, Copy)]
pub struct PathParts {
    pub count: u8,
    pub lens: [u8; MAX_COMPONENTS],
    pub names: [[u8; NAME_LEN]; MAX_COMPONENTS],
}

impl PathParts {
    pub const fn empty() -> Self {
        Self {
            count: 0,
            lens: [0; MAX_COMPONENTS],
            names: [[0; NAME_LEN]; MAX_COMPONENTS],
        }
    }

    pub fn component(&self, index: usize) -> Option<&[u8]> {
        if index >= self.count as usize {
            return None;
        }
        Some(&self.names[index][..self.lens[index] as usize])
    }
}

/// Path parse / validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    EmptyComponent,
    NameTooLong,
    TooManyComponents,
    InvalidByte,
}

/// Split an IPC path into components.
///
/// Accepts `""`, `"/"`, `"foo"`, `"/foo/bar"`. Leading/trailing `/` are ignored.
/// `.` and `..` are rejected. Bare names are a single root-relative component.
pub fn split_path(path: &[u8]) -> Result<PathParts, PathError> {
    let mut parts = PathParts::empty();
    let mut i = 0usize;
    while i < path.len() {
        while i < path.len() && path[i] == b'/' {
            i += 1;
        }
        if i >= path.len() {
            break;
        }
        let start = i;
        while i < path.len() && path[i] != b'/' {
            let b = path[i];
            if b == 0 || b == b'\\' {
                return Err(PathError::InvalidByte);
            }
            i += 1;
        }
        let comp = &path[start..i];
        if comp.is_empty() {
            return Err(PathError::EmptyComponent);
        }
        if comp == b"." || comp == b".." {
            return Err(PathError::InvalidByte);
        }
        if comp.len() > NAME_LEN {
            return Err(PathError::NameTooLong);
        }
        if parts.count as usize >= MAX_COMPONENTS {
            return Err(PathError::TooManyComponents);
        }
        let idx = parts.count as usize;
        parts.names[idx][..comp.len()].copy_from_slice(comp);
        parts.lens[idx] = comp.len() as u8;
        parts.count += 1;
    }
    Ok(parts)
}

/// Returns true when `sector` contains a valid `LERUXFS2` superblock.
pub fn is_formatted(sector: &[u8; SECTOR_SIZE]) -> bool {
    sector.starts_with(MAGIC) && read_u32(&sector[8..12]) == VERSION
}

/// Returns true when `sector` is a legacy `LERUXFS1` superblock.
pub fn is_legacy_v1(sector: &[u8; SECTOR_SIZE]) -> bool {
    sector.starts_with(MAGIC_V1) && read_u32(&sector[8..12]) == 1
}

/// Decode superblock fields from `sector`.
pub fn decode_superblock(sector: &[u8; SECTOR_SIZE]) -> Option<Superblock> {
    if !is_formatted(sector) {
        return None;
    }
    let sb = Superblock {
        root_dir_lba: read_u32(&sector[12..16]),
        free_map_lba: read_u32(&sector[16..20]),
        data_start_lba: read_u32(&sector[20..24]),
        total_lbas: read_u32(&sector[24..28]),
    };
    if sb.root_dir_lba == 0 || sb.free_map_lba == 0 || sb.total_lbas == 0 {
        return None;
    }
    if sb.total_lbas > TOTAL_LBAS {
        return None;
    }
    Some(sb)
}

/// Encode superblock into `sector`.
pub fn encode_superblock(sector: &mut [u8; SECTOR_SIZE], sb: &Superblock) {
    sector.fill(0);
    sector[..8].copy_from_slice(MAGIC);
    write_u32(&mut sector[8..12], VERSION);
    write_u32(&mut sector[12..16], sb.root_dir_lba);
    write_u32(&mut sector[16..20], sb.free_map_lba);
    write_u32(&mut sector[20..24], sb.data_start_lba);
    write_u32(&mut sector[24..28], sb.total_lbas);
}

/// Initialise free-map: all free, then mark reserved LBAs used.
pub fn encode_free_map_fresh(map: &mut [u8; SECTOR_SIZE], sb: &Superblock) {
    // 1 = free
    map.fill(0xff);
    for lba in 0..sb.data_start_lba.min(sb.total_lbas) {
        set_free(map, lba, false);
    }
    // LBAs beyond total_lbas stay free bits but are never allocated.
}

/// True when LBA is free in the bitmap.
pub fn is_free(map: &[u8; SECTOR_SIZE], lba: u32) -> bool {
    if lba >= TOTAL_LBAS {
        return false;
    }
    let byte = lba as usize / 8;
    let bit = lba as usize % 8;
    (map[byte] & (1 << bit)) != 0
}

/// Mark LBA free (`true`) or used (`false`).
pub fn set_free(map: &mut [u8; SECTOR_SIZE], lba: u32, free: bool) {
    if lba >= TOTAL_LBAS {
        return;
    }
    let byte = lba as usize / 8;
    let bit = lba as usize % 8;
    if free {
        map[byte] |= 1 << bit;
    } else {
        map[byte] &= !(1 << bit);
    }
}

/// Allocate `count` contiguous free LBAs at or after `sb.data_start_lba`.
pub fn alloc_contiguous(map: &mut [u8; SECTOR_SIZE], sb: &Superblock, count: u32) -> Option<u32> {
    if count == 0 || count > MAX_FILE_SECTORS {
        return None;
    }
    let start = sb.data_start_lba;
    let end = sb.total_lbas;
    if count > end.saturating_sub(start) {
        return None;
    }
    let mut lba = start;
    while lba + count <= end {
        let mut ok = true;
        for i in 0..count {
            if !is_free(map, lba + i) {
                ok = false;
                lba = lba + i + 1;
                break;
            }
        }
        if ok {
            for i in 0..count {
                set_free(map, lba + i, false);
            }
            return Some(lba);
        }
    }
    None
}

/// Free `count` contiguous LBAs starting at `lba`.
pub fn free_contiguous(map: &mut [u8; SECTOR_SIZE], lba: u32, count: u32) {
    for i in 0..count {
        set_free(map, lba.saturating_add(i), true);
    }
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
    entry.name_len = slice[NAME_LEN];
    if entry.name_len as usize > NAME_LEN {
        entry.name_len = NAME_LEN as u8;
    }
    entry.flags = slice[NAME_LEN + 1];
    entry.first_lba = read_u32(&slice[24..28]);
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
    let name_len = (entry.name_len as usize).min(NAME_LEN);
    slice[..name_len].copy_from_slice(&entry.name[..name_len]);
    slice[NAME_LEN] = name_len as u8;
    slice[NAME_LEN + 1] = entry.flags;
    write_u32(&mut slice[24..28], entry.first_lba);
    write_u32(&mut slice[28..32], entry.size);
}

/// Find a directory index by component name, if present.
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

/// Count free data LBAs in the free-map (`data_start..total_lbas`).
pub fn count_free_blocks(map: &[u8; SECTOR_SIZE], sb: &Superblock) -> u32 {
    let mut free = 0u32;
    let start = sb.data_start_lba.min(sb.total_lbas);
    for lba in start..sb.total_lbas {
        if is_free(map, lba) {
            free = free.saturating_add(1);
        }
    }
    free
}

/// True when a directory sector has no entries.
pub fn dir_is_empty(dir_sector: &[u8; SECTOR_SIZE]) -> bool {
    count_entries(dir_sector) == 0
}

/// Build a file or directory entry with the given name.
pub fn make_entry(name: &[u8], first_lba: u32, size: u32, is_dir: bool) -> DirEntry {
    let mut entry = DirEntry::empty();
    let name_len = name.len().min(NAME_LEN);
    entry.name[..name_len].copy_from_slice(&name[..name_len]);
    entry.name_len = name_len as u8;
    entry.flags = if is_dir { FLAG_DIR } else { 0 };
    entry.first_lba = first_lba;
    entry.size = size;
    entry
}

/// Absolute LBA for file byte `offset` given a contiguous extent at `first_lba`.
pub fn file_lba_for_offset(first_lba: u32, offset: u32) -> u32 {
    first_lba.saturating_add(offset / SECTOR_SIZE as u32)
}

/// Byte offset within a sector for file byte `offset`.
pub fn sector_offset(offset: u32) -> usize {
    (offset as usize) % SECTOR_SIZE
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
        let sb = Superblock::new();
        let mut sector = [0u8; SECTOR_SIZE];
        encode_superblock(&mut sector, &sb);
        assert!(is_formatted(&sector));
        assert!(!is_legacy_v1(&sector));
        assert_eq!(decode_superblock(&sector), Some(sb));
    }

    #[test]
    fn dir_entry_round_trip() {
        let mut dir = [0u8; SECTOR_SIZE];
        let entry = make_entry(b"hello", 9, 42, false);
        encode_dir_entry(&mut dir, 1, &entry);
        let decoded = decode_dir_entry(&dir, 1);
        assert_eq!(decoded.name_slice(), b"hello");
        assert_eq!(decoded.first_lba, 9);
        assert_eq!(decoded.size, 42);
        assert!(decoded.is_file());
        assert!(!decoded.is_dir());
    }

    #[test]
    fn dir_flag_round_trip() {
        let mut dir = [0u8; SECTOR_SIZE];
        let entry = make_entry(b"cfg", 5, 0, true);
        encode_dir_entry(&mut dir, 0, &entry);
        let decoded = decode_dir_entry(&dir, 0);
        assert!(decoded.is_dir());
        assert_eq!(decoded.name_slice(), b"cfg");
    }

    #[test]
    fn free_map_alloc_free() {
        let sb = Superblock::new();
        let mut map = [0u8; SECTOR_SIZE];
        encode_free_map_fresh(&mut map, &sb);
        assert!(!is_free(&map, 0));
        assert!(!is_free(&map, SUPERBLOCK_LBA));
        assert!(!is_free(&map, FREE_MAP_LBA));
        assert!(!is_free(&map, ROOT_DIR_LBA));
        assert!(is_free(&map, DATA_START_LBA));

        let a = alloc_contiguous(&mut map, &sb, 3).expect("alloc");
        assert_eq!(a, DATA_START_LBA);
        assert!(!is_free(&map, a));
        assert!(!is_free(&map, a + 2));
        assert!(is_free(&map, a + 3));

        free_contiguous(&mut map, a, 3);
        assert!(is_free(&map, a));
    }

    #[test]
    fn split_path_variants() {
        assert_eq!(split_path(b"").unwrap().count, 0);
        assert_eq!(split_path(b"/").unwrap().count, 0);
        let p = split_path(b"ping").unwrap();
        assert_eq!(p.count, 1);
        assert_eq!(p.component(0), Some(&b"ping"[..]));
        let p = split_path(b"/config/net").unwrap();
        assert_eq!(p.count, 2);
        assert_eq!(p.component(0), Some(&b"config"[..]));
        assert_eq!(p.component(1), Some(&b"net"[..]));
        assert!(matches!(
            split_path(b"/a/../../b"),
            Err(PathError::InvalidByte)
        ));
        assert!(matches!(
            split_path(b"/this-name-is-way-too-long-for-entry"),
            Err(PathError::NameTooLong)
        ));
    }

    #[test]
    fn file_sectors_math() {
        let mut e = make_entry(b"f", 4, 0, false);
        assert_eq!(e.file_sectors(), 1);
        e.size = 512;
        assert_eq!(e.file_sectors(), 1);
        e.size = 513;
        assert_eq!(e.file_sectors(), 2);
        e.size = MAX_FILE_SECTORS * SECTOR_SIZE as u32;
        assert_eq!(e.file_sectors(), MAX_FILE_SECTORS);
    }

    #[test]
    fn offset_helpers() {
        assert_eq!(file_lba_for_offset(10, 0), 10);
        assert_eq!(file_lba_for_offset(10, 512), 11);
        assert_eq!(sector_offset(513), 1);
    }

    #[test]
    fn count_free_after_alloc() {
        let sb = Superblock::new();
        let mut map = [0u8; SECTOR_SIZE];
        encode_free_map_fresh(&mut map, &sb);
        let before = count_free_blocks(&map, &sb);
        let _ = alloc_contiguous(&mut map, &sb, 2).unwrap();
        assert_eq!(count_free_blocks(&map, &sb), before - 2);
    }
}
