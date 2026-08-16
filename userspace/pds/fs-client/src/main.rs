#![no_std]
#![no_main]

use lerux_interface_types::{FsRequest, FsResponse, MAX_FS_DATA, SECTOR_SIZE};
use lerux_ipc::FsClient;
use lerux_logging::{log, serial};
#[cfg(feature = "isolation-sync")]
use sel4_microkit::ChannelSet;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const FS_SERVER: FsClient = FsClient::new(Channel::new(1));
/// Phase 60 isolation: notify from `debug-handler` after untrusted crash.
#[cfg(feature = "isolation-sync")]
const ISOLATION_GATE: Channel = Channel::new(2);

const TEST_PATH: &[u8] = b"ping";
const TEST_DATA: &[u8] = b"lerux-fs smoke";

struct HandlerImpl {
    #[cfg(feature = "isolation-sync")]
    probe_pending: bool,
}

fn fs_call(req: FsRequest) -> FsResponse {
    FS_SERVER.call(req)
}

fn fs_create(path: &[u8]) -> u8 {
    // Re-running smoke on a persistent disk.img often leaves files around,
    // so fall back to opening an existing file.
    FS_SERVER
        .create_or_open(path)
        .expect("create failed and open fallback failed")
}

fn fs_write(handle: u8, offset: u32, data: &[u8]) {
    match fs_call(FsRequest::write(handle, offset, data)) {
        FsResponse::Ok => {}
        FsResponse::Pending
        | FsResponse::Error
        | FsResponse::Handle { .. }
        | FsResponse::Data { .. }
        | FsResponse::Stat { .. }
        | FsResponse::DirList { .. }
        | FsResponse::DiskInfo { .. } => {
            panic!("write failed")
        }
    }
}

fn fs_read(handle: u8, offset: u32, len: u16) -> FsResponse {
    fs_call(FsRequest::Read {
        handle,
        offset,
        len,
    })
}

/// Chunk writes so each request stays inside one 512-byte sector (FAT multi-cluster
/// and LERUXFS2 both accept this; FAT rejects cross-cluster single Writes).
fn write_all(handle: u8, data: &[u8]) {
    write_all_at(handle, 0, data);
}

fn write_all_at(handle: u8, start: u32, data: &[u8]) {
    let mut offset = start;
    let end_abs = start as usize + data.len();
    while (offset as usize) < end_abs {
        let sector_left = SECTOR_SIZE - (offset as usize % SECTOR_SIZE);
        let rel = offset as usize - start as usize;
        let end_rel = (rel + MAX_FS_DATA.min(sector_left)).min(data.len());
        fs_write(handle, offset, &data[rel..end_rel]);
        offset = start + end_rel as u32;
    }
}

fn read_all(handle: u8, len: usize, out: &mut [u8]) {
    let mut offset = 0u32;
    while (offset as usize) < len {
        let sector_left = SECTOR_SIZE - (offset as usize % SECTOR_SIZE);
        let chunk = (len - offset as usize).min(MAX_FS_DATA).min(sector_left) as u16;
        let FsResponse::Data { data_len, data } = fs_read(handle, offset, chunk) else {
            panic!("read failed at offset {offset}");
        };
        assert!(data_len > 0, "short read at offset {offset}");
        let n = data_len as usize;
        out[offset as usize..offset as usize + n].copy_from_slice(&data[..n]);
        offset += data_len as u32;
    }
}

fn write_pattern(handle: u8, len: usize) {
    let mut offset = 0u32;
    while (offset as usize) < len {
        let mut chunk = [0u8; 256];
        let n = (len - offset as usize).min(256);
        for (i, b) in chunk[..n].iter_mut().enumerate() {
            *b = ((offset as usize + i) % 251) as u8;
        }
        write_all_at(handle, offset, &chunk[..n]);
        offset += n as u32;
    }
}

fn verify_pattern(handle: u8, len: usize) {
    let mut offset = 0u32;
    while (offset as usize) < len {
        let sector_left = SECTOR_SIZE - (offset as usize % SECTOR_SIZE);
        let n = (len - offset as usize).min(MAX_FS_DATA).min(sector_left) as u16;
        let FsResponse::Data { data_len, data } = fs_read(handle, offset, n) else {
            panic!("huge read failed at {offset}");
        };
        assert_eq!(data_len, n, "short huge read at {offset}");
        for i in 0..n as usize {
            let expect = ((offset as usize + i) % 251) as u8;
            assert_eq!(data[i], expect, "huge mismatch at {}", offset as usize + i);
        }
        offset += u32::from(n);
    }
}

