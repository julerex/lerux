//! Sector, page, and packet parsers. None of these functions stores a result.

use crate::text::Buf;

const LVM_GUID: [u8; 16] = [
    0x79, 0xD3, 0xD6, 0xE6, 0x07, 0xF5, 0xC2, 0x44, 0xA2, 0x3C, 0x23, 0x8F, 0x2A, 0x3D, 0xF9, 0x28,
];
const RAID_GUID: [u8; 16] = [
    0x0F, 0x88, 0x9D, 0xA1, 0xFC, 0x05, 0x3B, 0x4D, 0xA0, 0x06, 0x74, 0x3F, 0x0F, 0x84, 0x91, 0x1E,
];

/// Filesystem or volume label a boot sector can name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsKind {
    /// File Allocation Table boot sector.
    Fat,
    /// New Technology File System boot sector.
    Ntfs,
    /// ext superblock magic.
    Ext4,
    /// Btrfs superblock magic.
    Btrfs,
    /// XFS superblock magic.
    Xfs,
    /// `LUKS` magic. The parser does not unlock it.
    Luks,
    /// Logical Volume Manager label.
    Lvm,
    /// Linux RAID superblock magic.
    Raid,
    /// None of the magics above.
    Unknown,
}

/// One GUID Partition Table the caller already read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gpt {
    /// Entries with a type and a non-zero start.
    pub partitions: usize,
    /// An entry uses the Linux logical-volume type.
    pub lvm: bool,
    /// An entry uses the Linux RAID type.
    pub raid: bool,
}

/// When a copy onto a second disk is allowed.
///
/// The source stays read-only. The destination name is the dedicated lerux
/// volume. A disk that still holds another operating system is not a
/// destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CopyPolicy {
    /// The image has a second disk.
    pub second_disk: bool,
    /// The destination name is the dedicated volume.
    pub dest_dedicated: bool,
    /// The source is opened read-only.
    pub source_read_only: bool,
}

/// One host and one port. A range is not an endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Endpoint {
    /// Four decimal octets.
    pub addr: [u8; 4],
    /// Port in `1..=65535`.
    pub port: u16,
}

/// `true` when every flag on `policy` is set.
#[must_use]
pub const fn allow_copy(policy: CopyPolicy) -> bool {
    policy.second_disk && policy.dest_dedicated && policy.source_read_only
}

/// Boot sector `sddf-fatfs` writes for the QEMU volume.
#[must_use]
pub fn qemu_fat_boot() -> [u8; 512] {
    let mut sector = [0u8; 512];
    sector[0] = 0xEB;
    sector[1] = 0x3C;
    sector[2] = 0x90;
    sector[3..11].copy_from_slice(b"LERUXFAT");
    sector[11..13].copy_from_slice(&512u16.to_le_bytes());
    sector[13] = 1;
    sector[14..16].copy_from_slice(&1u16.to_le_bytes());
    sector[16] = 2;
    sector[17..19].copy_from_slice(&512u16.to_le_bytes());
    sector[19..21].copy_from_slice(&8192u16.to_le_bytes());
    sector[21] = 0xF8;
    sector[22..24].copy_from_slice(&32u16.to_le_bytes());
    sector[510] = 0x55;
    sector[511] = 0xAA;
    sector
}

/// A 512-byte File Allocation Table boot sector with a real geometry.
#[must_use]
pub fn fat_boot_ok(sector: &[u8]) -> bool {
    if sector.len() < 512 || sector[510] != 0x55 || sector[511] != 0xAA {
        return false;
    }
    if sector[0] != 0xEB || sector[2] != 0x90 || sector[16] == 0 || sector[21] != 0xF8 {
        return false;
    }
    let bps = u16::from_le_bytes([sector[11], sector[12]]);
    let reserved = u16::from_le_bytes([sector[14], sector[15]]);
    let root = u16::from_le_bytes([sector[17], sector[18]]);
    let total = u16::from_le_bytes([sector[19], sector[20]]);
    bps == 512
        && sector[13].is_power_of_two()
        && sector[13] != 0
        && reserved != 0
        && root != 0
        && total >= 97
}

