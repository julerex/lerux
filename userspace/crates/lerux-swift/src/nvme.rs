//! Read-only view of one named file. Writes and format are rejected.
//!
//! The Windows volume on the hynix SSD is left as it is. A FAT root file and
//! an NTFS resident file are both accepted. Sector 0 of a GUID partition table
//! is a protective master boot record, type `0xEE`, and the header is at LBA 1.
//! Each partition is read at its starting LBA. A boot sector that is none of
//! these is `nvme: raw`. A sector read that does not return is `nvme: admin`.

pub const NAMED_FILE: &[u8] = b"LERUX.TXT";

/// IO opcodes. Admin create-queue uses 0x01 on the admin queue and is not this.
pub const IO_READ: u8 = 0x02;
pub const IO_WRITE: u8 = 0x01;
pub const ADMIN_FORMAT: u8 = 0x80;

#[expect(
    clippy::large_enum_variant,
    reason = "file bytes stay inline; this crate has no allocator"
)]
pub enum DiskReport {
    Bytes([u8; 256], usize),
    Missing,
    Absent,
    Regs,
    Enable,
    Admin,
    Raw,
}

pub fn io_allowed(opcode: u8) -> bool {
    opcode == IO_READ
}

pub fn io_rejected(opcode: u8) -> bool {
    opcode == IO_WRITE || opcode == ADMIN_FORMAT
}

pub fn write_result(page: &mut [u8], payload: &[u8]) {
    let n = payload.len().min(256);
    if page.len() < 2 + n {
        return;
    }
    page[0..2].copy_from_slice(&(n as u16).to_le_bytes());
    page[2..2 + n].copy_from_slice(&payload[..n]);
}

pub fn report_bytes(report: &DiskReport) -> &[u8] {
    match report {
        DiskReport::Bytes(buf, len) => &buf[..*len],
        DiskReport::Missing => b"nvme: missing",
        DiskReport::Absent => b"nvme: absent",
        DiskReport::Regs => b"nvme: regs",
        DiskReport::Enable => b"nvme: enable",
        DiskReport::Admin => b"nvme: admin",
        DiskReport::Raw => b"nvme: raw",
    }
}

/// Read [`NAMED_FILE`] through `read`, which returns one 512-byte sector.
///
/// `None` from `read` is [`DiskReport::Admin`]. A protective master boot record
/// is followed to the partition table. A boot sector that is neither NTFS nor a
/// 512-byte FAT volume, and not that record, is [`DiskReport::Raw`].
pub fn load_named(mut read: impl FnMut(u64) -> Option<[u8; 512]>) -> DiskReport {
    let Some(boot) = read(0) else {
        return DiskReport::Admin;
    };
    if is_ntfs(&boot) {
        return load_ntfs(&boot, &mut read);
    }
    if is_fat(&boot) {
        return load_fat(&boot, &mut read);
    }
    if is_protective_mbr(&boot) {
        return load_gpt(&mut read);
    }
    DiskReport::Raw
}

fn is_ntfs(boot: &[u8; 512]) -> bool {
    boot[3..11] == *b"NTFS    "
}

fn is_fat(boot: &[u8; 512]) -> bool {
    boot[510] == 0x55 && boot[511] == 0xaa && u16::from_le_bytes([boot[11], boot[12]]) == 512
}

/// Partition type `0xEE` in any of the four master-boot-record slots.
fn is_protective_mbr(boot: &[u8; 512]) -> bool {
    if boot[510] != 0x55 || boot[511] != 0xaa {
        return false;
    }
    (0..4).any(|slot| boot[446 + slot * 16 + 4] == 0xee)
}