fn probe_fs() {
    // Basic create / write / read / stat (root file).
    let handle = fs_create(TEST_PATH);
    fs_write(handle, 0, TEST_DATA);

    let FsResponse::Data { data_len, data } = fs_read(handle, 0, TEST_DATA.len() as u16) else {
        panic!("read failed")
    };
    let len = data_len as usize;
    assert_eq!(&data[..len], TEST_DATA, "read round-trip mismatch");

    match fs_call(FsRequest::stat(TEST_PATH)) {
        FsResponse::Stat { size, is_dir } => {
            assert_eq!(size, TEST_DATA.len() as u32);
            assert!(!is_dir);
        }
        _ => panic!("stat failed"),
    }

    match fs_call(FsRequest::list_root()) {
        FsResponse::DirList { count, entries } => {
            assert!(count >= 1);
            let mut found = false;
            for e in entries.iter().take(count as usize) {
                if e.name_slice() == TEST_PATH {
                    found = true;
                    break;
                }
            }
            assert!(found, "ping not listed in root");
        }
        _ => panic!("listdir failed"),
    }

    // Phase 50 hierarchy + multi-sector (LERUXFS2).
    #[cfg(not(feature = "board-qemu_virt_aarch64_fs_fat"))]
    probe_fs_v2();

    // Phase 50 FAT stretch: subdirs, LFN, multi-cluster.
    #[cfg(feature = "board-qemu_virt_aarch64_fs_fat")]
    probe_fs_fat();

    log::info!("lerux-fs: round-trip ok");
    #[cfg(feature = "isolation-sync")]
    log::info!("lerux-isolation: fs-server survived untrusted PD crash");
}

#[cfg(feature = "board-qemu_virt_aarch64_fs_fat")]
fn list_contains(path: &[u8], name: &[u8], expect_dir: bool) -> bool {
    match fs_call(FsRequest::list_dir(path)) {
        FsResponse::DirList { count, entries } => entries
            .iter()
            .take(count as usize)
            .any(|e| e.name_slice() == name && e.is_dir == expect_dir),
        _ => panic!("listdir failed"),
    }
}

#[cfg(feature = "board-qemu_virt_aarch64_fs_fat")]
fn mkdir_idempotent(path: &[u8]) {
    match fs_call(FsRequest::mkdir(path)) {
        FsResponse::Ok => {}
        FsResponse::Error => match fs_call(FsRequest::stat(path)) {
            FsResponse::Stat { is_dir: true, .. } => {}
            _ => panic!("mkdir failed"),
        },
        _ => panic!("mkdir failed"),
    }
}

#[cfg(feature = "board-qemu_virt_aarch64_fs_fat")]
fn probe_fs_fat() {
    const DIR_PATH: &[u8] = b"/testdir";
    const NESTED_PATH: &[u8] = b"/testdir/nested";
    const LFN_PATH: &[u8] = b"/testdir/longfilename";
    const LFN_NAME: &[u8] = b"longfilename";
    const BIG_PATH: &[u8] = b"big.dat";

    mkdir_idempotent(DIR_PATH);
    match fs_call(FsRequest::stat(DIR_PATH)) {
        FsResponse::Stat { is_dir: true, .. } => {}
        _ => panic!("testdir stat failed"),
    }

    let nested = fs_create(NESTED_PATH);
    fs_write(nested, 0, b"nested-ok");
    match fs_call(FsRequest::stat(NESTED_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, 9),
        _ => panic!("nested stat failed"),
    }
    assert!(
        list_contains(DIR_PATH, b"nested", false),
        "nested not listed"
    );

    let lfn = fs_create(LFN_PATH);
    fs_write(lfn, 0, b"lfn-ok");
    match fs_call(FsRequest::stat(LFN_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, 6),
        _ => panic!("lfn stat failed"),
    }
    assert!(
        list_contains(DIR_PATH, LFN_NAME, false),
        "longfilename not listed"
    );
    log::info!("lerux-fs: fat lfn ok");

    let _ = fs_call(FsRequest::unlink(b"/testdir/renamed"));
    match fs_call(FsRequest::rename(NESTED_PATH, b"/testdir/renamed")) {
        FsResponse::Ok => {}
        _ => panic!("rename failed"),
    }
    match fs_call(FsRequest::unlink(b"/testdir/renamed")) {
        FsResponse::Ok => {}
        _ => panic!("unlink failed"),
    }
    log::info!("lerux-fs: fat hierarchy ok");

    let mut big = [0u8; 600];
    for (i, b) in big.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    let big_h = match FS_SERVER.create_or_open(BIG_PATH) {
        Ok(id) => id,
        Err(_) => panic!("big create/open failed"),
    };
    write_all(big_h, &big);
    match fs_call(FsRequest::stat(BIG_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, 600, "fat multi-cluster size"),
        _ => panic!("big stat failed"),
    }
    let mut got = [0u8; 600];
    read_all(big_h, 600, &mut got);
    assert_eq!(&got, &big, "fat multi-cluster round-trip mismatch");
    log::info!("lerux-fs: fat multi-cluster ok");
}

