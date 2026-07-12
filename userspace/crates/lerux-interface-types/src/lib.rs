//! Shared postcard RPC message types for lerux protection domains.

#![no_std]

use serde::{Deserialize, Serialize};

/// Maximum payload length for [`EchoRequest::Echo`] / [`EchoResponse::Echo`].
pub const MAX_ECHO_LEN: usize = 32;

/// Echo service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoRequest {
    Ping,
    Echo { len: u8, text: [u8; MAX_ECHO_LEN] },
}

impl EchoRequest {
    pub fn echo(text: &[u8]) -> Self {
        let mut buf = [0u8; MAX_ECHO_LEN];
        let len = text.len().min(MAX_ECHO_LEN) as u8;
        buf[..len as usize].copy_from_slice(&text[..len as usize]);
        Self::Echo { len, text: buf }
    }
}

/// Echo service responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoResponse {
    Pong,
    Echo { len: u8, text: [u8; MAX_ECHO_LEN] },
}

impl EchoResponse {
    pub fn as_echo_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Pong => None,
            Self::Echo { len, text } => Some(&text[..*len as usize]),
        }
    }
}

/// Sector size for [`BlockResponse::Sector`].
pub const SECTOR_SIZE: usize = 512;

/// Block service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "WriteSector carries one disk sector inline for IPC"
)]
pub enum BlockRequest {
    ReadSector {
        lba: u32,
    },
    WriteSector {
        lba: u32,
        #[serde(with = "sector_bytes")]
        data: [u8; SECTOR_SIZE],
    },
    Poll,
}

/// Block service responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "Sector payload must hold one disk sector inline for IPC"
)]
pub enum BlockResponse {
    Pending,
    Ok,
    Sector {
        #[serde(with = "sector_bytes")]
        data: [u8; SECTOR_SIZE],
    },
    Error,
}

/// Maximum UDP payload for [`NetRequest::UdpTx`].
pub const MAX_NET_UDP_PAYLOAD: usize = 128;

/// Maximum TCP payload for [`NetRequest::TcpSend`] / [`NetResponse::TcpData`].
pub const MAX_NET_TCP_PAYLOAD: usize = 512;

/// Maximum hostname length for [`NetRequest::DnsResolve`].
pub const MAX_DNS_NAME: usize = 32;

/// Network service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "TcpSend carries inline payload for IPC"
)]
pub enum NetRequest {
    UdpTx {
        payload_len: u8,
        #[serde(with = "net_payload_bytes")]
        payload: [u8; MAX_NET_UDP_PAYLOAD],
    },
    /// Receive one UDP datagram previously bound via [`NetRequest::UdpTx`] / listen port.
    UdpRecv,
    DnsResolve {
        name_len: u8,
        #[serde(with = "dns_name_bytes")]
        name: [u8; MAX_DNS_NAME],
    },
    TcpConnect {
        addr: [u8; 4],
        port: u16,
    },
    /// Listen for inbound TCP (Phase 40 HTTP file browser).
    /// Uses a dedicated listen socket so outbound TcpConnect can run concurrently (Phase 51).
    TcpListen {
        port: u16,
    },
    TcpSend {
        payload_len: u16,
        #[serde(with = "tcp_payload_bytes")]
        payload: [u8; MAX_NET_TCP_PAYLOAD],
    },
    TcpRecv,
    /// Close the active TCP socket (client or accepted listen connection).
    TcpClose,
    /// Abandon a pending recv without closing sockets (client timeout).
    Abort,
    /// Return current IPv4 configuration (static or DHCP). Non-blocking.
    GetIface,
    Poll,
}

impl NetRequest {
    pub fn udp_tx(text: &[u8]) -> Self {
        let mut payload = [0u8; MAX_NET_UDP_PAYLOAD];
        let payload_len = text.len().min(MAX_NET_UDP_PAYLOAD) as u8;
        payload[..payload_len as usize].copy_from_slice(&text[..payload_len as usize]);
        Self::UdpTx {
            payload_len,
            payload,
        }
    }

    pub fn dns_resolve(name: &[u8]) -> Self {
        let mut buf = [0u8; MAX_DNS_NAME];
        let name_len = name.len().min(MAX_DNS_NAME) as u8;
        buf[..name_len as usize].copy_from_slice(&name[..name_len as usize]);
        Self::DnsResolve {
            name_len,
            name: buf,
        }
    }

    pub fn tcp_send(data: &[u8]) -> Self {
        let mut payload = [0u8; MAX_NET_TCP_PAYLOAD];
        let payload_len = data.len().min(MAX_NET_TCP_PAYLOAD) as u16;
        payload[..payload_len as usize].copy_from_slice(&data[..payload_len as usize]);
        Self::TcpSend {
            payload_len,
            payload,
        }
    }
}

/// Maximum path length for filesystem IPC (`Open`, `Create`, `Stat`, …).
///
/// Path grammar (Phase 50):
/// - Byte strings; `/` separates components; leading `/` is optional (root-relative).
/// - `""` or `"/"` is the root directory.
/// - Components are non-empty, not `.` / `..`, max 22 bytes each, max 8 components.
/// - No trailing-only empty segments (`//` collapses).
pub const MAX_FS_PATH: usize = 48;

/// Maximum name length returned in [`FsDirEntry`] (one path component).
pub const MAX_FS_NAME: usize = 24;

/// Maximum read/write payload per filesystem IPC message.
///
/// Multi-sector files use repeated `Read`/`Write` with advancing `offset`.
pub const MAX_FS_DATA: usize = 448;

/// Maximum directory entries returned in one [`FsResponse::DirList`].
pub const MAX_FS_DIR_LIST: usize = 8;

/// One directory entry in [`FsResponse::DirList`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsDirEntry {
    pub name_len: u8,
    #[serde(with = "fs_name_bytes")]
    pub name: [u8; MAX_FS_NAME],
    pub size: u32,
    /// True when the entry is a subdirectory.
    pub is_dir: bool,
}

impl FsDirEntry {
    pub fn from_name_size(name: &[u8], size: u32) -> Self {
        Self::from_name(name, size, false)
    }

