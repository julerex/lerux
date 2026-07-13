//! Shared postcard RPC message types for lerux protection domains.
//!
//! Fixed-capacity byte fields use the generic serde adapters at the bottom of
//! this file ([`bounded_bytes`], [`exact_bytes`], [`flat_matrix_bytes`],
//! [`bool_flags`]); all of them encode as a postcard byte string so the wire
//! format is independent of the helper used.

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
        #[serde(with = "exact_bytes")]
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
        #[serde(with = "exact_bytes")]
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
        #[serde(with = "bounded_bytes")]
        payload: [u8; MAX_NET_UDP_PAYLOAD],
    },
    /// Receive one UDP datagram previously bound via [`NetRequest::UdpTx`] / listen port.
    UdpRecv,
    DnsResolve {
        name_len: u8,
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
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
    #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    Create {
        path_len: u8,
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
        data: [u8; MAX_FS_DATA],
    },
    /// List directory at `path` (`""` / `"/"` = root).
    ListDir {
        path_len: u8,
        #[serde(with = "bounded_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    Stat {
        path_len: u8,
        #[serde(with = "bounded_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Create a directory (parents must exist unless the backend auto-creates).
    Mkdir {
        path_len: u8,
        #[serde(with = "bounded_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Remove a file or empty directory.
    Unlink {
        path_len: u8,
        #[serde(with = "bounded_bytes")]
        path: [u8; MAX_FS_PATH],
    },
    /// Rename or move `from` → `to` (same volume).
    Rename {
        from_len: u8,
        #[serde(with = "bounded_bytes")]
        from: [u8; MAX_FS_PATH],
        to_len: u8,
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "exact_bytes")]
        name_lens: [u8; MAX_SERVICES],
        #[serde(with = "flat_matrix_bytes")]
        names: [[u8; MAX_SERVICE_NAME]; MAX_SERVICES],
        #[serde(with = "bool_flags")]
        ready: [bool; MAX_SERVICES],
        /// Phase 57: [`SERVICE_STATE_READY`] … [`SERVICE_STATE_ERROR`].
        #[serde(with = "exact_bytes")]
        states: [u8; MAX_SERVICES],
    },
    /// Phase 57: ready flag + state + optional last error string.
    Status {
        ready: bool,
        state: u8,
        err_len: u8,
        #[serde(with = "bounded_bytes")]
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

/// Max keys returned by one [`ConfigResponse::Keys`].
pub const MAX_CONFIG_KEYS: usize = 8;

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
        #[serde(with = "bounded_bytes")]
        key: [u8; MAX_CONFIG_KEY_LEN],
    },
    Set {
        key_len: u8,
        #[serde(with = "bounded_bytes")]
        key: [u8; MAX_CONFIG_KEY_LEN],
        val_len: u8,
        #[serde(with = "bounded_bytes")]
        value: [u8; MAX_CONFIG_VAL_LEN],
    },
    /// Remove a key (Phase 54).
    Delete {
        key_len: u8,
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
        value: [u8; MAX_CONFIG_VAL_LEN],
    },
    Keys {
        count: u8,
        #[serde(with = "flat_matrix_bytes")]
        keys: [[u8; MAX_CONFIG_KEY_LEN]; MAX_CONFIG_KEYS],
        lens: [u8; MAX_CONFIG_KEYS],
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
        #[serde(with = "bounded_bytes")]
        tag: [u8; MAX_LOG_TAG],
        len: u8,
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "flat_matrix_bytes")]
        lines: [[u8; MAX_LOG_MSG]; MAX_LOG_LINES],
        levels: [u8; MAX_LOG_LINES],
        tag_lens: [u8; MAX_LOG_LINES],
        #[serde(with = "flat_matrix_bytes")]
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
        #[serde(with = "bounded_bytes")]
        data: [u8; MAX_NET_UDP_PAYLOAD],
    },
    TcpData {
        data_len: u16,
        #[serde(with = "bounded_bytes")]
        data: [u8; MAX_NET_TCP_PAYLOAD],
    },
}

/// Chat client (Phase 40 / 58 multi-room).
pub const MAX_CHAT_MSG: usize = 80;
pub const MAX_CHAT_LINES: usize = 12;
/// Max room name bytes (Phase 58).
pub const MAX_CHAT_ROOM: usize = 8;
/// Max concurrent rooms tracked by chat-client.
pub const MAX_CHAT_ROOMS: usize = 4;

