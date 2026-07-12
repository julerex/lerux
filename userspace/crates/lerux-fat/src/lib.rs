//! Minimal FAT16 layout helpers for lerux Phase 44.
//!
//! Supports a fixed small-volume layout suitable for QEMU `disk.img` (4 MiB):
//! - 512-byte sectors, 1 sector per cluster
//! - 1 reserved sector (boot + BPB)
//! - 2 FATs × 32 sectors
//! - 512 root directory entries (32 sectors)
//! - data area from LBA 97
//!
//! File size is limited to one cluster (512 bytes) by the server for v1,
//! matching the practical IPC payload (`MAX_FS_DATA` = 448). Root only; 8.3 names.

#![no_std]

use lerux_interface_types::SECTOR_SIZE;

/// Boot / BPB sector.
pub const BOOT_LBA: u32 = 0;

/// Sectors per FAT (fixed layout for 4 MiB images).
pub const FAT_SECTORS: u32 = 32;

/// Number of FATs.
pub const NUM_FATS: u8 = 2;

/// Root directory entries (FAT16 fixed root).
pub const ROOT_ENTRIES: u16 = 512;

/// Root directory size in sectors.
pub const ROOT_SECTORS: u32 = (ROOT_ENTRIES as u32 * 32) / SECTOR_SIZE as u32;

/// First LBA of FAT #1.
pub const FAT1_LBA: u32 = 1;

/// First LBA of FAT #2.
pub const FAT2_LBA: u32 = FAT1_LBA + FAT_SECTORS;

/// First LBA of the root directory.
pub const ROOT_LBA: u32 = FAT2_LBA + FAT_SECTORS;

/// First LBA of the data area (cluster 2).
pub const DATA_LBA: u32 = ROOT_LBA + ROOT_SECTORS;

/// Total sectors for a 4 MiB image.
pub const TOTAL_SECTORS: u32 = 4 * 1024 * 1024 / SECTOR_SIZE as u32;

/// Directory entry size.
pub const DIR_ENTRY_SIZE: usize = 32;

/// Entries per root directory sector.
pub const ENTRIES_PER_SECTOR: usize = SECTOR_SIZE / DIR_ENTRY_SIZE;

/// Maximum open-file handles tracked by the server (not part of on-disk format).
pub const MAX_HANDLES: usize = 16;

/// End-of-chain marker (FAT16).
pub const EOC: u16 = 0xFFFF;

/// Free cluster marker.
pub const FREE: u16 = 0x0000;

/// Media descriptor in FAT[0] low byte.
pub const MEDIA: u8 = 0xF8;

/// Parsed BIOS Parameter Block fields we care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bpb {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entries: u16,
    pub total_sectors: u32,
    pub fat_sectors: u16,
    pub root_lba: u32,
    pub data_lba: u32,
    pub fat1_lba: u32,
}

impl Bpb {
    /// Fixed layout used by [`encode_boot_sector`].
    pub const fn fixed() -> Self {
        Self {
            bytes_per_sector: SECTOR_SIZE as u16,
            sectors_per_cluster: 1,
            reserved_sectors: 1,
            num_fats: NUM_FATS,
            root_entries: ROOT_ENTRIES,
            total_sectors: TOTAL_SECTORS,
            fat_sectors: FAT_SECTORS as u16,
            root_lba: ROOT_LBA,
            data_lba: DATA_LBA,
            fat1_lba: FAT1_LBA,
        }
    }

    pub fn root_sectors(&self) -> u32 {
        (u32::from(self.root_entries) * 32) / u32::from(self.bytes_per_sector)
    }

    pub fn cluster_to_lba(&self, cluster: u16) -> u32 {
        self.data_lba + u32::from(cluster.saturating_sub(2)) * u32::from(self.sectors_per_cluster)
    }

    pub fn max_cluster(&self) -> u16 {
        let data_sectors = self.total_sectors.saturating_sub(self.data_lba);
        let clusters = data_sectors / u32::from(self.sectors_per_cluster);
        // Cluster numbers: 2 ..= (clusters + 1)
        2u16.saturating_add(clusters.saturating_sub(1) as u16)
    }
}

