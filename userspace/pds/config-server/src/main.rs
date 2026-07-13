//! Phase 36/54: FS-backed config under `/config/` (+ `/config/secrets/` for `secret.*`).

#![no_std]
#![no_main]

use lerux_interface_types::{
    ConfigRequest, ConfigResponse, FsRequest, FsResponse, CFG_SECRET_PREFIX, MAX_CONFIG_KEY_LEN,
    MAX_CONFIG_VAL_LEN, MAX_FS_PATH,
};
use lerux_ipc::{recv, send, send_unspecified_error, FsClient};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const FS_SERVER: FsClient = FsClient::new(Channel::new(3));
const SUPERVISOR: Channel = Channel::new(0);
const SHELL: Channel = Channel::new(1);
const NET_SERVER: Channel = Channel::new(2);

fn fs_call(req: FsRequest) -> FsResponse {
    FS_SERVER.call(req)
}

fn is_secret_key(key: &[u8]) -> bool {
    key.starts_with(CFG_SECRET_PREFIX)
}

/// Map logical key → absolute FS path under `/config/` or `/config/secrets/`.
fn key_to_path(key: &[u8], path: &mut [u8; MAX_FS_PATH]) -> Option<usize> {
    if key.is_empty() || key.iter().any(|&b| b == 0 || b == b'/') {
        return None;
    }
    let (prefix, name): (&[u8], &[u8]) = if is_secret_key(key) {
        (b"/config/secrets/", &key[CFG_SECRET_PREFIX.len()..])
    } else {
        (b"/config/", key)
    };
    if name.is_empty() {
        return None;
    }
    let mut pos = 0;
    if pos + prefix.len() > MAX_FS_PATH {
        return None;
    }
    path[pos..pos + prefix.len()].copy_from_slice(prefix);
    pos += prefix.len();
    let nlen = name.len().min(MAX_CONFIG_KEY_LEN);
    if pos + nlen > MAX_FS_PATH {
        return None;
    }
    path[pos..pos + nlen].copy_from_slice(&name[..nlen]);
    pos += nlen;
    Some(pos)
}

fn ensure_config_dirs() {
    let _ = fs_call(FsRequest::mkdir(b"/config"));
    let _ = fs_call(FsRequest::mkdir(b"/config/secrets"));
}

fn read_config_file(key: &[u8]) -> Option<(u8, [u8; MAX_CONFIG_VAL_LEN])> {
    let mut path = [0u8; MAX_FS_PATH];
    let pos = key_to_path(key, &mut path)?;
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
    ensure_config_dirs();
    let mut path = [0u8; MAX_FS_PATH];
    let Some(pos) = key_to_path(key, &mut path) else {
        return false;
    };
    let Ok(handle) = FS_SERVER.create_clean(&path[..pos]) else {
        return false;
    };
    matches!(fs_call(FsRequest::write(handle, 0, value)), FsResponse::Ok)
}

fn delete_config_file(key: &[u8]) -> bool {
    let mut path = [0u8; MAX_FS_PATH];
    let Some(pos) = key_to_path(key, &mut path) else {
        return false;
    };
    matches!(fs_call(FsRequest::unlink(&path[..pos])), FsResponse::Ok)
}

fn push_key(
    keys: &mut [[u8; MAX_CONFIG_KEY_LEN]; 8],
    lens: &mut [u8; 8],
    count: &mut u8,
    name: &[u8],
) {
    if (*count as usize) >= 8 || name.is_empty() {
        return;
    }
    let kl = name.len().min(MAX_CONFIG_KEY_LEN) as u8;
    keys[*count as usize][..kl as usize].copy_from_slice(&name[..kl as usize]);
    lens[*count as usize] = kl;
    *count += 1;
}

fn list_config_keys() -> (u8, [[u8; MAX_CONFIG_KEY_LEN]; 8], [u8; 8]) {
    let mut keys = [[0u8; MAX_CONFIG_KEY_LEN]; 8];
    let mut lens = [0u8; 8];
    let mut count: u8 = 0;

    if let FsResponse::DirList { count: dc, entries } = fs_call(FsRequest::list_dir(b"/config")) {
        for e in entries.iter().take(dc as usize) {
            if e.is_dir {
                // secrets dir is listed separately
                continue;
            }
            push_key(&mut keys, &mut lens, &mut count, e.name_slice());
        }
    }
    if let FsResponse::DirList { count: dc, entries } =
        fs_call(FsRequest::list_dir(b"/config/secrets"))
    {
        for e in entries.iter().take(dc as usize) {
            if e.is_dir {
                continue;
            }
            // Present as secret.<name>
            let mut full = [0u8; MAX_CONFIG_KEY_LEN];
            let pref = CFG_SECRET_PREFIX;
            if pref.len() + e.name_slice().len() > MAX_CONFIG_KEY_LEN {
                continue;
            }
            full[..pref.len()].copy_from_slice(pref);
            let n = e.name_slice().len();
            full[pref.len()..pref.len() + n].copy_from_slice(e.name_slice());
            push_key(&mut keys, &mut lens, &mut count, &full[..pref.len() + n]);
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
        ConfigRequest::Delete { key_len, key } => {
            let k = &key[..key_len as usize];
            if delete_config_file(k) {
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
    log::info!("config-server: ready (Phase 54 schema)");
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