/// Requests from shell to the chat-client PD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRequest {
    /// Join (or switch to) a room; empty room → `lobby`.
    Join {
        room_len: u8,
        #[serde(with = "bounded_bytes")]
        room: [u8; MAX_CHAT_ROOM],
    },
    /// Append and UDP-send a line of text in the current room.
    Send {
        msg_len: u8,
        #[serde(with = "bounded_bytes")]
        msg: [u8; MAX_CHAT_MSG],
    },
    /// Pull any inbound UDP into the local ring, then return [`ChatResponse::View`].
    Recv,
    GetView,
    /// List joined rooms (names in View lines).
    ListRooms,
    Quit,
}

impl ChatRequest {
    pub fn join(room: &[u8]) -> Self {
        let mut buf = [0u8; MAX_CHAT_ROOM];
        let room_len = room.len().min(MAX_CHAT_ROOM) as u8;
        buf[..room_len as usize].copy_from_slice(&room[..room_len as usize]);
        Self::Join {
            room_len,
            room: buf,
        }
    }
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
        #[serde(with = "exact_bytes")]
        line_lens: [u8; MAX_CHAT_LINES],
        #[serde(with = "flat_matrix_bytes")]
        lines: [[u8; MAX_CHAT_MSG]; MAX_CHAT_LINES],
        /// Current room name (Phase 58).
        room_len: u8,
        #[serde(with = "bounded_bytes")]
        room: [u8; MAX_CHAT_ROOM],
    },
}