    pub fn from_name(name: &[u8], size: u32, is_dir: bool) -> Self {
        let mut buf = [0u8; MAX_FS_NAME];
        let name_len = name.len().min(MAX_FS_NAME) as u8;
        buf[..name_len as usize].copy_from_slice(&name[..name_len as usize]);
        Self {
            name_len,
            name: buf,
            size,
            is_dir,
        }
    }

    pub fn name_slice(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

/// Filesystem service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "Write/Rename carry inline path/payload for IPC"
)]
pub enum FsRequest {
    Open {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    Create {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    Read {
        handle: u8,
        offset: u32,
        len: u16,
    },
    Write {
        handle: u8,
        offset: u32,
        data_len: u16,
        #[serde(with = "fs_data_bytes")]
        data: [u8; MAX_FS_DATA],
    },
    /// List directory at `path` (`""` / `"/"` = root).
    ListDir {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    Stat {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Create a directory (parents must exist unless the backend auto-creates).
    Mkdir {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Remove a file or empty directory.
    Unlink {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Rename or move `from` → `to` (same volume).
    Rename {
        from_len: u8,
        #[serde(with = "fs_path_bytes")]
        from: [u8; MAX_FS_PATH],
        to_len: u8,
        #[serde(with = "fs_path_bytes")]
        to: [u8; MAX_FS_PATH],
    },
    /// Phase 53: total/free block capacity for shell `df`.
    DiskInfo,
    Poll,
}

impl FsRequest {
    fn path_buf(path: &[u8]) -> ([u8; MAX_FS_PATH], u8) {
        let mut buf = [0u8; MAX_FS_PATH];
        let path_len = path.len().min(MAX_FS_PATH) as u8;
        buf[..path_len as usize].copy_from_slice(&path[..path_len as usize]);
        (buf, path_len)
    }

    pub fn open(path: &[u8]) -> Self {
        let (path, path_len) = Self::path_buf(path);
        Self::Open { path_len, path }
    }

    pub fn create(path: &[u8]) -> Self {
        let (path, path_len) = Self::path_buf(path);
        Self::Create { path_len, path }
    }

    pub fn stat(path: &[u8]) -> Self {
        let (path, path_len) = Self::path_buf(path);
        Self::Stat { path_len, path }
    }

    pub fn list_dir(path: &[u8]) -> Self {
        let (path, path_len) = Self::path_buf(path);
        Self::ListDir { path_len, path }
    }

    pub fn list_root() -> Self {
        Self::list_dir(b"/")
    }

    pub fn mkdir(path: &[u8]) -> Self {
        let (path, path_len) = Self::path_buf(path);
        Self::Mkdir { path_len, path }
    }

    pub fn unlink(path: &[u8]) -> Self {
        let (path, path_len) = Self::path_buf(path);
        Self::Unlink { path_len, path }
    }

    pub fn rename(from: &[u8], to: &[u8]) -> Self {
        let (from, from_len) = Self::path_buf(from);
        let (to, to_len) = Self::path_buf(to);
        Self::Rename {
            from_len,
            from,
            to_len,
            to,
        }
    }

    pub fn write(handle: u8, offset: u32, data: &[u8]) -> Self {
        let mut payload = [0u8; MAX_FS_DATA];
        let data_len = data.len().min(MAX_FS_DATA) as u16;
        payload[..data_len as usize].copy_from_slice(&data[..data_len as usize]);
        Self::Write {
            handle,
            offset,
            data_len,
            data: payload,
        }
    }
}

/// Filesystem service responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsResponse {
    Pending,
    Ok,
    Error,
    Handle {
        id: u8,
    },
    Data {
        data_len: u16,
        #[serde(with = "fs_data_bytes")]
        data: [u8; MAX_FS_DATA],
    },
    Stat {
        size: u32,
        is_dir: bool,
    },
    DirList {
        count: u8,
        entries: [FsDirEntry; MAX_FS_DIR_LIST],
    },
    /// Capacity snapshot for [`FsRequest::DiskInfo`].
    DiskInfo {
        block_size: u32,
        total_blocks: u32,
        free_blocks: u32,
    },
}

/// Max services returned by [`SupervisorResponse::ServiceList`] (Phase 40 `top`).
pub const MAX_SERVICES: usize = 8;

/// Max bytes of one service name in [`SupervisorResponse::ServiceList`].
pub const MAX_SERVICE_NAME: usize = 16;

/// Max last-error string for [`SupervisorResponse::Status`] (Phase 57).
pub const MAX_SERVICE_ERR: usize = 24;

/// Service is up and probed successfully (Phase 57).
pub const SERVICE_STATE_READY: u8 = 0;
/// Service is present but not yet probed / still starting.
pub const SERVICE_STATE_STARTING: u8 = 1;
/// Service up with a non-fatal issue.
pub const SERVICE_STATE_DEGRADED: u8 = 2;
/// Service failed probe or reported an error.
pub const SERVICE_STATE_ERROR: u8 = 3;

/// Supervisor service requests (Phase 33/34 / 53).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorRequest {
    Reboot,
    ListServices,
    ServiceStatus {
        id: u8,
    },
    GetTime,
    /// Seconds since supervisor init (Phase 53 `uptime`).
    GetUptime,
}

/// Supervisor service responses (Phase 33/34 / Phase 40 / 53 / 57).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorResponse {
    Ok,
    Error,
    Services {
        count: u8,
    },
    /// Named service table for `top` / `ps` (Phase 40 / 57).
    ServiceList {
        count: u8,
        #[serde(with = "service_name_lens")]
        name_lens: [u8; MAX_SERVICES],
        #[serde(with = "service_names_bytes")]
        names: [[u8; MAX_SERVICE_NAME]; MAX_SERVICES],
        #[serde(with = "service_ready_flags")]
        ready: [bool; MAX_SERVICES],
        /// Phase 57: [`SERVICE_STATE_READY`] … [`SERVICE_STATE_ERROR`].
        #[serde(with = "service_states_bytes")]
        states: [u8; MAX_SERVICES],
    },
    /// Phase 57: ready flag + state + optional last error string.
    Status {
        ready: bool,
        state: u8,
        err_len: u8,
        #[serde(with = "service_err_bytes")]
        err: [u8; MAX_SERVICE_ERR],
    },
    Time {
        year: u16,
        month: u8,
        day: u8,
    },
    Uptime {
        secs: u32,
    },
}

/// Config service (Phase 36 / 54).
///
/// Keys are stored as files under `/config/` (or `/config/secrets/` for
/// `secret.*` keys). See `docs/config.md` for the Phase 54 schema.
pub const MAX_CONFIG_KEY_LEN: usize = 32;
pub const MAX_CONFIG_VAL_LEN: usize = 64;

/// Well-known config keys (Phase 54 schema).
pub const CFG_NET_MODE: &[u8] = b"net.mode";
pub const CFG_NET_IP: &[u8] = b"net.ip";
pub const CFG_NET_GATEWAY: &[u8] = b"net.gateway";
pub const CFG_NET_DNS: &[u8] = b"net.dns";
pub const CFG_NET_PREFIX: &[u8] = b"net.prefix";
pub const CFG_HOSTNAME: &[u8] = b"hostname";
pub const CFG_LOG_LEVEL: &[u8] = b"log.level";
pub const CFG_BOOT_SEEDED: &[u8] = b"boot.seeded";
pub const CFG_LOG_ROTATE: &[u8] = b"log.rotate";

/// Prefix for secret keys (`secret.token` → file under `/config/secrets/`).
pub const CFG_SECRET_PREFIX: &[u8] = b"secret.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigRequest {
    Get {
        key_len: u8,
        #[serde(with = "config_key_bytes")]
        key: [u8; MAX_CONFIG_KEY_LEN],
    },
    Set {
        key_len: u8,
        #[serde(with = "config_key_bytes")]
        key: [u8; MAX_CONFIG_KEY_LEN],
        val_len: u8,
        #[serde(with = "config_val_bytes")]
        value: [u8; MAX_CONFIG_VAL_LEN],
    },
    /// Remove a key (Phase 54).
    Delete {
        key_len: u8,
        #[serde(with = "config_key_bytes")]
        key: [u8; MAX_CONFIG_KEY_LEN],
    },
    List,
}

impl ConfigRequest {
    pub fn get(key: &[u8]) -> Self {
        let mut buf = [0u8; MAX_CONFIG_KEY_LEN];
        let key_len = key.len().min(MAX_CONFIG_KEY_LEN) as u8;
        buf[..key_len as usize].copy_from_slice(&key[..key_len as usize]);
        Self::Get { key_len, key: buf }
    }

    pub fn set(key: &[u8], value: &[u8]) -> Self {
        let mut kbuf = [0u8; MAX_CONFIG_KEY_LEN];
        let key_len = key.len().min(MAX_CONFIG_KEY_LEN) as u8;
        kbuf[..key_len as usize].copy_from_slice(&key[..key_len as usize]);
        let mut vbuf = [0u8; MAX_CONFIG_VAL_LEN];
        let val_len = value.len().min(MAX_CONFIG_VAL_LEN) as u8;
        vbuf[..val_len as usize].copy_from_slice(&value[..val_len as usize]);
        Self::Set {
            key_len,
            key: kbuf,
            val_len,
            value: vbuf,
        }
    }

    pub fn delete(key: &[u8]) -> Self {
        let mut buf = [0u8; MAX_CONFIG_KEY_LEN];
        let key_len = key.len().min(MAX_CONFIG_KEY_LEN) as u8;
        buf[..key_len as usize].copy_from_slice(&key[..key_len as usize]);
        Self::Delete { key_len, key: buf }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigResponse {
    Pending,
    Ok,
    Error,
    Value {
        val_len: u8,
        #[serde(with = "config_val_bytes")]
        value: [u8; MAX_CONFIG_VAL_LEN],
    },
    Keys {
        count: u8,
        #[serde(with = "config_keys_bytes")]
        keys: [[u8; MAX_CONFIG_KEY_LEN]; 8],
        lens: [u8; 8],
    },
}

/// Max length of one log message text for LogRequest / ring.
pub const MAX_LOG_MSG: usize = 80;

/// Max number of recent log lines returned by one LogResponse::Recent (for dmesg).
pub const MAX_LOG_LINES: usize = 6;

/// Max PD tag bytes in log entries (Phase 57).
pub const MAX_LOG_TAG: usize = 8;

/// Log level: error (Phase 57; matches typical syslog severity order).
pub const LOG_LEVEL_ERROR: u8 = 1;
/// Log level: warn.
pub const LOG_LEVEL_WARN: u8 = 2;
/// Log level: info (default).
pub const LOG_LEVEL_INFO: u8 = 3;
/// Log level: debug.
pub const LOG_LEVEL_DEBUG: u8 = 4;

/// Log service requests (Phase 36 / 57 log-server).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogRequest {
    /// Append a (pre-formatted) log line to the ring buffer and output.
    Append {
        level: u8,
        tag_len: u8,
        #[serde(with = "log_tag_bytes")]
        tag: [u8; MAX_LOG_TAG],
        len: u8,
        #[serde(with = "log_msg_bytes")]
        text: [u8; MAX_LOG_MSG],
    },
    /// Subscribe for future log notifications (stub for Phase 36).
    Subscribe,
    /// Fetch recent logs, optionally filtered by min level and/or PD tag (Phase 57).
    ///
    /// `min_level == 0` means any level; `tag_len == 0` means any tag.
    GetRecent {
        min_level: u8,
        tag_len: u8,
        #[serde(with = "log_tag_bytes")]
        tag: [u8; MAX_LOG_TAG],
    },
    /// Drop lines below this level when appending (Phase 57; default info).
    SetMinLevel { level: u8 },
}

impl LogRequest {
    /// Append at info level with empty tag.
    pub fn append(text: &[u8]) -> Self {
        Self::append_tagged(LOG_LEVEL_INFO, b"", text)
    }

    /// Append with explicit level and PD tag (Phase 57).
    pub fn append_tagged(level: u8, tag: &[u8], text: &[u8]) -> Self {
        let mut tbuf = [0u8; MAX_LOG_TAG];
        let tag_len = tag.len().min(MAX_LOG_TAG) as u8;
        tbuf[..tag_len as usize].copy_from_slice(&tag[..tag_len as usize]);
        let mut buf = [0u8; MAX_LOG_MSG];
        let len = text.len().min(MAX_LOG_MSG) as u8;
        buf[..len as usize].copy_from_slice(&text[..len as usize]);
        Self::Append {
            level,
            tag_len,
            tag: tbuf,
            len,
            text: buf,
        }
    }

    /// Unfiltered recent log lines.
    pub fn get_recent() -> Self {
        Self::GetRecent {
            min_level: 0,
            tag_len: 0,
            tag: [0u8; MAX_LOG_TAG],
        }
    }

    /// Filtered recent log lines (`tag` empty = any PD).
    pub fn get_filtered(min_level: u8, tag: &[u8]) -> Self {
        let mut tbuf = [0u8; MAX_LOG_TAG];
        let tag_len = tag.len().min(MAX_LOG_TAG) as u8;
        tbuf[..tag_len as usize].copy_from_slice(&tag[..tag_len as usize]);
        Self::GetRecent {
            min_level,
            tag_len,
            tag: tbuf,
        }
    }
}

/// Log service responses (Phase 36 / 57).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "Recent carries array of log lines for dmesg"
)]
pub enum LogResponse {
    Ok,
    Error,
    Recent {
        count: u8,
        lens: [u8; MAX_LOG_LINES],
        #[serde(with = "log_lines_bytes")]
        lines: [[u8; MAX_LOG_MSG]; MAX_LOG_LINES],
        levels: [u8; MAX_LOG_LINES],
        tag_lens: [u8; MAX_LOG_LINES],
        #[serde(with = "log_tags_bytes")]
        tags: [[u8; MAX_LOG_TAG]; MAX_LOG_LINES],
    },
}

/// Network service responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "TcpData carries inline payload for IPC"
)]
pub enum NetResponse {
    Pending,
    Ok,
    Error,
    Ipv4 {
        addr: [u8; 4],
    },
    /// Current interface configuration ([`NetRequest::GetIface`]).
    Iface {
        addr: [u8; 4],
        prefix: u8,
        gateway: [u8; 4],
        dns: [u8; 4],
        /// True when the address came from DHCP (vs static fallback).
        dhcp: bool,
    },
    UdpData {
        data_len: u8,
        #[serde(with = "net_payload_bytes")]
        data: [u8; MAX_NET_UDP_PAYLOAD],
    },
    TcpData {
        data_len: u16,
        #[serde(with = "tcp_payload_bytes")]
        data: [u8; MAX_NET_TCP_PAYLOAD],
    },
}

