#![no_std]
#![no_main]

use lerux_interface_types::{FsRequest, FsResponse, MAX_FS_DATA};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const FS_SERVER: Channel = Channel::new(1);

const TEST_PATH: &[u8] = b"ping";
const TEST_DATA: &[u8] = b"lerux-fs smoke";
const DIR_PATH: &[u8] = b"/testdir";
const NESTED_PATH: &[u8] = b"/testdir/nested";
/// Multi-sector payload: > 512 bytes so Phase 50 extent growth is exercised.
const BIG_PATH: &[u8] = b"/testdir/big";

struct HandlerImpl;

fn poll_fs() -> FsResponse {
    loop {
        match call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::Poll).expect("Poll IPC") {
            FsResponse::Pending => {}
            other => return other,
        }
    }
}

fn fs_call(req: FsRequest) -> FsResponse {
    match call::<FsRequest, FsResponse>(FS_SERVER, req).expect("FS IPC") {
        FsResponse::Pending => poll_fs(),
        other => other,
    }
}

fn fs_create(path: &[u8]) -> u8 {
    match fs_call(FsRequest::create(path)) {
        FsResponse::Handle { id } => id,
        // Re-running smoke on a persistent disk.img often leaves files around.
        FsResponse::Error => match fs_call(FsRequest::open(path)) {
            FsResponse::Handle { id } => id,
            _ => panic!("create failed and open fallback failed"),
        },
        FsResponse::Pending
        | FsResponse::Ok
        | FsResponse::Data { .. }
        | FsResponse::Stat { .. }
        | FsResponse::DirList { .. }
        | FsResponse::DiskInfo { .. } => panic!("create failed"),
    }
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

#[cfg(not(feature = "board-qemu_virt_aarch64_fs_fat"))]
fn write_all(handle: u8, data: &[u8]) {
    let mut offset = 0u32;
    while (offset as usize) < data.len() {
        let end = (offset as usize + MAX_FS_DATA).min(data.len());
        fs_write(handle, offset, &data[offset as usize..end]);
        offset = end as u32;
    }
}

#[cfg(not(feature = "board-qemu_virt_aarch64_fs_fat"))]
fn read_all(handle: u8, len: usize, out: &mut [u8]) {
    let mut offset = 0u32;
    while (offset as usize) < len {
        let chunk = (len - offset as usize).min(MAX_FS_DATA) as u16;
        let FsResponse::Data { data_len, data } = fs_read(handle, offset, chunk) else {
            panic!("read failed at offset {offset}");
        };
        assert!(data_len > 0, "short read at offset {offset}");
        let n = data_len as usize;
        out[offset as usize..offset as usize + n].copy_from_slice(&data[..n]);
        offset += data_len as u32;
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

    // Phase 50 hierarchy + multi-sector (LERUXFS2 only; FAT stays root/single-cluster).
    #[cfg(not(feature = "board-qemu_virt_aarch64_fs_fat"))]
    probe_fs_v2();

    log::info!("lerux-fs: round-trip ok");
}

#[cfg(not(feature = "board-qemu_virt_aarch64_fs_fat"))]
fn probe_fs_v2() {
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
    probe_fs();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
