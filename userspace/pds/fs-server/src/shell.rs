//! Format-independent fs-server shell.
//!
//! Owns everything the filesystem format does not: the client channel table,
//! single-active-client arbitration, request validation, the Poll RPC, and the
//! Microkit [`Handler`] plumbing. The format-specific job machines live behind
//! the [`FsFormat`] seam (`leruxfs_format` / `fat_format`).

use lerux_interface_types::{FsRequest, FsResponse, MAX_FS_DATA, MAX_FS_PATH};
use lerux_ipc::{recv, send, send_unspecified_error};
use sel4_microkit::{Channel, ChannelSet, Handler, Infallible, MessageInfo};

use crate::block_io::BLK_DRIVER;

/// Client ends of the fs-server, as assigned by `support/profiles/workstation*.toml`.
/// Channels 1 (blk driver) and 4 (log server) are not PPC clients.
pub const SUPERVISOR: Channel = Channel::new(2);
pub const SHELL: Channel = Channel::new(3);
pub const CONFIG_SERVER: Channel = Channel::new(5);
pub const EDIT: Channel = Channel::new(6);
pub const HTTP_FILE_BROWSER: Channel = Channel::new(7);
pub const BACKUP: Channel = Channel::new(8);

const CLIENTS: [Channel; 6] = [
    SUPERVISOR,
    SHELL,
    CONFIG_SERVER,
    EDIT,
    HTTP_FILE_BROWSER,
    BACKUP,
];

/// Seam between the server shell and a filesystem format implementation.
///
/// The shell guarantees `begin` is only called while [`FsFormat::busy`] is
/// false, and that request payload lengths are already validated.
pub trait FsFormat {
    /// Start the job for `req`. `Some(resp)` completes the request
    /// synchronously (`FsResponse::Pending` keeps the client reserved, e.g.
    /// while an initial mount runs); `None` queues a job that the shell will
    /// drive via [`FsFormat::advance`].
    fn begin(&mut self, req: FsRequest) -> Option<FsResponse>;

    /// Drive the in-flight job forward. `Some` is the finished response.
    fn advance(&mut self) -> Option<FsResponse>;

    /// True while a job or mount/format task is in flight.
    fn busy(&self) -> bool;

    /// True while a sector I/O request is outstanding on the block ring.
    fn io_busy(&self) -> bool;

    /// Poll-time housekeeping before [`FsFormat::advance`] (drain coalesced
    /// blk completions, drive mount/format tasks). `Some` is a finished
    /// response.
    fn poll_progress(&mut self) -> Option<FsResponse> {
        None
    }

    /// A block-driver notification arrived. `Some` completes a pending job
    /// (e.g. an async mount finishing).
    fn on_blk_notified(&mut self) -> Option<FsResponse>;
}

fn request_lengths_valid(req: &FsRequest) -> bool {
    match req {
        FsRequest::Open { path_len, .. }
        | FsRequest::Create { path_len, .. }
        | FsRequest::Stat { path_len, .. }
        | FsRequest::ListDir { path_len, .. }
        | FsRequest::Mkdir { path_len, .. }
        | FsRequest::Unlink { path_len, .. } => *path_len as usize <= MAX_FS_PATH,
        FsRequest::Rename {
            from_len, to_len, ..
        } => *from_len as usize <= MAX_FS_PATH && *to_len as usize <= MAX_FS_PATH,
        FsRequest::Write { data_len, .. } => *data_len as usize <= MAX_FS_DATA,
        FsRequest::Read { .. } | FsRequest::DiskInfo | FsRequest::Poll => true,
    }
}

/// One fs-server shell over any [`FsFormat`] adapter.
pub struct FsServer<F> {
    format: F,
    completed: Option<FsResponse>,
    /// Client that owns the in-flight async operation (or pending completion).
    active_client: Option<Channel>,
}

impl<F: FsFormat> FsServer<F> {
    pub fn new(format: F) -> Self {
        Self {
            format,
            completed: None,
            active_client: None,
        }
    }

    fn is_client(channel: Channel) -> bool {
        CLIENTS.contains(&channel)
    }

    /// Reserve this client for an async op. Returns false when another client
    /// owns the job machine or this client still has an undelivered completion.
    fn begin_async(&mut self, channel: Channel) -> bool {
        if self.completed.is_some() {
            return false;
        }
        if self.format.busy() && self.active_client != Some(channel) {
            return false;
        }
        self.active_client = Some(channel);
        true
    }

    fn finish_async(&mut self) {
        self.active_client = None;
    }

    fn take_completed(&mut self, channel: Channel) -> Option<FsResponse> {
        if self.active_client != Some(channel) {
            return None;
        }
        let resp = self.completed.take()?;
        self.finish_async();
        Some(resp)
    }

    /// Deliver a synchronous response, releasing the client unless it must
    /// keep polling (`Pending`).
    fn sync_response(&mut self, resp: FsResponse) -> FsResponse {
        if !matches!(resp, FsResponse::Pending) {
            self.finish_async();
        }
        resp
    }

    fn complete(&mut self, channel: Channel, resp: FsResponse) -> FsResponse {
        self.completed = Some(resp);
        self.take_completed(channel).unwrap_or(FsResponse::Pending)
    }

    fn handle_request(&mut self, channel: Channel, req: FsRequest) -> FsResponse {
        if matches!(req, FsRequest::Poll) {
            return self.handle_poll(channel);
        }
        if !self.begin_async(channel) {
            return FsResponse::Pending;
        }
        if self.completed.is_some() || self.format.busy() {
            return FsResponse::Pending;
        }
        if !request_lengths_valid(&req) {
            return self.sync_response(FsResponse::Error);
        }
        if let Some(resp) = self.format.begin(req) {
            return self.sync_response(resp);
        }
        match self.format.advance() {
            Some(resp) => self.complete(channel, resp),
            None => FsResponse::Pending,
        }
    }

    fn handle_poll(&mut self, channel: Channel) -> FsResponse {
        if let Some(resp) = self.take_completed(channel) {
            return resp;
        }
        if self.active_client != Some(channel) {
            return FsResponse::Pending;
        }
        if let Some(resp) = self.format.poll_progress() {
            return self.complete(channel, resp);
        }
        if let Some(resp) = self.format.advance() {
            return self.complete(channel, resp);
        }
        // Notify the driver in case a completion notify was coalesced while we
        // were only handling PPC Polls (busy-wait clients).
        if self.format.io_busy() {
            BLK_DRIVER.notify();
        }
        FsResponse::Pending
    }
}

impl<F: FsFormat> Handler for FsServer<F> {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if !Self::is_client(channel) {
            unreachable!("unexpected fs client");
        }
        Ok(match recv::<FsRequest>(msg_info) {
            Ok(req) => send(self.handle_request(channel, req)),
            Err(_) => send_unspecified_error(),
        })
    }

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(BLK_DRIVER)
            && let Some(resp) = self.format.on_blk_notified()
        {
            self.completed = Some(resp);
        }
        Ok(())
    }
}