mod fs_path_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_FS_PATH;

    pub fn serialize<S: Serializer>(
        path: &[u8; MAX_FS_PATH],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(path)
    }

    struct PathVisitor;

    impl<'de> Visitor<'de> for PathVisitor {
        type Value = [u8; MAX_FS_PATH];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max filesystem path size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_FS_PATH {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut path = [0u8; MAX_FS_PATH];
            path[..v.len()].copy_from_slice(v);
            Ok(path)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut path = [0u8; MAX_FS_PATH];
            for (i, byte) in path.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_FS_PATH - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_FS_PATH + 1, &self));
                }
            }
            Ok(path)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_FS_PATH], D::Error> {
        deserializer.deserialize_bytes(PathVisitor)
    }
}

mod fs_name_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_FS_NAME;

    pub fn serialize<S: Serializer>(
        name: &[u8; MAX_FS_NAME],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(name)
    }

    struct NameVisitor;

    impl<'de> Visitor<'de> for NameVisitor {
        type Value = [u8; MAX_FS_NAME];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max filesystem name size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_FS_NAME {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut name = [0u8; MAX_FS_NAME];
            name[..v.len()].copy_from_slice(v);
            Ok(name)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut name = [0u8; MAX_FS_NAME];
            for (i, byte) in name.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_FS_NAME - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_FS_NAME + 1, &self));
                }
            }
            Ok(name)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_FS_NAME], D::Error> {
        deserializer.deserialize_bytes(NameVisitor)
    }
}