/// Returns true if `sector` looks like a FAT12/16 boot sector with our geometry.
pub fn is_fat_boot(sector: &[u8; SECTOR_SIZE]) -> bool {
    if sector[510] != 0x55 || sector[511] != 0xAA {
        return false;
    }
    // Jump instruction
    if sector[0] != 0xEB && sector[0] != 0xE9 {
        return false;
    }
    let bps = read_u16(&sector[11..13]);
    if bps != SECTOR_SIZE as u16 {
        return false;
    }
    let spc = sector[13];
    if spc == 0 {
        return false;
    }
    let fats = sector[16];
    if fats == 0 {
        return false;
    }
    // FAT32 has fat_size_16 == 0; we only accept FAT12/16.
    let fat_sz16 = read_u16(&sector[22..24]);
    fat_sz16 != 0
}

/// Decode BPB from a boot sector. Returns [`None`] if not a usable FAT12/16 volume.
pub fn decode_bpb(sector: &[u8; SECTOR_SIZE]) -> Option<Bpb> {
    if !is_fat_boot(sector) {
        return None;
    }
    let bytes_per_sector = read_u16(&sector[11..13]);
    let sectors_per_cluster = sector[13];
    let reserved_sectors = read_u16(&sector[14..16]);
    let num_fats = sector[16];
    let root_entries = read_u16(&sector[17..19]);
    let total_sectors_16 = read_u16(&sector[19..21]);
    let fat_sectors = read_u16(&sector[22..24]);
    let total_sectors_32 = read_u32(&sector[32..36]);
    let total_sectors = if total_sectors_16 != 0 {
        u32::from(total_sectors_16)
    } else {
        total_sectors_32
    };
    if bytes_per_sector == 0
        || sectors_per_cluster == 0
        || num_fats == 0
        || fat_sectors == 0
        || root_entries == 0
        || total_sectors == 0
    {
        return None;
    }
    let fat1_lba = u32::from(reserved_sectors);
    let root_lba = fat1_lba + u32::from(fat_sectors) * u32::from(num_fats);
    let root_bytes = u32::from(root_entries) * 32;
    let root_sectors = root_bytes.div_ceil(u32::from(bytes_per_sector));
    let data_lba = root_lba + root_sectors;
    Some(Bpb {
        bytes_per_sector,
        sectors_per_cluster,
        reserved_sectors,
        num_fats,
        root_entries,
        total_sectors,
        fat_sectors,
        root_lba,
        data_lba,
        fat1_lba,
    })
}

/// Encode a fixed FAT16 boot sector for a 4 MiB volume.
pub fn encode_boot_sector(sector: &mut [u8; SECTOR_SIZE]) {
    sector.fill(0);
    // jmp short + nop
    sector[0] = 0xEB;
    sector[1] = 0x3C;
    sector[2] = 0x90;
    sector[3..11].copy_from_slice(b"LERUXFAT");
    write_u16(&mut sector[11..13], SECTOR_SIZE as u16);
    sector[13] = 1; // sectors per cluster
    write_u16(&mut sector[14..16], 1); // reserved
    sector[16] = NUM_FATS;
    write_u16(&mut sector[17..19], ROOT_ENTRIES);
    write_u16(&mut sector[19..21], TOTAL_SECTORS as u16);
    sector[21] = MEDIA;
    write_u16(&mut sector[22..24], FAT_SECTORS as u16);
    write_u16(&mut sector[24..26], 32); // sectors per track (dummy)
    write_u16(&mut sector[26..28], 2); // heads (dummy)
                                       // BPB_HiddSec / TotSec32 left zero (TotSec16 set)
                                       // Extended boot signature
    sector[38] = 0x29;
    write_u32(&mut sector[39..43], 0x1234_5678);
    sector[43..54].copy_from_slice(b"LERUX_DISK ");
    sector[54..62].copy_from_slice(b"FAT16   ");
    sector[510] = 0x55;
    sector[511] = 0xAA;
}

/// First FAT sector: media descriptor + EOC for cluster 1, rest free.
pub fn encode_fat_first_sector(sector: &mut [u8; SECTOR_SIZE]) {
    sector.fill(0);
    // FAT[0] = media | 0xFF00, FAT[1] = EOC
    sector[0] = MEDIA;
    sector[1] = 0xFF;
    sector[2] = 0xFF;
    sector[3] = 0xFF;
}

/// Encode a zero sector (subsequent FAT / empty root sectors).
pub fn encode_zero_sector(sector: &mut [u8; SECTOR_SIZE]) {
    sector.fill(0);
}