/// A New Technology File System boot sector.
#[must_use]
pub fn ntfs_boot(sector: &[u8]) -> bool {
    if sector.len() < 512 || sector[510] != 0x55 || sector[511] != 0xAA {
        return false;
    }
    if sector.get(3..11) != Some(b"NTFS    ".as_slice()) {
        return false;
    }
    u16::from_le_bytes([sector[11], sector[12]]) == 512
}

/// The first magic `bytes` names.
#[must_use]
pub fn identify(bytes: &[u8]) -> FsKind {
    if ntfs_boot(bytes) {
        return FsKind::Ntfs;
    }
    if fat_boot_ok(bytes) {
        return FsKind::Fat;
    }
    if has_prefix(bytes, b"XFSB") {
        return FsKind::Xfs;
    }
    if luks(bytes) {
        return FsKind::Luks;
    }
    if ext4(bytes) {
        return FsKind::Ext4;
    }
    if btrfs(bytes) {
        return FsKind::Btrfs;
    }
    if lvm_label(bytes) {
        return FsKind::Lvm;
    }
    if raid_magic(bytes) {
        return FsKind::Raid;
    }
    FsKind::Unknown
}

/// A GUID header and its entry array. A bad signature is [`None`].
#[must_use]
pub fn read_gpt(header: &[u8], entries: &[u8]) -> Option<Gpt> {
    if header.len() < 92 || header.get(..8) != Some(b"EFI PART".as_slice()) {
        return None;
    }
    let my = u64::from_le_bytes(header.get(24..32)?.try_into().ok()?);
    let count = u32::from_le_bytes(header.get(80..84)?.try_into().ok()?);
    let size = u32::from_le_bytes(header.get(84..88)?.try_into().ok()?);
    if my != 1 || size != 128 || count == 0 || count > 128 {
        return None;
    }
    tally(entries, usize::try_from(count).ok()?)
}

/// Entries that still have a type and a start, even when the header is damaged.
#[must_use]
pub fn guess_gpt(entries: &[u8]) -> usize {
    let count = entries.len() / 128;
    tally(entries, count).map_or(0, |gpt| gpt.partitions)
}

/// `cmd count` and a newline. The caller does not write the table back.
#[must_use]
pub fn format_count(cmd: &[u8], count: usize, out: &mut [u8]) -> Option<usize> {
    let mut buf = Buf::new(out);
    let Ok(count) = u64::try_from(count) else {
        return None;
    };
    if buf.push(cmd) && buf.push(b" ") && buf.u64(count) && buf.push(b"\n") {
        Some(buf.len())
    } else {
        None
    }
}

/// An ATA SMART data page. `smartctl N` when attribute 194 has a temperature.
#[must_use]
pub fn ata_smart_line(page: &[u8], out: &mut [u8]) -> Option<usize> {
    let page = page.get(..512)?;
    if page.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) != 0 {
        return None;
    }
    if u16::from_le_bytes([page[0], page[1]]) == 0 {
        return None;
    }
    let mut buf = Buf::new(out);
    if !buf.push(b"smartctl ") {
        return None;
    }
    let wrote = match ata_temp(page) {
        Some(temp) => buf.u64(u64::from(temp)),
        None => buf.push(b"ok"),
    };
    if wrote {
        Some(buf.len())
    } else {
        None
    }
}

/// Celsius from an `NVMe` health log. A non-zero warning byte is not a temperature.
#[must_use]
pub fn nvme_celsius(page: &[u8]) -> Option<u16> {
    if page.first().copied()? != 0 {
        return None;
    }
    // Bytes 1..3 are the composite temperature in kelvin.
    let kelvin = u16::from_le_bytes([*page.get(1)?, *page.get(2)?]);
    kelvin.checked_sub(273)
}