#[cfg(not(feature = "board-qemu_virt_aarch64_fs_fat"))]
fn probe_fs_v2() {
    const DIR_PATH: &[u8] = b"/testdir";
    const NESTED_PATH: &[u8] = b"/testdir/nested";
    /// Multi-sector payload: > 512 bytes so Phase 50 extent growth is exercised.
    const BIG_PATH: &[u8] = b"/testdir/big";

    // Hierarchy: mkdir + nested create.
    match fs_call(FsRequest::mkdir(DIR_PATH)) {
        FsResponse::Ok => {}
        // Idempotent when re-running on persistent disk.
        FsResponse::Error => match fs_call(FsRequest::stat(DIR_PATH)) {
            FsResponse::Stat { is_dir: true, .. } => {}
            _ => panic!("mkdir failed"),
        },
        _ => panic!("mkdir failed"),
    }

    let nested = fs_create(NESTED_PATH);
    fs_write(nested, 0, b"nested-ok");
    match fs_call(FsRequest::stat(NESTED_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, 9),
        _ => panic!("nested stat failed"),
    }
    match fs_call(FsRequest::list_dir(DIR_PATH)) {
        FsResponse::DirList { count, entries } => {
            assert!(count >= 1);
            assert_eq!(entries[0].name_slice(), b"nested");
        }
        _ => panic!("list testdir failed"),
    }

    // Multi-sector file via chunked Write/Read.
    let mut big = [0u8; 600];
    for (i, b) in big.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    let _ = fs_call(FsRequest::unlink(BIG_PATH));
    let big_h = fs_create(BIG_PATH);
    write_all(big_h, &big);
    match fs_call(FsRequest::stat(BIG_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, 600),
        _ => panic!("big stat failed"),
    }
    let mut got = [0u8; 600];
    read_all(big_h, 600, &mut got);
    assert_eq!(&got, &big, "multi-sector round-trip mismatch");

    // Phase 62: file larger than the old 16 KiB cap, verified in chunks.
    const HUGE_PATH: &[u8] = b"/testdir/huge";
    const HUGE_LEN: usize = 20 * 1024;
    let _ = fs_call(FsRequest::unlink(HUGE_PATH));
    let huge_h = fs_create(HUGE_PATH);
    write_pattern(huge_h, HUGE_LEN);
    match fs_call(FsRequest::stat(HUGE_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, HUGE_LEN as u32, "v3 size"),
        _ => panic!("huge stat failed"),
    }
    verify_pattern(huge_h, HUGE_LEN);
    log::info!("lerux-fs: v3 20k ok");

    // Phase 62: path longer than the old 48-byte cap.
    const LONG_PATH: &[u8] = b"/testdir/a/b/c/deep-component-name";
    let long_h = fs_create(LONG_PATH);
    fs_write(long_h, 0, b"deep-ok");
    match fs_call(FsRequest::stat(LONG_PATH)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => assert_eq!(size, 7),
        _ => panic!("long path stat failed"),
    }
    log::info!("lerux-fs: v3 long path ok");

    // Rename + unlink.
    match fs_call(FsRequest::rename(b"/testdir/nested", b"/testdir/renamed")) {
        FsResponse::Ok => {}
        _ => panic!("rename failed"),
    }
    match fs_call(FsRequest::unlink(b"/testdir/renamed")) {
        FsResponse::Ok => {}
        _ => panic!("unlink failed"),
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    #[cfg(feature = "isolation-sync")]
    {
        log::info!("lerux-isolation: waiting for untrusted PD crash");
        HandlerImpl {
            probe_pending: true,
        }
    }
    #[cfg(not(feature = "isolation-sync"))]
    {
        probe_fs();
        HandlerImpl {}
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    #[cfg(feature = "isolation-sync")]
    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if self.probe_pending && channels.contains(ISOLATION_GATE) {
            self.probe_pending = false;
            log::info!("lerux-isolation: probe after crash");
            probe_fs();
        }
        Ok(())
    }
}