mod fs_data_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_FS_DATA;

    pub fn serialize<S: Serializer>(
        data: &[u8; MAX_FS_DATA],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(data)
    }

    struct DataVisitor;

    impl<'de> Visitor<'de> for DataVisitor {
        type Value = [u8; MAX_FS_DATA];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max filesystem data size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_FS_DATA {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut data = [0u8; MAX_FS_DATA];
            data[..v.len()].copy_from_slice(v);
            Ok(data)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut data = [0u8; MAX_FS_DATA];
            for (i, byte) in data.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_FS_DATA - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_FS_DATA + 1, &self));
                }
            }
            Ok(data)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_FS_DATA], D::Error> {
        deserializer.deserialize_bytes(DataVisitor)
    }
}

mod sector_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::SECTOR_SIZE;

    pub fn serialize<S: Serializer>(
        data: &[u8; SECTOR_SIZE],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(data)
    }

    struct SectorVisitor;

    impl<'de> Visitor<'de> for SectorVisitor {
        type Value = [u8; SECTOR_SIZE];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of sector size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != SECTOR_SIZE {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut data = [0u8; SECTOR_SIZE];
            data.copy_from_slice(v);
            Ok(data)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut data = [0u8; SECTOR_SIZE];
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(i, &self))?;
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(SECTOR_SIZE + 1, &self));
            }
            Ok(data)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; SECTOR_SIZE], D::Error> {
        deserializer.deserialize_bytes(SectorVisitor)
    }
}

mod dns_name_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_DNS_NAME;

    pub fn serialize<S: Serializer>(
        name: &[u8; MAX_DNS_NAME],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(name)
    }

    struct NameVisitor;

    impl<'de> Visitor<'de> for NameVisitor {
        type Value = [u8; MAX_DNS_NAME];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max DNS name size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_DNS_NAME {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut name = [0u8; MAX_DNS_NAME];
            name[..v.len()].copy_from_slice(v);
            Ok(name)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut name = [0u8; MAX_DNS_NAME];
            for (i, byte) in name.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_DNS_NAME - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_DNS_NAME + 1, &self));
                }
            }
            Ok(name)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_DNS_NAME], D::Error> {
        deserializer.deserialize_bytes(NameVisitor)
    }
}

