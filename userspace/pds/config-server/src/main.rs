#![no_std]
#![no_main]

use lerux_interface_types::{
    ConfigRequest, ConfigResponse, FsRequest, FsResponse, MAX_CONFIG_KEY_LEN, MAX_CONFIG_VAL_LEN,
    MAX_FS_PATH,
};
use lerux_ipc::{call, recv, send, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const FS_SERVER: Channel = Channel::new(3);
const SUPERVISOR: Channel = Channel::new(0);
const SHELL: Channel = Channel::new(1);
const NET_SERVER: Channel = Channel::new(2);

fn fs_call(req: FsRequest) -> FsResponse {
    match call::<FsRequest, FsResponse>(FS_SERVER, req) {
        Ok(FsResponse::Pending) => poll_fs(),
        Ok(other) => other,
        Err(_) => FsResponse::Error,
    }
}

fn poll_fs() -> FsResponse {
    loop {
        match call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::Poll) {
            Ok(FsResponse::Pending) => {}
            Ok(other) => return other,
            Err(_) => return FsResponse::Error,
        }
    }
}

fn read_config_file(key: &[u8]) -> Option<(u8, [u8; MAX_CONFIG_VAL_LEN])> {
    let mut path = [0u8; MAX_FS_PATH];
    let prefix = b"/config/";
    let mut pos = 0;
    if pos + prefix.len() > MAX_FS_PATH {
        return None;
    }
    path[pos..pos + prefix.len()].copy_from_slice(prefix);
    pos += prefix.len();
    let klen = key.len().min(MAX_CONFIG_KEY_LEN);
    if pos + klen > MAX_FS_PATH {
        return None;
    }
    path[pos..pos + klen].copy_from_slice(&key[..klen]);
    pos += klen;

    let handle = match fs_call(FsRequest::open(&path[..pos])) {
        FsResponse::Handle { id } => id,
        _ => return None,
    };

    let mut buf = [0u8; MAX_CONFIG_VAL_LEN];
    let mut total: u8 = 0;
    let mut offset = 0u32;
    loop {
        match fs_call(FsRequest::Read {
            handle,
            offset,
            len: 64,
        }) {
            FsResponse::Data { data_len, data } if data_len > 0 => {
                let n = data_len as usize;
                if (total as usize + n) > MAX_CONFIG_VAL_LEN {
                    break;
                }
                buf[total as usize..total as usize + n].copy_from_slice(&data[..n]);
                total += data_len as u8;
                offset += data_len as u32;
            }
            _ => break,
        }
    }
    Some((total, buf))
}

fn write_config_file(key: &[u8], value: &[u8]) -> bool {
    let mut path = [0u8; MAX_FS_PATH];
    let prefix = b"/config/";
    let mut pos = 0;
    if pos + prefix.len() > MAX_FS_PATH {
        return false;
    }
    path[pos..pos + prefix.len()].copy_from_slice(prefix);
    pos += prefix.len();
    let klen = key.len().min(MAX_CONFIG_KEY_LEN);
    if pos + klen > MAX_FS_PATH {
        return false;
    }
    path[pos..pos + klen].copy_from_slice(&key[..klen]);
    pos += klen;

    let handle = match fs_call(FsRequest::create(&path[..pos])) {
        FsResponse::Handle { id } => id,
        _ => return false,
    };
    matches!(fs_call(FsRequest::write(handle, 0, value)), FsResponse::Ok)
}

fn list_config_keys() -> (u8, [[u8; MAX_CONFIG_KEY_LEN]; 8], [u8; 8]) {
    let mut keys = [[0u8; MAX_CONFIG_KEY_LEN]; 8];
    let mut lens = [0u8; 8];
    let mut count: u8 = 0;

    // Phase 50: `/config` is a real directory; list component names inside it.
    if let FsResponse::DirList { count: dc, entries } = fs_call(FsRequest::list_dir(b"/config")) {
        for e in entries.iter().take(dc as usize) {
            if e.is_dir {
                continue;
            }
            let name = e.name_slice();
            if count < 8 {
                let kl = name.len().min(MAX_CONFIG_KEY_LEN) as u8;
                keys[count as usize][..kl as usize].copy_from_slice(&name[..kl as usize]);
                lens[count as usize] = kl;
                count += 1;
            }
        }
    }
    (count, keys, lens)
}

fn handle_config(req: ConfigRequest) -> ConfigResponse {
    match req {
        ConfigRequest::Get { key_len, key } => {
            let k = &key[..key_len as usize];
            if let Some((vlen, v)) = read_config_file(k) {
                ConfigResponse::Value {
                    val_len: vlen,
                    value: v,
                }
            } else {
                ConfigResponse::Error
            }
        }
        ConfigRequest::Set {
            key_len,
            key,
            val_len,
            value,
        } => {
            let k = &key[..key_len as usize];
            let v = &value[..val_len as usize];
            if write_config_file(k, v) {
                ConfigResponse::Ok
            } else {
                ConfigResponse::Error
            }
        }
        ConfigRequest::List => {
            let (count, keys, lens) = list_config_keys();
            ConfigResponse::Keys { count, keys, lens }
        }
    }
}

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("config-server: ready (using FS for /config/* )");
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel == SUPERVISOR || channel == SHELL || channel == NET_SERVER {
            return Ok(match recv::<ConfigRequest>(msg_info) {
                Ok(req) => send(handle_config(req)),
                Err(_) => send_unspecified_error(),
            });
        }
        Ok(send_unspecified_error())
    }
}