fn load_gpt(read: &mut impl FnMut(u64) -> Option<[u8; 512]>) -> DiskReport {
    let Some(header) = read(1) else {
        return DiskReport::Admin;
    };
    if &header[..8] != b"EFI PART" {
        return DiskReport::Raw;
    }
    let header_size = u32_at(&header, 12);
    let my_lba = u64_at(&header, 24);
    let entry_lba = u64_at(&header, 72);
    let count = u32_at(&header, 80);
    let entry_size = u32_at(&header, 84);
    if !(92..=512).contains(&header_size)
        || my_lba != 1
        || entry_lba == 0
        || entry_size != 128
        || count == 0
    {
        return DiskReport::Raw;
    }
    let count = count.min(128);
    let mut saw_fs = false;
    for index in 0..u64::from(count) {
        let Some(entry) = read_gpt_entry(read, entry_lba, index) else {
            return DiskReport::Admin;
        };
        if entry[..16] == [0; 16] {
            continue;
        }
        let start = u64_at(&entry, 32);
        if start == 0 {
            continue;
        }
        match load_volume(start, read) {
            DiskReport::Bytes(buf, len) => return DiskReport::Bytes(buf, len),
            DiskReport::Missing => saw_fs = true,
            DiskReport::Admin => return DiskReport::Admin,
            DiskReport::Raw | DiskReport::Absent | DiskReport::Regs | DiskReport::Enable => {}
        }
    }
    if saw_fs {
        DiskReport::Missing
    } else {
        DiskReport::Raw
    }
}

fn load_volume(start: u64, read: &mut impl FnMut(u64) -> Option<[u8; 512]>) -> DiskReport {
    let Some(boot) = read(start) else {
        return DiskReport::Admin;
    };
    let mut at = |lba: u64| {
        let abs = start.checked_add(lba)?;
        read(abs)
    };
    if is_ntfs(&boot) {
        return load_ntfs(&boot, &mut at);
    }
    if is_fat(&boot) {
        return load_fat(&boot, &mut at);
    }
    DiskReport::Raw
}

fn read_gpt_entry(
    read: &mut impl FnMut(u64) -> Option<[u8; 512]>,
    entry_lba: u64,
    index: u64,
) -> Option<[u8; 128]> {
    let lba = entry_lba.checked_add(index / 4)?;
    let sector = read(lba)?;
    let at = (index % 4) as usize * 128;
    let mut entry = [0u8; 128];
    entry.copy_from_slice(&sector[at..at + 128]);
    Some(entry)
}

fn u32_at(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
}

fn u64_at(buf: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(buf[at..at + 8].try_into().unwrap())
}

fn load_fat(boot: &[u8; 512], read: &mut impl FnMut(u64) -> Option<[u8; 512]>) -> DiskReport {
    let reserved = u16::from_le_bytes([boot[14], boot[15]]) as u64;
    let fats = boot[16] as u64;
    let root_entries = u16::from_le_bytes([boot[17], boot[18]]) as u64;
    let fat_sectors = u16::from_le_bytes([boot[22], boot[23]]) as u64;
    if fats == 0 || root_entries == 0 || fat_sectors == 0 {
        return DiskReport::Raw;
    }
    let root_lba = reserved + fats * fat_sectors;
    let root_sectors = root_entries * 32 / 512;
    let want = fat_name(NAMED_FILE);
    for sector in 0..root_sectors {
        let Some(dir) = read(root_lba + sector) else {
            return DiskReport::Admin;
        };
        for slot in 0..16 {
            let entry = &dir[slot * 32..slot * 32 + 32];
            if entry[0] == 0x00 {
                return DiskReport::Missing;
            }
            if entry[0] == 0xe5 || entry[11] == 0x0f {
                continue;
            }
            if entry[..11] == want {
                let cluster = u16::from_le_bytes([entry[26], entry[27]]) as u64;
                let size =
                    u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]) as usize;
                let data_lba = root_lba + root_sectors + cluster.saturating_sub(2);
                return read_bytes(read, data_lba, size.min(256));
            }
        }
    }
    DiskReport::Missing
}

fn read_bytes(
    read: &mut impl FnMut(u64) -> Option<[u8; 512]>,
    lba: u64,
    size: usize,
) -> DiskReport {
    if size == 0 {
        return DiskReport::Bytes([0; 256], 0);
    }
    let Some(sector) = read(lba) else {
        return DiskReport::Admin;
    };
    let mut buf = [0u8; 256];
    let n = size.min(256).min(sector.len());
    buf[..n].copy_from_slice(&sector[..n]);
    DiskReport::Bytes(buf, n)
}

fn fat_name(name: &[u8]) -> [u8; 11] {
    let mut out = [b' '; 11];
    let (stem, ext) = match name.iter().position(|byte| *byte == b'.') {
        Some(dot) => (&name[..dot], &name[dot + 1..]),
        None => (name, b"".as_slice()),
    };
    let n = stem.len().min(8);
    out[..n].copy_from_slice(&stem[..n]);
    let n = ext.len().min(3);
    out[8..8 + n].copy_from_slice(&ext[..n]);
    for byte in &mut out {
        if byte.is_ascii_lowercase() {
            *byte = byte.to_ascii_uppercase();
        }
    }
    out
}