mod tcp_payload_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_NET_TCP_PAYLOAD;

    pub fn serialize<S: Serializer>(
        payload: &[u8; MAX_NET_TCP_PAYLOAD],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(payload)
    }

    struct PayloadVisitor;

    impl<'de> Visitor<'de> for PayloadVisitor {
        type Value = [u8; MAX_NET_TCP_PAYLOAD];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max net TCP payload size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_NET_TCP_PAYLOAD {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut payload = [0u8; MAX_NET_TCP_PAYLOAD];
            payload[..v.len()].copy_from_slice(v);
            Ok(payload)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut payload = [0u8; MAX_NET_TCP_PAYLOAD];
            for (i, byte) in payload.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_NET_TCP_PAYLOAD - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_NET_TCP_PAYLOAD + 1, &self));
                }
            }
            Ok(payload)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_NET_TCP_PAYLOAD], D::Error> {
        deserializer.deserialize_bytes(PayloadVisitor)
    }
}

mod net_payload_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_NET_UDP_PAYLOAD;

    pub fn serialize<S: Serializer>(
        payload: &[u8; MAX_NET_UDP_PAYLOAD],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(payload)
    }

    struct PayloadVisitor;

    impl<'de> Visitor<'de> for PayloadVisitor {
        type Value = [u8; MAX_NET_UDP_PAYLOAD];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max net UDP payload size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_NET_UDP_PAYLOAD {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut payload = [0u8; MAX_NET_UDP_PAYLOAD];
            payload[..v.len()].copy_from_slice(v);
            Ok(payload)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut payload = [0u8; MAX_NET_UDP_PAYLOAD];
            for (i, byte) in payload.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_NET_UDP_PAYLOAD - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_NET_UDP_PAYLOAD + 1, &self));
                }
            }
            Ok(payload)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_NET_UDP_PAYLOAD], D::Error> {
        deserializer.deserialize_bytes(PayloadVisitor)
    }
}

mod log_tag_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_LOG_TAG;

    pub fn serialize<S: Serializer>(
        tag: &[u8; MAX_LOG_TAG],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(tag)
    }

    struct TagVisitor;

    impl<'de> Visitor<'de> for TagVisitor {
        type Value = [u8; MAX_LOG_TAG];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max log tag size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_LOG_TAG {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut tag = [0u8; MAX_LOG_TAG];
            tag[..v.len()].copy_from_slice(v);
            Ok(tag)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut tag = [0u8; MAX_LOG_TAG];
            for (i, byte) in tag.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_LOG_TAG - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_LOG_TAG + 1, &self));
                }
            }
            Ok(tag)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_LOG_TAG], D::Error> {
        deserializer.deserialize_bytes(TagVisitor)
    }
}

mod log_tags_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::{MAX_LOG_LINES, MAX_LOG_TAG};

    pub fn serialize<S: Serializer>(
        tags: &[[u8; MAX_LOG_TAG]; MAX_LOG_LINES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut flat = [0u8; MAX_LOG_LINES * MAX_LOG_TAG];
        for (i, tag) in tags.iter().enumerate() {
            let start = i * MAX_LOG_TAG;
            flat[start..start + MAX_LOG_TAG].copy_from_slice(tag);
        }
        serializer.serialize_bytes(&flat)
    }

    struct TagsVisitor;

    impl<'de> Visitor<'de> for TagsVisitor {
        type Value = [[u8; MAX_LOG_TAG]; MAX_LOG_LINES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("flat byte array of log tags")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            let need = MAX_LOG_LINES * MAX_LOG_TAG;
            if v.len() != need {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut tags = [[0u8; MAX_LOG_TAG]; MAX_LOG_LINES];
            for (i, tag) in tags.iter_mut().enumerate() {
                let start = i * MAX_LOG_TAG;
                tag.copy_from_slice(&v[start..start + MAX_LOG_TAG]);
            }
            Ok(tags)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut flat = [0u8; MAX_LOG_LINES * MAX_LOG_TAG];
            for byte in flat.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(
                    MAX_LOG_LINES * MAX_LOG_TAG + 1,
                    &self,
                ));
            }
            let mut tags = [[0u8; MAX_LOG_TAG]; MAX_LOG_LINES];
            for (i, tag) in tags.iter_mut().enumerate() {
                let start = i * MAX_LOG_TAG;
                tag.copy_from_slice(&flat[start..start + MAX_LOG_TAG]);
            }
            Ok(tags)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[[u8; MAX_LOG_TAG]; MAX_LOG_LINES], D::Error> {
        deserializer.deserialize_bytes(TagsVisitor)
    }
}

mod log_msg_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_LOG_MSG;

    pub fn serialize<S: Serializer>(
        msg: &[u8; MAX_LOG_MSG],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(msg)
    }

    struct MsgVisitor;

    impl<'de> Visitor<'de> for MsgVisitor {
        type Value = [u8; MAX_LOG_MSG];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max log message size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_LOG_MSG {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut msg = [0u8; MAX_LOG_MSG];
            msg[..v.len()].copy_from_slice(v);
            Ok(msg)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut msg = [0u8; MAX_LOG_MSG];
            for (i, byte) in msg.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_LOG_MSG - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_LOG_MSG + 1, &self));
                }
            }
            Ok(msg)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_LOG_MSG], D::Error> {
        deserializer.deserialize_bytes(MsgVisitor)
    }
}