/// Backup / sync service (Phase 58).
pub const MAX_BACKUP_PATH: usize = MAX_FS_PATH;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupRequest {
    /// Snapshot root listing into `/backup/manifest` (creates `/backup/`).
    Snapshot,
    /// Report last snapshot stats.
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupResponse {
    Ok,
    Error,
    /// `files` = entries written to the manifest.
    Report {
        files: u8,
        bytes: u32,
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
        #[serde(with = "bounded_bytes")]
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
        #[serde(with = "bounded_bytes")]
        path: [u8; MAX_FS_PATH],
        line_count: u8,
        #[serde(with = "exact_bytes")]
        line_lens: [u8; MAX_EDIT_LINES],
        #[serde(with = "flat_matrix_bytes")]
        lines: [[u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES],
        cursor_row: u8,
        cursor_col: u8,
        modified: bool,
    },
}

/// Serde adapter for `[u8; N]` fields whose useful length is tracked by a
/// separate `*_len` field.
///
/// Encodes as a postcard byte string of exactly `N` bytes (the full,
/// zero-padded buffer). Deserialization accepts up to `N` bytes and zero-pads,
/// so shorter (older) senders remain decodable.
mod bounded_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S: Serializer, const N: usize>(
        bytes: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(bytes)
    }

    struct BytesVisitor<const N: usize>;

    impl<'de, const N: usize> Visitor<'de> for BytesVisitor<N> {
        type Value = [u8; N];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a byte string of at most {N} bytes")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() > N {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut out = [0u8; N];
            out[..v.len()].copy_from_slice(v);
            Ok(out)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = [0u8; N];
            let mut len = 0usize;
            while let Some(value) = seq.next_element::<u8>()? {
                if len == N {
                    return Err(de::Error::invalid_length(N + 1, &self));
                }
                out[len] = value;
                len += 1;
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error> {
        deserializer.deserialize_bytes(BytesVisitor::<N>)
    }
}

/// Serde adapter for `[u8; N]` fields where every byte is meaningful
/// (sector payloads, per-slot length tables).
///
/// Encodes as a postcard byte string of exactly `N` bytes; deserialization
/// rejects any other length.
mod exact_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S: Serializer, const N: usize>(
        bytes: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(bytes)
    }

    struct BytesVisitor<const N: usize>;

    impl<'de, const N: usize> Visitor<'de> for BytesVisitor<N> {
        type Value = [u8; N];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a byte string of exactly {N} bytes")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != N {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut out = [0u8; N];
            out.copy_from_slice(v);
            Ok(out)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = [0u8; N];
            for (i, byte) in out.iter_mut().enumerate() {
                *byte = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(i, &self))?;
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(N + 1, &self));
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error> {
        deserializer.deserialize_bytes(BytesVisitor::<N>)
    }
}

/// Serde adapter for `[[u8; M]; L]` fields (fixed tables of fixed-size rows).
///
/// Encodes the rows row-major as one postcard byte string of exactly `M * L`
/// bytes; deserialization rejects any other length.
mod flat_matrix_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S: Serializer, const M: usize, const L: usize>(
        rows: &[[u8; M]; L],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(rows.as_flattened())
    }

    struct MatrixVisitor<const M: usize, const L: usize>;

    impl<'de, const M: usize, const L: usize> Visitor<'de> for MatrixVisitor<M, L> {
        type Value = [[u8; M]; L];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a byte string of exactly {} bytes", M * L)
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != M * L {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut rows = [[0u8; M]; L];
            rows.as_flattened_mut().copy_from_slice(v);
            Ok(rows)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut rows = [[0u8; M]; L];
            let mut len = 0usize;
            {
                let flat = rows.as_flattened_mut();
                while let Some(value) = seq.next_element::<u8>()? {
                    if len == M * L {
                        return Err(de::Error::invalid_length(M * L + 1, &self));
                    }
                    flat[len] = value;
                    len += 1;
                }
            }
            Ok(rows)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const M: usize, const L: usize>(
        deserializer: D,
    ) -> Result<[[u8; M]; L], D::Error> {
        deserializer.deserialize_bytes(MatrixVisitor::<M, L>)
    }
}

/// Serde adapter for `[bool; N]` flag tables.
///
/// Encodes as a postcard byte string of exactly `N` bytes (0 / 1 per flag);
/// deserialization rejects any other length and maps non-zero to `true`.
mod bool_flags {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S: Serializer, const N: usize>(
        flags: &[bool; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes: [u8; N] = core::array::from_fn(|i| u8::from(flags[i]));
        serializer.serialize_bytes(&bytes)
    }

    struct FlagsVisitor<const N: usize>;

    impl<'de, const N: usize> Visitor<'de> for FlagsVisitor<N> {
        type Value = [bool; N];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a byte string of exactly {N} flag bytes")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != N {
                return Err(E::invalid_length(v.len(), &self));
            }
            Ok(core::array::from_fn(|i| v[i] != 0))
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut bytes = [0u8; N];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(i, &self))?;
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(N + 1, &self));
            }
            Ok(core::array::from_fn(|i| bytes[i] != 0))
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[bool; N], D::Error> {
        deserializer.deserialize_bytes(FlagsVisitor::<N>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Large enough for the biggest message (EditResponse::View ≈ 2 KiB).
    const BUF: usize = 4096;

    fn round_trip<T>(value: T) -> T
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let mut buf = [0u8; BUF];
        let used = postcard::to_slice(&value, &mut buf).expect("serialize");
        postcard::from_bytes(used).expect("deserialize")
    }

    #[test]
    fn echo_round_trip() {
        let req = EchoRequest::echo(b"hello");
        assert_eq!(round_trip(req), req);
        let resp = EchoResponse::Echo {
            len: 5,
            text: {
                let mut t = [0u8; MAX_ECHO_LEN];
                t[..5].copy_from_slice(b"hello");
                t
            },
        };
        assert_eq!(round_trip(resp).as_echo_slice(), Some(&b"hello"[..]));
    }

    #[test]
    fn block_sector_round_trip() {
        let mut data = [0u8; SECTOR_SIZE];
        for (i, b) in data.iter_mut().enumerate() {
            *b = i as u8;
        }
        let req = BlockRequest::WriteSector { lba: 7, data };
        assert_eq!(round_trip(req), req);
        let resp = BlockResponse::Sector { data };
        assert_eq!(round_trip(resp.clone()), resp);
    }

    #[test]
    fn net_round_trip() {
        for req in [
            NetRequest::udp_tx(b"ping"),
            NetRequest::dns_resolve(b"example.com"),
            NetRequest::tcp_send(&[0xAB; MAX_NET_TCP_PAYLOAD]),
            NetRequest::TcpConnect {
                addr: [10, 0, 0, 1],
                port: 80,
            },
        ] {
            assert_eq!(round_trip(req), req);
        }
        let resp = NetResponse::Iface {
            addr: [192, 168, 1, 2],
            prefix: 24,
            gateway: [192, 168, 1, 1],
            dns: [1, 1, 1, 1],
            dhcp: true,
        };
        assert_eq!(round_trip(resp), resp);
    }

    #[test]
    fn fs_round_trip() {
        for req in [
            FsRequest::open(b"/etc/motd"),
            FsRequest::rename(b"/a", b"/b"),
            FsRequest::write(3, 96, b"payload"),
            FsRequest::DiskInfo,
        ] {
            assert_eq!(round_trip(req), req);
        }
        let entries =
            core::array::from_fn(|i| FsDirEntry::from_name(b"entry", (i * 10) as u32, i % 2 == 0));
        let resp = FsResponse::DirList { count: 8, entries };
        assert_eq!(round_trip(resp), resp);
    }

    #[test]
    fn supervisor_service_list_round_trip() {
        let mut names = [[0u8; MAX_SERVICE_NAME]; MAX_SERVICES];
        names[0][..5].copy_from_slice(b"shell");
        names[1][..3].copy_from_slice(b"net");
        let resp = SupervisorResponse::ServiceList {
            count: 2,
            name_lens: [5, 3, 0, 0, 0, 0, 0, 0],
            names,
            ready: [true, false, true, false, true, false, true, false],
            states: [
                SERVICE_STATE_READY,
                SERVICE_STATE_STARTING,
                SERVICE_STATE_DEGRADED,
                SERVICE_STATE_ERROR,
                0,
                0,
                0,
                0,
            ],
        };
        assert_eq!(round_trip(resp), resp);
    }

    #[test]
    fn config_round_trip() {
        let req = ConfigRequest::set(CFG_HOSTNAME, b"lerux");
        assert_eq!(round_trip(req), req);
        let mut keys = [[0u8; MAX_CONFIG_KEY_LEN]; MAX_CONFIG_KEYS];
        keys[0][..8].copy_from_slice(b"hostname");
        let resp = ConfigResponse::Keys {
            count: 1,
            keys,
            lens: [8, 0, 0, 0, 0, 0, 0, 0],
        };
        assert_eq!(round_trip(resp), resp);
    }

    #[test]
    fn log_round_trip() {
        let req = LogRequest::append_tagged(LOG_LEVEL_WARN, b"shell", b"disk almost full");
        assert_eq!(round_trip(req), req);
        let mut lines = [[0u8; MAX_LOG_MSG]; MAX_LOG_LINES];
        lines[0][..4].copy_from_slice(b"boot");
        let mut tags = [[0u8; MAX_LOG_TAG]; MAX_LOG_LINES];
        tags[0][..4].copy_from_slice(b"init");
        let resp = LogResponse::Recent {
            count: 1,
            lens: [4, 0, 0, 0, 0, 0],
            lines,
            levels: [LOG_LEVEL_INFO, 0, 0, 0, 0, 0],
            tag_lens: [4, 0, 0, 0, 0, 0],
            tags,
        };
        assert_eq!(round_trip(resp), resp);
    }

    #[test]
    fn chat_and_edit_round_trip() {
        let req = ChatRequest::join(b"lobby");
        assert_eq!(round_trip(req), req);
        let mut lines = [[0u8; MAX_CHAT_MSG]; MAX_CHAT_LINES];
        lines[0][..2].copy_from_slice(b"hi");
        let resp = ChatResponse::View {
            count: 1,
            line_lens: [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            lines,
            room_len: 5,
            room: *b"lobby\0\0\0",
        };
        assert_eq!(round_trip(resp), resp);

        let mut edit_lines = [[0u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES];
        edit_lines[0][..5].copy_from_slice(b"line1");
        let view = EditResponse::View {
            path_len: 5,
            path: {
                let mut p = [0u8; MAX_FS_PATH];
                p[..5].copy_from_slice(b"/file");
                p
            },
            line_count: 1,
            line_lens: [0u8; MAX_EDIT_LINES],
            lines: edit_lines,
            cursor_row: 0,
            cursor_col: 5,
            modified: true,
        };
        assert_eq!(round_trip(view), view);
    }

    #[test]
    fn bounded_bytes_rejects_oversize() {
        // A DnsResolve whose name byte string carries more than MAX_DNS_NAME
        // bytes must fail to decode rather than truncate silently.
        // Wire layout: variant tag, name_len, byte-string length prefix, bytes.
        let mut wire = [0u8; 3 + MAX_DNS_NAME + 1];
        wire[0] = 2; // NetRequest::DnsResolve variant index
        wire[1] = 11; // name_len
        wire[2] = (MAX_DNS_NAME + 1) as u8; // oversized byte-string length
        assert!(postcard::from_bytes::<NetRequest>(&wire).is_err());
    }

    #[test]
    fn exact_bytes_rejects_short_sector() {
        // Undersized sector payloads must be rejected, not zero-padded.
        // Wire layout: variant tag, lba varint, byte-string length prefix, bytes.
        let mut wire = [0u8; 4 + SECTOR_SIZE - 1];
        wire[0] = 1; // BlockRequest::WriteSector variant index
        wire[1] = 1; // lba = 1
                     // 511-byte string (varint 0x1FF).
        wire[2] = 0xFF;
        wire[3] = 0x03;
        assert!(postcard::from_bytes::<BlockRequest>(&wire).is_err());
    }
}