/// Identify-controller model, trailing spaces removed.
#[must_use]
pub fn nvme_model(id: &[u8]) -> Option<&[u8]> {
    let raw = id.get(24..64)?;
    let end = raw.iter().rposition(|byte| *byte != b' ' && *byte != 0)?;
    let model = raw.get(..=end)?;
    if model.is_empty()
        || model
            .iter()
            .any(|byte| !byte.is_ascii_graphic() && *byte != b' ')
    {
        return None;
    }
    Some(model)
}

/// The first IPv4 answer in a DNS response, or [`None`] when the packet is not one.
#[must_use]
pub fn dns_ipv4(packet: &[u8]) -> Option<[u8; 4]> {
    if packet.len() < 12 {
        return None;
    }
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    if flags & 0x8000 == 0 || flags & 0x000F != 0 {
        return None;
    }
    let questions = u16::from_be_bytes([packet[4], packet[5]]);
    let answers = u16::from_be_bytes([packet[6], packet[7]]);
    if questions == 0 || answers == 0 {
        return None;
    }
    let mut at = 12usize;
    for _ in 0..questions {
        at = skip_name(packet, at)?.checked_add(4)?;
    }
    at = skip_name(packet, at)?;
    ipv4_answer(packet, at)
}

/// `host port`. A dash, a slash, or a comma is a range and is rejected.
#[must_use]
pub fn endpoint(rest: &[u8]) -> Option<Endpoint> {
    let (host, port) = split_word(rest)?;
    if host.is_empty() || !split_word(port).is_none_or(|(_, extra)| extra.is_empty()) {
        return None;
    }
    if host.iter().copied().any(range_mark) || port.iter().copied().any(range_mark) {
        return None;
    }
    let (port, _) = split_word(port)?;
    Some(Endpoint {
        addr: ipv4(host)?,
        port: parse_port(port)?,
    })
}

/// Write `0xA5`, check it, write `0x5A`, and check it. An empty window fails.
#[must_use]
pub fn memtest(window: &mut [u8]) -> bool {
    if window.is_empty() {
        return false;
    }
    paint(window, 0xA5) && paint(window, 0x5A)
}

fn paint(window: &mut [u8], byte: u8) -> bool {
    for slot in window.iter_mut() {
        *slot = byte;
    }
    window.iter().all(|item| *item == byte)
}

fn tally(entries: &[u8], count: usize) -> Option<Gpt> {
    let mut partitions = 0usize;
    let mut lvm = false;
    let mut raid = false;
    for index in 0..count {
        let at = index.checked_mul(128)?;
        let entry = entries.get(at..at.checked_add(128)?)?;
        if entry[..16] == [0; 16] {
            continue;
        }
        let start = u64::from_le_bytes(entry[32..40].try_into().ok()?);
        if start == 0 {
            continue;
        }
        partitions += 1;
        lvm |= entry[..16] == LVM_GUID;
        raid |= entry[..16] == RAID_GUID;
    }
    Some(Gpt {
        partitions,
        lvm,
        raid,
    })
}

fn ata_temp(page: &[u8]) -> Option<u8> {
    for index in 0..30 {
        let at = 2 + index * 12;
        let id = *page.get(at)?;
        if id == 0 {
            return None;
        }
        if id == 194 {
            return page.get(at + 5).copied();
        }
    }
    None
}

fn ipv4_answer(packet: &[u8], at: usize) -> Option<[u8; 4]> {
    let kind = u16::from_be_bytes([*packet.get(at)?, *packet.get(at + 1)?]);
    let class = u16::from_be_bytes([*packet.get(at + 2)?, *packet.get(at + 3)?]);
    let len = u16::from_be_bytes([*packet.get(at + 8)?, *packet.get(at + 9)?]);
    if kind != 1 || class != 1 || len != 4 {
        return None;
    }
    let data = at.checked_add(10)?;
    Some([
        *packet.get(data)?,
        *packet.get(data + 1)?,
        *packet.get(data + 2)?,
        *packet.get(data + 3)?,
    ])
}