mod log_lines_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::{MAX_LOG_LINES, MAX_LOG_MSG};

    pub fn serialize<S: Serializer>(
        lines: &[[u8; MAX_LOG_MSG]; MAX_LOG_LINES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut flat = [0u8; MAX_LOG_LINES * MAX_LOG_MSG];
        for (i, line) in lines.iter().enumerate() {
            let start = i * MAX_LOG_MSG;
            flat[start..start + MAX_LOG_MSG].copy_from_slice(line);
        }
        serializer.serialize_bytes(&flat)
    }

    struct LinesVisitor;

    impl<'de> Visitor<'de> for LinesVisitor {
        type Value = [[u8; MAX_LOG_MSG]; MAX_LOG_LINES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of max log lines size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_LOG_LINES * MAX_LOG_MSG {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut lines = [[0u8; MAX_LOG_MSG]; MAX_LOG_LINES];
            for (i, chunk) in v.chunks(MAX_LOG_MSG).take(MAX_LOG_LINES).enumerate() {
                if let Some(dst) = lines.get_mut(i) {
                    let n = chunk.len().min(MAX_LOG_MSG);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(lines)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut flat = [0u8; MAX_LOG_LINES * MAX_LOG_MSG];
            for byte in flat.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(
                    MAX_LOG_LINES * MAX_LOG_MSG + 1,
                    &self,
                ));
            }
            let mut lines = [[0u8; MAX_LOG_MSG]; MAX_LOG_LINES];
            for (i, chunk) in flat.chunks(MAX_LOG_MSG).take(MAX_LOG_LINES).enumerate() {
                if let Some(dst) = lines.get_mut(i) {
                    let n = chunk.len().min(MAX_LOG_MSG);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(lines)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[[u8; MAX_LOG_MSG]; MAX_LOG_LINES], D::Error> {
        deserializer.deserialize_bytes(LinesVisitor)
    }
}

mod config_key_bytes {
    use core::fmt;
    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_CONFIG_KEY_LEN;

    pub fn serialize<S: Serializer>(
        key: &[u8; MAX_CONFIG_KEY_LEN],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(key)
    }

    struct KeyVisitor;

    impl<'de> Visitor<'de> for KeyVisitor {
        type Value = [u8; MAX_CONFIG_KEY_LEN];
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max config key size")
        }
        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_CONFIG_KEY_LEN {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut key = [0u8; MAX_CONFIG_KEY_LEN];
            key[..v.len()].copy_from_slice(v);
            Ok(key)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut key = [0u8; MAX_CONFIG_KEY_LEN];
            for (i, byte) in key.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_CONFIG_KEY_LEN - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_CONFIG_KEY_LEN + 1, &self));
                }
            }
            Ok(key)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_CONFIG_KEY_LEN], D::Error> {
        deserializer.deserialize_bytes(KeyVisitor)
    }
}

mod config_val_bytes {
    use core::fmt;
    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_CONFIG_VAL_LEN;

    pub fn serialize<S: Serializer>(
        val: &[u8; MAX_CONFIG_VAL_LEN],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(val)
    }

    struct ValVisitor;

    impl<'de> Visitor<'de> for ValVisitor {
        type Value = [u8; MAX_CONFIG_VAL_LEN];
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max config value size")
        }
        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_CONFIG_VAL_LEN {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut val = [0u8; MAX_CONFIG_VAL_LEN];
            val[..v.len()].copy_from_slice(v);
            Ok(val)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut val = [0u8; MAX_CONFIG_VAL_LEN];
            for (i, byte) in val.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_CONFIG_VAL_LEN - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_CONFIG_VAL_LEN + 1, &self));
                }
            }
            Ok(val)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_CONFIG_VAL_LEN], D::Error> {
        deserializer.deserialize_bytes(ValVisitor)
    }
}

mod config_keys_bytes {
    use core::fmt;
    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_CONFIG_KEY_LEN;

    pub fn serialize<S: Serializer>(
        keys: &[[u8; MAX_CONFIG_KEY_LEN]; 8],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut flat = [0u8; 8 * MAX_CONFIG_KEY_LEN];
        for (i, k) in keys.iter().enumerate() {
            let start = i * MAX_CONFIG_KEY_LEN;
            flat[start..start + MAX_CONFIG_KEY_LEN].copy_from_slice(k);
        }
        serializer.serialize_bytes(&flat)
    }

    struct KeysVisitor;

    impl<'de> Visitor<'de> for KeysVisitor {
        type Value = [[u8; MAX_CONFIG_KEY_LEN]; 8];
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of config keys")
        }
        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != 8 * MAX_CONFIG_KEY_LEN {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut keys = [[0u8; MAX_CONFIG_KEY_LEN]; 8];
            for (i, chunk) in v.chunks(MAX_CONFIG_KEY_LEN).take(8).enumerate() {
                if let Some(dst) = keys.get_mut(i) {
                    let n = chunk.len().min(MAX_CONFIG_KEY_LEN);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(keys)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut flat = [0u8; 8 * MAX_CONFIG_KEY_LEN];
            for byte in flat.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(8 * MAX_CONFIG_KEY_LEN + 1, &self));
            }
            let mut keys = [[0u8; MAX_CONFIG_KEY_LEN]; 8];
            for (i, chunk) in flat.chunks(MAX_CONFIG_KEY_LEN).take(8).enumerate() {
                if let Some(dst) = keys.get_mut(i) {
                    let n = chunk.len().min(MAX_CONFIG_KEY_LEN);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(keys)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[[u8; MAX_CONFIG_KEY_LEN]; 8], D::Error> {
        deserializer.deserialize_bytes(KeysVisitor)
    }
}

/// Chat client (Phase 40).
pub const MAX_CHAT_MSG: usize = 80;
pub const MAX_CHAT_LINES: usize = 12;

/// Requests from shell to the chat-client PD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRequest {
    /// Append and UDP-send a line of text.
    Send {
        msg_len: u8,
        #[serde(with = "chat_msg_bytes")]
        msg: [u8; MAX_CHAT_MSG],
    },
    /// Pull any inbound UDP into the local ring, then return [`ChatResponse::View`].
    Recv,
    GetView,
    Quit,
}

/// Responses from chat-client PD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "View carries chat ring for TUI redraw"
)]
pub enum ChatResponse {
    Pending,
    Ok,
    Error,
    View {
        count: u8,
        #[serde(with = "chat_line_lens")]
        line_lens: [u8; MAX_CHAT_LINES],
        #[serde(with = "chat_lines_bytes")]
        lines: [[u8; MAX_CHAT_MSG]; MAX_CHAT_LINES],
    },
}

/// Edit TUI app (Phase 38 "edit").
/// Small fixed buffers keep everything stack / postcard friendly.
pub const MAX_EDIT_LINES: usize = 24;
pub const MAX_EDIT_LINE_LEN: usize = 80;