fn load_ntfs(boot: &[u8; 512], read: &mut impl FnMut(u64) -> Option<[u8; 512]>) -> DiskReport {
    let bps = u16::from_le_bytes([boot[0x0b], boot[0x0c]]);
    if bps != 512 {
        return DiskReport::Raw;
    }
    let spc = boot[0x0d] as u64;
    if spc == 0 {
        return DiskReport::Raw;
    }
    let mft_lcn = u64::from_le_bytes(boot[0x30..0x38].try_into().unwrap());
    let record = mft_record_bytes(boot[0x40], spc);
    if record == 0 || record > 4096 {
        return DiskReport::Raw;
    }
    let sectors = (record / 512) as u64;
    let encoded = utf16(NAMED_FILE);
    let want = &encoded[..NAMED_FILE.len()];
    for index in 0..64u64 {
        let lba = mft_lcn * spc + index * sectors;
        let mut raw = [0u8; 4096];
        for s in 0..sectors {
            let Some(sector) = read(lba + s) else {
                return DiskReport::Admin;
            };
            let at = s as usize * 512;
            raw[at..at + 512].copy_from_slice(&sector);
        }
        if let Some(found) = file_in_record(&raw[..record], want) {
            return found;
        }
    }
    DiskReport::Missing
}

fn mft_record_bytes(encoded: u8, spc: u64) -> usize {
    let encoded = encoded as i8;
    if encoded > 0 {
        (encoded as u64 * spc * 512) as usize
    } else {
        1usize << (encoded.unsigned_abs())
    }
}

fn file_in_record(raw: &[u8], want: &[u16]) -> Option<DiskReport> {
    if raw.len() < 0x40 || &raw[..4] != b"FILE" {
        return None;
    }
    let usa_off = u16::from_le_bytes([raw[4], raw[5]]) as usize;
    let usa_count = u16::from_le_bytes([raw[6], raw[7]]) as usize;
    let mut fixed = [0u8; 4096];
    if raw.len() > fixed.len() {
        return None;
    }
    fixed[..raw.len()].copy_from_slice(raw);
    apply_fixups(&mut fixed[..raw.len()], usa_off, usa_count)?;
    let flags = u16::from_le_bytes([fixed[0x16], fixed[0x17]]);
    if flags & 1 == 0 {
        return None;
    }
    let mut attr = u16::from_le_bytes([fixed[0x14], fixed[0x15]]) as usize;
    let mut named = false;
    let mut data = None;
    while attr + 8 <= raw.len() {
        let kind = u32::from_le_bytes(fixed[attr..attr + 4].try_into().ok()?);
        let len = u32::from_le_bytes(fixed[attr + 4..attr + 8].try_into().ok()?) as usize;
        if kind == 0xffff_ffff || len < 8 || attr + len > raw.len() {
            break;
        }
        let nonresident = fixed[attr + 8];
        let name_len = fixed[attr + 9] as usize;
        if nonresident == 0 && name_len == 0 {
            let value_len =
                u32::from_le_bytes(fixed[attr + 0x10..attr + 0x14].try_into().ok()?) as usize;
            let value_off = u16::from_le_bytes([fixed[attr + 0x14], fixed[attr + 0x15]]) as usize;
            let start = attr + value_off;
            let end = start + value_len;
            if end <= attr + len {
                let value = &fixed[start..end];
                if kind == 0x30 && filename_is(value, want) {
                    named = true;
                }
                if kind == 0x80 {
                    data = Some((start, value_len));
                }
            }
        }
        attr += len;
    }
    if !named {
        return None;
    }
    let (start, len) = data?;
    let n = len.min(256);
    let mut buf = [0u8; 256];
    buf[..n].copy_from_slice(&fixed[start..start + n]);
    Some(DiskReport::Bytes(buf, n))
}