/// Total sectors written during format after the boot sector (2×FAT + root).
pub fn format_payload_sectors() -> u32 {
    u32::from(NUM_FATS) * FAT_SECTORS + ROOT_SECTORS
}

/// LBA for format payload index `i` (0-based after boot).
pub fn format_payload_lba(i: u32) -> u32 {
    1 + i
}

/// Whether format payload index `i` is the first sector of a FAT copy.
pub fn format_payload_is_fat_head(i: u32) -> bool {
    i == 0 || i == FAT_SECTORS
}

/// 8.3 short name (11 bytes, space-padded).
pub type ShortName = [u8; 11];

/// Convert a path/name (possibly with leading `/`) to an 8.3 short name.
///
/// Accepts up to 8 char base + optional `.` + up to 3 char extension.
/// Lowercase ASCII is uppercased. Returns [`None`] if the name is invalid.
pub fn path_to_short_name(path: &[u8]) -> Option<ShortName> {
    let mut name = path;
    while name.first() == Some(&b'/') {
        name = &name[1..];
    }
    if name.is_empty() || name.len() > 12 {
        return None;
    }
    let mut out = [b' '; 11];
    let (base, ext) = match name.iter().position(|&b| b == b'.') {
        Some(dot) => (&name[..dot], &name[dot + 1..]),
        None => (name, &name[0..0]),
    };
    if base.is_empty() || base.len() > 8 || ext.len() > 3 {
        return None;
    }
    for (i, &b) in base.iter().enumerate() {
        out[i] = to_upper_83(b)?;
    }
    for (i, &b) in ext.iter().enumerate() {
        out[8 + i] = to_upper_83(b)?;
    }
    Some(out)
}

fn to_upper_83(b: u8) -> Option<u8> {
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

/// Display name from an 8.3 entry (trimmed base + optional `.ext`), into `out`.
/// Returns length written (max 12).
pub fn short_name_to_display(name: &ShortName, out: &mut [u8; 12]) -> u8 {
    out.fill(0);
    let mut len = 0usize;
    for &b in &name[..8] {
        if b == b' ' {
            break;
        }
        out[len] = b;
        len += 1;
    }
    let mut ext_len = 0usize;
    for &b in &name[8..] {
        if b == b' ' {
            break;
        }
        ext_len += 1;
    }
    if ext_len > 0 {
        out[len] = b'.';
        len += 1;
        for i in 0..ext_len {
            out[len] = name[8 + i];
            len += 1;
        }
    }
    len as u8
}

/// On-disk directory entry fields we use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirEntry {
    pub name: ShortName,
    pub attr: u8,
    pub first_cluster: u16,
    pub size: u32,
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            name: [0; 11],
            attr: 0,
            first_cluster: 0,
            size: 0,
        }
    }

    pub fn is_free(&self) -> bool {
        self.name[0] == 0x00 || self.name[0] == 0xE5
    }

    pub fn is_end(&self) -> bool {
        self.name[0] == 0x00
    }

    pub fn is_volume_or_lfn(&self) -> bool {
        self.attr & 0x08 != 0 || self.attr & 0x0F == 0x0F
    }

    pub fn is_dir(&self) -> bool {
        self.attr & 0x10 != 0
    }
}

/// Decode directory entry at `index` within one sector (0..15).
pub fn decode_dir_entry(sector: &[u8; SECTOR_SIZE], index: usize) -> DirEntry {
    if index >= ENTRIES_PER_SECTOR {
        return DirEntry::empty();
    }
    let off = index * DIR_ENTRY_SIZE;
    let s = &sector[off..off + DIR_ENTRY_SIZE];
    let mut name = [0u8; 11];
    name.copy_from_slice(&s[0..11]);
    let attr = s[11];
    let cluster_hi = read_u16(&s[20..22]);
    let cluster_lo = read_u16(&s[26..28]);
    let first_cluster = ((cluster_hi as u32) << 16 | u32::from(cluster_lo)) as u16;
    let size = read_u32(&s[28..32]);
    DirEntry {
        name,
        attr,
        first_cluster,
        size,
    }
}

/// Encode directory entry at `index` within one sector.
pub fn encode_dir_entry(sector: &mut [u8; SECTOR_SIZE], index: usize, entry: &DirEntry) {
    if index >= ENTRIES_PER_SECTOR {
        return;
    }
    let off = index * DIR_ENTRY_SIZE;
    let s = &mut sector[off..off + DIR_ENTRY_SIZE];
    s.fill(0);
    s[0..11].copy_from_slice(&entry.name);
    s[11] = entry.attr;
    write_u16(&mut s[20..22], 0);
    write_u16(&mut s[26..28], entry.first_cluster);
    write_u32(&mut s[28..32], entry.size);
}