/// Requests sent by shell (or future TUI host) to the edit PD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditRequest {
    /// Open (or create empty) a file for editing.
    Open {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Insert printable char at cursor.
    InsertChar(u8),
    Backspace,
    Newline,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    /// Write buffer back to the open path.
    Save,
    /// Snapshot for rendering (cursor + lines + path + flags).
    GetView,
    /// Leave edit mode (caller decides whether to save first).
    Quit,
}

/// Responses from edit PD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "View carries full editor lines for TUI redraw"
)]
pub enum EditResponse {
    Pending,
    Ok,
    Error,
    View {
        path_len: u8,
        #[serde(with = "fs_path_bytes")]
        path: [u8; MAX_FS_PATH],
        line_count: u8,
        #[serde(with = "edit_line_lens")]
        line_lens: [u8; MAX_EDIT_LINES],
        #[serde(with = "edit_lines_bytes")]
        lines: [[u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES],
        cursor_row: u8,
        cursor_col: u8,
        modified: bool,
    },
}

mod edit_line_lens {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_EDIT_LINES;

    pub fn serialize<S: Serializer>(
        lens: &[u8; MAX_EDIT_LINES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(lens)
    }

    struct LensVisitor;

    impl<'de> Visitor<'de> for LensVisitor {
        type Value = [u8; MAX_EDIT_LINES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of edit line lengths")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_EDIT_LINES {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut lens = [0u8; MAX_EDIT_LINES];
            lens.copy_from_slice(v);
            Ok(lens)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut lens = [0u8; MAX_EDIT_LINES];
            for byte in lens.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_EDIT_LINES + 1, &self));
            }
            Ok(lens)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_EDIT_LINES], D::Error> {
        deserializer.deserialize_bytes(LensVisitor)
    }
}