fn filename_is(value: &[u8], want: &[u16]) -> bool {
    if value.len() < 0x42 {
        return false;
    }
    let n = value[0x40] as usize;
    if n != want.len() || value.len() < 0x42 + n * 2 {
        return false;
    }
    for (i, unit) in want.iter().enumerate() {
        let got = u16::from_le_bytes([value[0x42 + i * 2], value[0x43 + i * 2]]);
        if got != *unit {
            return false;
        }
    }
    true
}

fn apply_fixups(raw: &mut [u8], usa_off: usize, usa_count: usize) -> Option<()> {
    if usa_count < 2 || usa_off + usa_count * 2 > raw.len() {
        return None;
    }
    let signature = u16::from_le_bytes([raw[usa_off], raw[usa_off + 1]]);
    for sector in 1..usa_count {
        let at = sector * 512 - 2;
        if at + 2 > raw.len() {
            return None;
        }
        let found = u16::from_le_bytes([raw[at], raw[at + 1]]);
        if found != signature {
            return None;
        }
        let src = usa_off + sector * 2;
        raw[at] = raw[src];
        raw[at + 1] = raw[src + 1];
    }
    Some(())
}

fn utf16(name: &[u8]) -> [u16; 16] {
    let mut out = [0u16; 16];
    for (i, byte) in name.iter().enumerate() {
        out[i] = u16::from(*byte);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_lines_name_the_failure() {
        assert_eq!(report_bytes(&DiskReport::Regs), b"nvme: regs");
        assert_eq!(report_bytes(&DiskReport::Enable), b"nvme: enable");
        assert_eq!(report_bytes(&DiskReport::Admin), b"nvme: admin");
        assert_eq!(report_bytes(&DiskReport::Raw), b"nvme: raw");
        assert_eq!(report_bytes(&DiskReport::Absent), b"nvme: absent");
        assert_eq!(report_bytes(&DiskReport::Missing), b"nvme: missing");
    }

    #[test]
    fn a_missed_sector_is_admin_and_a_bare_signature_is_raw() {
        assert_eq!(report_bytes(&load_named(|_| None)), b"nvme: admin");
        let mut boot = [0u8; 512];
        boot[510] = 0x55;
        boot[511] = 0xaa;
        let report = load_named(|lba| (lba == 0).then_some(boot));
        assert_eq!(report_bytes(&report), b"nvme: raw");
    }

    #[test]
    fn a_protective_mbr_without_a_gpt_header_is_raw_and_a_missed_header_is_admin() {
        let mut disk = [[0u8; 512]; 2];
        disk[0] = protective_mbr();
        let report = load_named(|lba| disk.get(lba as usize).copied());
        assert_eq!(report_bytes(&report), b"nvme: raw");
        let report = load_named(|lba| (lba == 0).then_some(disk[0]));
        assert_eq!(report_bytes(&report), b"nvme: admin");
    }

    #[test]
    fn reads_the_ntfs_file_behind_the_efi_partition() {
        let mut disk = [[0u8; 512]; 11];
        disk[0] = protective_mbr();
        disk[1] = gpt_header(2, 2);
        write_entry(&mut disk[2][..128], 3);
        write_entry(&mut disk[2][128..256], 8);
        disk[3] = fat_boot();
        disk[8] = ntfs_boot(1);
        let record = file_record();
        disk[9].copy_from_slice(&record[..512]);
        disk[10].copy_from_slice(&record[512..]);
        let report = load_named(|lba| disk.get(lba as usize).copied());
        assert_eq!(report_bytes(&report), b"hello");
    }

    #[test]
    fn rejects_write_and_format() {
        assert!(io_allowed(IO_READ));
        assert!(io_rejected(IO_WRITE));
        assert!(io_rejected(ADMIN_FORMAT));
    }

    #[test]
    fn reads_a_fat_root_file() {
        let mut disk = [[0u8; 512]; 8];
        disk[0][11] = 0;
        disk[0][12] = 2;
        disk[0][13] = 1;
        disk[0][14] = 1;
        disk[0][16] = 2;
        disk[0][17] = 16;
        disk[0][22] = 1;
        disk[0][510] = 0x55;
        disk[0][511] = 0xaa;
        let entry = &mut disk[3][..32];
        entry[..11].copy_from_slice(b"LERUX   TXT");
        entry[26] = 2;
        entry[28] = 5;
        disk[4][..5].copy_from_slice(b"hello");
        let report = load_named(|lba| disk.get(lba as usize).copied());
        assert_eq!(report_bytes(&report), b"hello");
    }

    #[test]
    fn reads_a_resident_ntfs_file_and_does_not_need_a_format() {
        let mut disk = [[0u8; 512]; 6];
        disk[0][3..11].copy_from_slice(b"NTFS    ");
        disk[0][0x0b] = 0x00;
        disk[0][0x0c] = 0x02;
        disk[0][0x0d] = 1;
        disk[0][0x30] = 1;
        disk[0][0x40] = (-10i8) as u8;
        let record = file_record();
        disk[1].copy_from_slice(&record[..512]);
        disk[2].copy_from_slice(&record[512..]);
        let report = load_named(|lba| disk.get(lba as usize).copied());
        assert_eq!(report_bytes(&report), b"hello");
    }

    fn protective_mbr() -> [u8; 512] {
        let mut boot = [0u8; 512];
        boot[450] = 0xee;
        boot[510] = 0x55;
        boot[511] = 0xaa;
        boot
    }

    fn gpt_header(entry_lba: u64, count: u32) -> [u8; 512] {
        let mut header = [0u8; 512];
        header[..8].copy_from_slice(b"EFI PART");
        header[12..16].copy_from_slice(&92u32.to_le_bytes());
        header[24..32].copy_from_slice(&1u64.to_le_bytes());
        header[72..80].copy_from_slice(&entry_lba.to_le_bytes());
        header[80..84].copy_from_slice(&count.to_le_bytes());
        header[84..88].copy_from_slice(&128u32.to_le_bytes());
        header
    }

    fn write_entry(entry: &mut [u8], start: u64) {
        entry[0] = 1;
        entry[32..40].copy_from_slice(&start.to_le_bytes());
    }

    fn fat_boot() -> [u8; 512] {
        let mut boot = [0u8; 512];
        boot[12] = 2;
        boot[13] = 1;
        boot[14] = 1;
        boot[16] = 2;
        boot[17] = 16;
        boot[22] = 1;
        boot[510] = 0x55;
        boot[511] = 0xaa;
        boot
    }

    fn ntfs_boot(mft_lcn: u64) -> [u8; 512] {
        let mut boot = [0u8; 512];
        boot[3..11].copy_from_slice(b"NTFS    ");
        boot[0x0c] = 0x02;
        boot[0x0d] = 1;
        boot[0x30..0x38].copy_from_slice(&mft_lcn.to_le_bytes());
        boot[0x40] = (-10i8) as u8;
        boot
    }

    fn file_record() -> [u8; 1024] {
        let mut raw = [0u8; 1024];
        raw[..4].copy_from_slice(b"FILE");
        raw[4] = 0x30;
        raw[6] = 3;
        raw[0x14] = 0x38;
        raw[0x16] = 1;
        raw[0x30] = 0x01;
        raw[0x31] = 0x00;
        raw[510] = 0x01;
        raw[511] = 0x00;
        raw[1022] = 0x01;
        raw[1023] = 0x00;
        raw[0x32] = 0x00;
        raw[0x33] = 0x00;
        let name_at = 0x38;
        write_attr(&mut raw, name_at, 0x30, &filename_value());
        let name_len =
            u32::from_le_bytes(raw[name_at + 4..name_at + 8].try_into().unwrap()) as usize;
        let data_at = name_at + name_len;
        write_attr(&mut raw, data_at, 0x80, b"hello");
        raw
    }

    fn filename_value() -> [u8; 0x42 + 18] {
        let mut value = [0u8; 0x42 + 18];
        value[0x40] = 9;
        for (i, byte) in b"LERUX.TXT".iter().enumerate() {
            value[0x42 + i * 2] = *byte;
        }
        value
    }

    fn write_attr(raw: &mut [u8], at: usize, kind: u32, value: &[u8]) {
        raw[at..at + 4].copy_from_slice(&kind.to_le_bytes());
        let header = 0x18;
        let len = header + value.len();
        raw[at + 4..at + 8].copy_from_slice(&(len as u32).to_le_bytes());
        raw[at + 0x10..at + 0x14].copy_from_slice(&(value.len() as u32).to_le_bytes());
        raw[at + 0x14] = header as u8;
        raw[at + header..at + header + value.len()].copy_from_slice(value);
    }
}