fn skip_name(packet: &[u8], mut at: usize) -> Option<usize> {
    for _ in 0..16 {
        let len = usize::from(*packet.get(at)?);
        if len == 0 {
            return at.checked_add(1);
        }
        if len & 0xC0 == 0xC0 {
            return at.checked_add(2);
        }
        if len & 0xC0 != 0 {
            return None;
        }
        at = at.checked_add(1 + len)?;
    }
    None
}

fn ipv4(text: &[u8]) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    let mut rest = text;
    for slot in &mut out {
        let (word, tail) = split_dot(rest)?;
        *slot = parse_octet(word)?;
        rest = tail;
    }
    if rest.is_empty() {
        Some(out)
    } else {
        None
    }
}

fn split_dot(text: &[u8]) -> Option<(&[u8], &[u8])> {
    if text.is_empty() {
        return None;
    }
    let Some(dot) = text.iter().position(|byte| *byte == b'.') else {
        return Some((text, b""));
    };
    Some((&text[..dot], &text[dot + 1..]))
}

fn parse_octet(text: &[u8]) -> Option<u8> {
    if text.is_empty() || text.len() > 3 {
        return None;
    }
    let mut value = 0u16;
    for byte in text {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u16::from(byte - b'0'))?;
    }
    u8::try_from(value).ok()
}

fn parse_port(text: &[u8]) -> Option<u16> {
    if text.is_empty() || text.len() > 5 {
        return None;
    }
    let mut value = 0u32;
    for byte in text {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))?;
    }
    let port = u16::try_from(value).ok()?;
    if port == 0 {
        None
    } else {
        Some(port)
    }
}

fn split_word(text: &[u8]) -> Option<(&[u8], &[u8])> {
    let text = trim(text);
    if text.is_empty() {
        return None;
    }
    let end = text
        .iter()
        .position(|byte| *byte == b' ' || *byte == b'\t')
        .unwrap_or(text.len());
    Some((&text[..end], trim(&text[end..])))
}

fn trim(text: &[u8]) -> &[u8] {
    let start = text
        .iter()
        .position(|byte| *byte != b' ' && *byte != b'\t')
        .unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|byte| *byte != b' ' && *byte != b'\t')
        .map_or(start, |index| index + 1);
    &text[start..end]
}

const fn range_mark(byte: u8) -> bool {
    byte == b'-' || byte == b'/' || byte == b','
}

fn has_prefix(bytes: &[u8], magic: &[u8]) -> bool {
    bytes.get(..magic.len()) == Some(magic)
}

fn luks(bytes: &[u8]) -> bool {
    bytes.len() >= 6
        && bytes.get(..4) == Some(b"LUKS".as_slice())
        && bytes[4] == 0xBA
        && bytes[5] == 0xBE
}

fn ext4(bytes: &[u8]) -> bool {
    let Some(magic) = bytes.get(0x438..0x43A) else {
        return false;
    };
    u16::from_le_bytes([magic[0], magic[1]]) == 0xEF53
}

fn btrfs(bytes: &[u8]) -> bool {
    bytes.get(0x40..0x48) == Some(b"_BHRfS_M".as_slice())
        || bytes.get(0x1_0040..0x1_0048) == Some(b"_BHRfS_M".as_slice())
}

fn lvm_label(bytes: &[u8]) -> bool {
    bytes.get(512..520) == Some(b"LABELONE".as_slice())
}

fn raid_magic(bytes: &[u8]) -> bool {
    let Some(magic) = bytes.get(4096..4100) else {
        return false;
    };
    u32::from_le_bytes([magic[0], magic[1], magic[2], magic[3]]) == 0xA92B_4EFC
}