mod edit_lines_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::{MAX_EDIT_LINES, MAX_EDIT_LINE_LEN};

    pub fn serialize<S: Serializer>(
        lines: &[[u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut flat = [0u8; MAX_EDIT_LINES * MAX_EDIT_LINE_LEN];
        for (i, line) in lines.iter().enumerate() {
            let start = i * MAX_EDIT_LINE_LEN;
            flat[start..start + MAX_EDIT_LINE_LEN].copy_from_slice(line);
        }
        serializer.serialize_bytes(&flat)
    }

    struct LinesVisitor;

    impl<'de> Visitor<'de> for LinesVisitor {
        type Value = [[u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("flat byte array of edit lines")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_EDIT_LINES * MAX_EDIT_LINE_LEN {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut lines = [[0u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES];
            for (i, chunk) in v.chunks(MAX_EDIT_LINE_LEN).take(MAX_EDIT_LINES).enumerate() {
                if let Some(dst) = lines.get_mut(i) {
                    let n = chunk.len().min(MAX_EDIT_LINE_LEN);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(lines)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut flat = [0u8; MAX_EDIT_LINES * MAX_EDIT_LINE_LEN];
            for byte in flat.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(
                    MAX_EDIT_LINES * MAX_EDIT_LINE_LEN + 1,
                    &self,
                ));
            }
            let mut lines = [[0u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES];
            for (i, chunk) in flat
                .chunks(MAX_EDIT_LINE_LEN)
                .take(MAX_EDIT_LINES)
                .enumerate()
            {
                if let Some(dst) = lines.get_mut(i) {
                    let n = chunk.len().min(MAX_EDIT_LINE_LEN);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(lines)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[[u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES], D::Error> {
        deserializer.deserialize_bytes(LinesVisitor)
    }
}

mod service_name_lens {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_SERVICES;

    pub fn serialize<S: Serializer>(
        lens: &[u8; MAX_SERVICES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(lens)
    }

    struct LensVisitor;

    impl<'de> Visitor<'de> for LensVisitor {
        type Value = [u8; MAX_SERVICES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of service name lengths")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_SERVICES {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut lens = [0u8; MAX_SERVICES];
            lens.copy_from_slice(v);
            Ok(lens)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut lens = [0u8; MAX_SERVICES];
            for byte in lens.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_SERVICES + 1, &self));
            }
            Ok(lens)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_SERVICES], D::Error> {
        deserializer.deserialize_bytes(LensVisitor)
    }
}

mod service_names_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::{MAX_SERVICES, MAX_SERVICE_NAME};

    pub fn serialize<S: Serializer>(
        names: &[[u8; MAX_SERVICE_NAME]; MAX_SERVICES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut flat = [0u8; MAX_SERVICES * MAX_SERVICE_NAME];
        for (i, name) in names.iter().enumerate() {
            let start = i * MAX_SERVICE_NAME;
            flat[start..start + MAX_SERVICE_NAME].copy_from_slice(name);
        }
        serializer.serialize_bytes(&flat)
    }

    struct NamesVisitor;

    impl<'de> Visitor<'de> for NamesVisitor {
        type Value = [[u8; MAX_SERVICE_NAME]; MAX_SERVICES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("flat byte array of service names")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_SERVICES * MAX_SERVICE_NAME {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut names = [[0u8; MAX_SERVICE_NAME]; MAX_SERVICES];
            for (i, chunk) in v.chunks(MAX_SERVICE_NAME).take(MAX_SERVICES).enumerate() {
                if let Some(dst) = names.get_mut(i) {
                    let n = chunk.len().min(MAX_SERVICE_NAME);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(names)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut flat = [0u8; MAX_SERVICES * MAX_SERVICE_NAME];
            for byte in flat.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(
                    MAX_SERVICES * MAX_SERVICE_NAME + 1,
                    &self,
                ));
            }
            let mut names = [[0u8; MAX_SERVICE_NAME]; MAX_SERVICES];
            for (i, chunk) in flat.chunks(MAX_SERVICE_NAME).take(MAX_SERVICES).enumerate() {
                if let Some(dst) = names.get_mut(i) {
                    let n = chunk.len().min(MAX_SERVICE_NAME);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(names)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[[u8; MAX_SERVICE_NAME]; MAX_SERVICES], D::Error> {
        deserializer.deserialize_bytes(NamesVisitor)
    }
}

mod service_ready_flags {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_SERVICES;

    pub fn serialize<S: Serializer>(
        ready: &[bool; MAX_SERVICES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes: [u8; MAX_SERVICES] = core::array::from_fn(|i| u8::from(ready[i]));
        serializer.serialize_bytes(&bytes)
    }

    struct ReadyVisitor;

    impl<'de> Visitor<'de> for ReadyVisitor {
        type Value = [bool; MAX_SERVICES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of service ready flags")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_SERVICES {
                return Err(E::invalid_length(v.len(), &self));
            }
            Ok(core::array::from_fn(|i| v[i] != 0))
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut bytes = [0u8; MAX_SERVICES];
            for byte in bytes.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_SERVICES + 1, &self));
            }
            Ok(core::array::from_fn(|i| bytes[i] != 0))
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[bool; MAX_SERVICES], D::Error> {
        deserializer.deserialize_bytes(ReadyVisitor)
    }
}

mod service_states_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_SERVICES;

    pub fn serialize<S: Serializer>(
        states: &[u8; MAX_SERVICES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(states)
    }

    struct StatesVisitor;

    impl<'de> Visitor<'de> for StatesVisitor {
        type Value = [u8; MAX_SERVICES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of service states")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_SERVICES {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut states = [0u8; MAX_SERVICES];
            states.copy_from_slice(v);
            Ok(states)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut states = [0u8; MAX_SERVICES];
            for byte in states.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_SERVICES + 1, &self));
            }
            Ok(states)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_SERVICES], D::Error> {
        deserializer.deserialize_bytes(StatesVisitor)
    }
}

mod service_err_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_SERVICE_ERR;

    pub fn serialize<S: Serializer>(
        err: &[u8; MAX_SERVICE_ERR],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(err)
    }

    struct ErrVisitor;

    impl<'de> Visitor<'de> for ErrVisitor {
        type Value = [u8; MAX_SERVICE_ERR];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max service error size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_SERVICE_ERR {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut err = [0u8; MAX_SERVICE_ERR];
            err[..v.len()].copy_from_slice(v);
            Ok(err)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut err = [0u8; MAX_SERVICE_ERR];
            for (i, byte) in err.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_SERVICE_ERR - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_SERVICE_ERR + 1, &self));
                }
            }
            Ok(err)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_SERVICE_ERR], D::Error> {
        deserializer.deserialize_bytes(ErrVisitor)
    }
}

mod chat_msg_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_CHAT_MSG;

    pub fn serialize<S: Serializer>(
        msg: &[u8; MAX_CHAT_MSG],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(msg)
    }

    struct MsgVisitor;

    impl<'de> Visitor<'de> for MsgVisitor {
        type Value = [u8; MAX_CHAT_MSG];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of max chat message size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > MAX_CHAT_MSG {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut msg = [0u8; MAX_CHAT_MSG];
            msg[..v.len()].copy_from_slice(v);
            Ok(msg)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut msg = [0u8; MAX_CHAT_MSG];
            for (i, byte) in msg.iter_mut().enumerate() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
                if i == MAX_CHAT_MSG - 1 && seq.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(MAX_CHAT_MSG + 1, &self));
                }
            }
            Ok(msg)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_CHAT_MSG], D::Error> {
        deserializer.deserialize_bytes(MsgVisitor)
    }
}

mod chat_line_lens {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::MAX_CHAT_LINES;

    pub fn serialize<S: Serializer>(
        lens: &[u8; MAX_CHAT_LINES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(lens)
    }

    struct LensVisitor;

    impl<'de> Visitor<'de> for LensVisitor {
        type Value = [u8; MAX_CHAT_LINES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("byte array of chat line lengths")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_CHAT_LINES {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut lens = [0u8; MAX_CHAT_LINES];
            lens.copy_from_slice(v);
            Ok(lens)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut lens = [0u8; MAX_CHAT_LINES];
            for byte in lens.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(MAX_CHAT_LINES + 1, &self));
            }
            Ok(lens)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; MAX_CHAT_LINES], D::Error> {
        deserializer.deserialize_bytes(LensVisitor)
    }
}

mod chat_lines_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::{MAX_CHAT_LINES, MAX_CHAT_MSG};

    pub fn serialize<S: Serializer>(
        lines: &[[u8; MAX_CHAT_MSG]; MAX_CHAT_LINES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut flat = [0u8; MAX_CHAT_LINES * MAX_CHAT_MSG];
        for (i, line) in lines.iter().enumerate() {
            let start = i * MAX_CHAT_MSG;
            flat[start..start + MAX_CHAT_MSG].copy_from_slice(line);
        }
        serializer.serialize_bytes(&flat)
    }

    struct LinesVisitor;

    impl<'de> Visitor<'de> for LinesVisitor {
        type Value = [[u8; MAX_CHAT_MSG]; MAX_CHAT_LINES];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("flat byte array of chat lines")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != MAX_CHAT_LINES * MAX_CHAT_MSG {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut lines = [[0u8; MAX_CHAT_MSG]; MAX_CHAT_LINES];
            for (i, chunk) in v.chunks(MAX_CHAT_MSG).take(MAX_CHAT_LINES).enumerate() {
                if let Some(dst) = lines.get_mut(i) {
                    let n = chunk.len().min(MAX_CHAT_MSG);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(lines)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut flat = [0u8; MAX_CHAT_LINES * MAX_CHAT_MSG];
            for byte in flat.iter_mut() {
                match seq.next_element()? {
                    Some(value) => *byte = value,
                    None => break,
                }
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(
                    MAX_CHAT_LINES * MAX_CHAT_MSG + 1,
                    &self,
                ));
            }
            let mut lines = [[0u8; MAX_CHAT_MSG]; MAX_CHAT_LINES];
            for (i, chunk) in flat.chunks(MAX_CHAT_MSG).take(MAX_CHAT_LINES).enumerate() {
                if let Some(dst) = lines.get_mut(i) {
                    let n = chunk.len().min(MAX_CHAT_MSG);
                    dst[..n].copy_from_slice(&chunk[..n]);
                }
            }
            Ok(lines)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[[u8; MAX_CHAT_MSG]; MAX_CHAT_LINES], D::Error> {
        deserializer.deserialize_bytes(LinesVisitor)
    }
}
