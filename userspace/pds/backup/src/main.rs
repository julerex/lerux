//! Backup PD: snapshot FS root listing into `/backup/manifest` (Phase 58).

#![no_std]
#![no_main]

use lerux_interface_types::{BackupRequest, BackupResponse, FsRequest, FsResponse};
use lerux_ipc::{recv, send, send_unspecified_error, FsClient};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const SHELL: Channel = Channel::new(0);
const FS_SERVER: FsClient = FsClient::new(Channel::new(1));

fn fs_call(req: FsRequest) -> FsResponse {
    FS_SERVER.call(req)
}

struct HandlerImpl {
    last_files: u8,
    last_bytes: u32,
}

impl HandlerImpl {
    fn snapshot(&mut self) -> BackupResponse {
        let _ = fs_call(FsRequest::mkdir(b"/backup"));
        let listing = match fs_call(FsRequest::list_root()) {
            FsResponse::DirList { count, entries } => (count, entries),
            _ => return BackupResponse::Error,
        };
        let mut buf = [0u8; 400];
        let mut pos = 0usize;
        let hdr = b"# lerux-backup manifest\n";
        buf[..hdr.len()].copy_from_slice(hdr);
        pos += hdr.len();
        let mut files = 0u8;
        for e in listing.1.iter().take(listing.0 as usize) {
            let name = e.name_slice();
            if pos + name.len() + 1 >= buf.len() {
                break;
            }
            buf[pos..pos + name.len()].copy_from_slice(name);
            pos += name.len();
            buf[pos] = b'\n';
            pos += 1;
            files = files.saturating_add(1);
        }
        let Ok(handle) = FS_SERVER.create_clean(b"/backup/manifest") else {
            return BackupResponse::Error;
        };
        match fs_call(FsRequest::write(handle, 0, &buf[..pos])) {
            FsResponse::Ok => {
                self.last_files = files;
                self.last_bytes = pos as u32;
                log::info!("lerux-backup: snapshot files={} bytes={}", files, pos);
                BackupResponse::Report {
                    files,
                    bytes: pos as u32,
                }
            }
            _ => BackupResponse::Error,
        }
    }

    fn handle(&mut self, req: BackupRequest) -> BackupResponse {
        match req {
            BackupRequest::Snapshot => self.snapshot(),
            BackupRequest::Status => BackupResponse::Report {
                files: self.last_files,
                bytes: self.last_bytes,
            },
        }
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug log init");
    log::info!("lerux-backup: ready");
    HandlerImpl {
        last_files: 0,
        last_bytes: 0,
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != SHELL {
            return Ok(send_unspecified_error());
        }
        Ok(match recv::<BackupRequest>(msg_info) {
            Ok(req) => send(self.handle(req)),
            Err(_) => send_unspecified_error(),
        })
    }
}