/// Mark entry free (0xE5).
pub fn clear_dir_entry(sector: &mut [u8; SECTOR_SIZE], index: usize) {
    if index >= ENTRIES_PER_SECTOR {
        return;
    }
    let off = index * DIR_ENTRY_SIZE;
    sector[off] = 0xE5;
}

/// Absolute root-directory slot index → (sector_offset_from_root, entry_in_sector).
pub fn root_slot_location(slot: u16) -> (u32, usize) {
    let entries_per = ENTRIES_PER_SECTOR as u16;
    let sec = u32::from(slot / entries_per);
    let idx = (slot % entries_per) as usize;
    (sec, idx)
}

/// Read a FAT16 entry from a cached FAT sector image.
/// `cluster` is the cluster number; `fat_sector` holds one sector of the FAT starting at
/// cluster index `base_cluster` (base_cluster = sector_index * 256).
pub fn fat_get(fat_sector: &[u8; SECTOR_SIZE], cluster_in_sector: usize) -> u16 {
    let off = cluster_in_sector * 2;
    read_u16(&fat_sector[off..off + 2])
}

/// Write a FAT16 entry into a cached FAT sector.
pub fn fat_set(fat_sector: &mut [u8; SECTOR_SIZE], cluster_in_sector: usize, value: u16) {
    let off = cluster_in_sector * 2;
    write_u16(&mut fat_sector[off..off + 2], value);
}

/// Which FAT sector (0-based within FAT) and index within that sector for `cluster`.
pub fn fat_sector_index(cluster: u16) -> (u32, usize) {
    let idx = u32::from(cluster);
    (idx / 256, (idx % 256) as usize)
}

pub fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub fn write_u16(bytes: &mut [u8], value: u16) {
    let le = value.to_le_bytes();
    bytes[0] = le[0];
    bytes[1] = le[1];
}

pub fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub fn write_u32(bytes: &mut [u8], value: u32) {
    let le = value.to_le_bytes();
    bytes[..4].copy_from_slice(&le);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_round_trip() {
        let mut sector = [0u8; SECTOR_SIZE];
        encode_boot_sector(&mut sector);
        assert!(is_fat_boot(&sector));
        let bpb = decode_bpb(&sector).expect("bpb");
        assert_eq!(bpb.bytes_per_sector, 512);
        assert_eq!(bpb.sectors_per_cluster, 1);
        assert_eq!(bpb.fat_sectors, FAT_SECTORS as u16);
        assert_eq!(bpb.root_lba, ROOT_LBA);
        assert_eq!(bpb.data_lba, DATA_LBA);
    }

    #[test]
    fn short_name_ping() {
        let n = path_to_short_name(b"ping").unwrap();
        assert_eq!(&n[..4], b"PING");
        assert_eq!(&n[4..], b"       ");
        let mut disp = [0u8; 12];
        let len = short_name_to_display(&n, &mut disp);
        assert_eq!(&disp[..len as usize], b"PING");
    }

    #[test]
    fn short_name_with_ext() {
        let n = path_to_short_name(b"/boot.log").unwrap();
        assert_eq!(&n[..4], b"BOOT");
        assert_eq!(&n[8..11], b"LOG");
    }

    #[test]
    fn dir_entry_round_trip() {
        let mut sector = [0u8; SECTOR_SIZE];
        let mut e = DirEntry::empty();
        e.name = path_to_short_name(b"ping").unwrap();
        e.attr = 0x20;
        e.first_cluster = 3;
        e.size = 14;
        encode_dir_entry(&mut sector, 2, &e);
        let d = decode_dir_entry(&sector, 2);
        assert_eq!(d.first_cluster, 3);
        assert_eq!(d.size, 14);
        assert_eq!(d.name, e.name);
    }

    #[test]
    fn fat_entry_helpers() {
        let mut sec = [0u8; SECTOR_SIZE];
        fat_set(&mut sec, 2, EOC);
        assert_eq!(fat_get(&sec, 2), EOC);
        assert_eq!(fat_sector_index(300), (1, 44));
    }
}
