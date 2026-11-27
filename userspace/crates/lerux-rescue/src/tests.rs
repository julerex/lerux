use crate::{
    allow_copy, ata_smart_line, dns_ipv4, endpoint, fat_boot_ok, format_count, guess_gpt, identify,
    memtest, ntfs_boot, nvme_celsius, nvme_model, qemu_fat_boot, read_gpt, step, CopyPolicy, Flags,
    FsKind, Step, View,
};

fn sample(block: Option<&'static [u8]>, flags: Flags) -> View<'static> {
    View {
        arch: b"x86_64",
        cores: 1,
        block,
        roles: &[],
        pci: &[],
        stack: 0x4000,
        faults: 0,
        boot_ns: None,
        nvme_model: None,
        smart: None,
        mac: None,
        flags,
    }
}

fn shown(cmd: &[u8], rest: &[u8], view: &View<'_>) -> Vec<u8> {
    let mut out = [0u8; 128];
    match step(cmd, rest, view, &mut out) {
        Step::Show(n) => out[..n].to_vec(),
        Step::Absent => {
            let mut line = cmd.to_vec();
            line.extend_from_slice(b" err\n");
            line
        }
        Step::Unknown => b"unknown\n".to_vec(),
        _ => b"other\n".to_vec(),
    }
}

#[test]
fn fs_script_lines() {
    let view = sample(Some(b"virtio-blk"), Flags::new().mounted(true));
    assert_eq!(
        shown(b"lsblk", b"", &view).as_slice(),
        b"lsblk virtio-blk\n"
    );
    assert_eq!(shown(b"fdisk", b"", &view).as_slice(), b"fdisk none\n");
    assert_eq!(shown(b"fsck", b"", &view).as_slice(), b"fsck ok\n");
    assert_eq!(shown(b"ss", b"", &view).as_slice(), b"ss err\n");
    assert_eq!(shown(b"dig", b"", &view).as_slice(), b"dig err\n");
}

#[test]
fn native_script_lines() {
    let view = sample(Some(b"ahci"), Flags::new().mounted(true).net(true));
    assert_eq!(shown(b"lsblk", b"", &view).as_slice(), b"lsblk ahci\n");
    assert_eq!(shown(b"fdisk", b"", &view).as_slice(), b"fdisk none\n");
    assert_eq!(shown(b"fsck", b"", &view).as_slice(), b"fsck ok\n");
    assert_eq!(shown(b"ss", b"", &view).as_slice(), b"ss none\n");
    assert_eq!(shown(b"dig", b"", &view).as_slice(), b"dig err\n");
}

#[test]
fn fat_boot_matches_the_server_geometry() {
    let sector = qemu_fat_boot();
    assert!(fat_boot_ok(&sector));
    assert_eq!(identify(&sector), FsKind::Fat);
    let mut broken = sector;
    broken[510] = 0;
    assert!(!fat_boot_ok(&broken));
}

#[test]
fn ntfs_boot_names_the_oem() {
    let mut sector = [0u8; 512];
    sector[3..11].copy_from_slice(b"NTFS    ");
    sector[11..13].copy_from_slice(&512u16.to_le_bytes());
    sector[510] = 0x55;
    sector[511] = 0xAA;
    assert!(ntfs_boot(&sector));
    assert_eq!(identify(&sector), FsKind::Ntfs);
}

#[test]
fn guid_header_counts_a_logical_volume() {
    let (header, entry) = gpt_pair();
    let gpt = read_gpt(&header, &entry).expect("header");
    assert!(gpt.lvm);
    assert_eq!(gpt.partitions, 1);
    let mut out = [0u8; 16];
    let n = format_count(b"gdisk", gpt.partitions, &mut out).expect("line");
    assert_eq!(&out[..n], b"gdisk 1\n");
}

#[test]
fn damaged_guid_header_is_a_guess() {
    let (mut header, entry) = gpt_pair();
    header[..8].fill(0);
    assert!(read_gpt(&header, &entry).is_none());
    assert_eq!(guess_gpt(&entry), 1);
}

#[test]
fn smart_page_reports_temperature() {
    let page = smart_page(40);
    let mut out = [0u8; 16];
    let n = ata_smart_line(&page, &mut out).expect("page");
    assert_eq!(&out[..n], b"smartctl 40");
    let mut bad = page;
    bad[511] ^= 1;
    assert!(ata_smart_line(&bad, &mut out).is_none());
}

#[test]
fn dns_answer_is_the_first_address() {
    assert_eq!(dns_ipv4(&answer_a()), Some([10, 0, 2, 2]));
    let mut denied = answer_a();
    denied[3] |= 2;
    assert!(dns_ipv4(&denied).is_none());
}

#[test]
fn nmap_is_one_host_and_one_port() {
    let view = sample(None, Flags::new().net(true));
    let mut out = [0u8; 8];
    assert_eq!(
        step(b"nmap", b"10.0.2.2 80", &view, &mut out),
        Step::Probe([10, 0, 2, 2], 80)
    );
    assert_eq!(
        step(b"nmap", b"10.0.2.0/24 80", &view, &mut out),
        Step::Absent
    );
    assert_eq!(endpoint(b"10.0.2.2 1-80"), None);
}

#[test]
fn memtest_pattern_rejects_an_empty_window() {
    let mut window = [0u8; 4];
    assert!(memtest(&mut window));
    assert!(!memtest(&mut []));
}

#[test]
fn copy_needs_a_second_disk_and_the_dedicated_name() {
    let yes = CopyPolicy {
        second_disk: true,
        dest_dedicated: true,
        source_read_only: true,
    };
    assert!(allow_copy(yes));
    assert!(!allow_copy(CopyPolicy {
        second_disk: false,
        ..yes
    }));
}

#[test]
fn nvme_model_and_health_temperature() {
    let mut id = [0u8; 64];
    id[24..29].copy_from_slice(b"Hynix");
    assert_eq!(nvme_model(&id), Some(b"Hynix".as_slice()));
    assert_eq!(nvme_celsius(&[0, 0x2C, 0x01]), Some(27));
    assert_eq!(nvme_celsius(&[1, 0x2C, 0x01]), None);
}

#[test]
fn luks_magic_stays_locked() {
    let mut bytes = [0u8; 8];
    bytes[..6].copy_from_slice(b"LUKS\xba\xbe");
    assert_eq!(identify(&bytes), FsKind::Luks);
    let view = sample(Some(b"virtio-blk"), Flags::new().mounted(true));
    assert_eq!(
        shown(b"cryptsetup", b"", &view).as_slice(),
        b"cryptsetup err\n"
    );
}

fn gpt_pair() -> ([u8; 512], [u8; 128]) {
    let mut header = [0u8; 512];
    header[..8].copy_from_slice(b"EFI PART");
    header[24..32].copy_from_slice(&1u64.to_le_bytes());
    header[80..84].copy_from_slice(&1u32.to_le_bytes());
    header[84..88].copy_from_slice(&128u32.to_le_bytes());
    let mut entry = [0u8; 128];
    entry[..16].copy_from_slice(&[
        0x79, 0xD3, 0xD6, 0xE6, 0x07, 0xF5, 0xC2, 0x44, 0xA2, 0x3C, 0x23, 0x8F, 0x2A, 0x3D, 0xF9,
        0x28,
    ]);
    entry[32..40].copy_from_slice(&2048u64.to_le_bytes());
    (header, entry)
}

fn smart_page(temp: u8) -> [u8; 512] {
    let mut page = [0u8; 512];
    page[0] = 1;
    page[2] = 194;
    page[7] = temp;
    let sum = page.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    page[511] = 0u8.wrapping_sub(sum);
    page
}

fn answer_a() -> Vec<u8> {
    let mut packet = vec![0, 0, 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0];
    packet.extend_from_slice(b"\x07example\x03com\x00");
    packet.extend_from_slice(&[0, 1, 0, 1]);
    packet.extend_from_slice(&[0xC0, 0x0C, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4, 10, 0, 2, 2]);
    packet
}
