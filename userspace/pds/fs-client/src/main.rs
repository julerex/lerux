#![no_std]
#![no_main]

use lerux_interface_types::{FsRequest, FsResponse};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const FS_SERVER: Channel = Channel::new(1);

const TEST_PATH: &[u8] = b"ping";
const TEST_DATA: &[u8] = b"lerux-fs smoke";

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
        FsResponse::Pending
        | FsResponse::Ok
        | FsResponse::Error
        | FsResponse::Data { .. }
        | FsResponse::Stat { .. }
        | FsResponse::DirList { .. } => panic!("create failed"),
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
        | FsResponse::DirList { .. } => {
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

fn probe_fs() {
    let handle = fs_create(TEST_PATH);
    fs_write(handle, 0, TEST_DATA);

    let FsResponse::Data { data_len, data } = fs_read(handle, 0, TEST_DATA.len() as u16) else {
        panic!("read failed")
    };
    let len = data_len as usize;
    assert_eq!(&data[..len], TEST_DATA, "read round-trip mismatch");

    match fs_call(FsRequest::stat(TEST_PATH)) {
        FsResponse::Stat { size } => assert_eq!(size, TEST_DATA.len() as u32),
        _ => panic!("stat failed"),
    }

    match fs_call(FsRequest::ListDir) {
        FsResponse::DirList { count, entries } => {
            assert!(count >= 1);
            assert_eq!(entries[0].name_slice(), TEST_PATH);
        }
        _ => panic!("listdir failed"),
    }

    log::info!("lerux-fs: round-trip ok");
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
