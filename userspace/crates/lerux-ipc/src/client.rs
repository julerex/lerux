//! Typed clients for the lerux async service protocols (fs, net, blk).
//!
//! The fs/net/blk servers share one completion model: a request may answer
//! `Pending`, in which case the client drains the single in-flight operation
//! with repeated `Poll` requests. This module owns that loop (and the error
//! policy: transport/decode failures map to the protocol's `Error` response)
//! so protection domains do not each carry a divergent copy.

use serde::{Deserialize, Serialize};

use sel4_microkit::Channel;
use sel4_microkit_simple_ipc as simple_ipc;

use lerux_interface_types::{
    BlockRequest, BlockResponse, FsRequest, FsResponse, NetRequest, NetResponse,
};

/// A request/response protocol with the Pending → Poll completion model.
pub trait PollProtocol {
    type Request: Serialize;
    type Response: for<'de> Deserialize<'de>;

    /// Request that drains the in-flight operation.
    fn poll_request() -> Self::Request;
    fn is_pending(resp: &Self::Response) -> bool;
    /// Response reported when IPC transport or decode fails.
    fn transport_error() -> Self::Response;
}

/// Typed client for one service channel speaking a [`PollProtocol`].
pub struct ServiceClient<P> {
    server: Channel,
    _protocol: core::marker::PhantomData<P>,
}

impl<P: PollProtocol> ServiceClient<P> {
    pub const fn new(server: Channel) -> Self {
        Self {
            server,
            _protocol: core::marker::PhantomData,
        }
    }

    pub const fn channel(&self) -> Channel {
        self.server
    }

    /// One raw request without Pending draining.
    pub fn call_raw(&self, req: P::Request) -> P::Response {
        simple_ipc::call::<P::Request, P::Response>(self.server, req)
            .unwrap_or_else(|_| P::transport_error())
    }

    /// Call and, on `Pending`, spin `Poll` until the operation completes.
    pub fn call(&self, req: P::Request) -> P::Response {
        let resp = self.call_raw(req);
        if P::is_pending(&resp) {
            return self.poll();
        }
        resp
    }

    /// Drain the in-flight operation until it completes.
    pub fn poll(&self) -> P::Response {
        loop {
            let resp = self.call_raw(P::poll_request());
            if !P::is_pending(&resp) {
                return resp;
            }
        }
    }

    /// One non-spinning drain attempt (may return a pending response).
    pub fn poll_once(&self) -> P::Response {
        self.call_raw(P::poll_request())
    }

    /// Call and drain with at most `max_polls` polls; returns the last
    /// (possibly still pending) response. For callers that must not hang on a
    /// busy server (supervisor probes, HTTP accept paths).
    pub fn call_bounded(&self, req: P::Request, max_polls: usize) -> P::Response {
        let mut resp = self.call_raw(req);
        for _ in 0..max_polls {
            if !P::is_pending(&resp) {
                return resp;
            }
            resp = self.call_raw(P::poll_request());
        }
        resp
    }
}

/// Filesystem service protocol (`fs-server`).
pub enum FsProtocol {}

impl PollProtocol for FsProtocol {
    type Request = FsRequest;
    type Response = FsResponse;

    fn poll_request() -> FsRequest {
        FsRequest::Poll
    }

    fn is_pending(resp: &FsResponse) -> bool {
        matches!(resp, FsResponse::Pending)
    }

    fn transport_error() -> FsResponse {
        FsResponse::Error
    }
}

/// Network service protocol (`net-server`).
pub enum NetProtocol {}

impl PollProtocol for NetProtocol {
    type Request = NetRequest;
    type Response = NetResponse;

    fn poll_request() -> NetRequest {
        NetRequest::Poll
    }

    fn is_pending(resp: &NetResponse) -> bool {
        matches!(resp, NetResponse::Pending)
    }

    fn transport_error() -> NetResponse {
        NetResponse::Error
    }
}

/// Block service protocol (`blk-server`).
pub enum BlkProtocol {}

impl PollProtocol for BlkProtocol {
    type Request = BlockRequest;
    type Response = BlockResponse;

    fn poll_request() -> BlockRequest {
        BlockRequest::Poll
    }

    fn is_pending(resp: &BlockResponse) -> bool {
        matches!(resp, BlockResponse::Pending)
    }

    fn transport_error() -> BlockResponse {
        BlockResponse::Error
    }
}

pub type FsClient = ServiceClient<FsProtocol>;
pub type NetClient = ServiceClient<NetProtocol>;
pub type BlkClient = ServiceClient<BlkProtocol>;

impl FsClient {
    /// Create `path`, opening it instead when it already exists (persistent
    /// disks across smoke reruns). Returns the file handle.
    #[expect(
        clippy::result_large_err,
        reason = "Err is the FsResponse wire enum; callers need the full variant"
    )]
    pub fn create_or_open(&self, path: &[u8]) -> Result<u8, FsResponse> {
        match self.call(FsRequest::create(path)) {
            FsResponse::Handle { id } => Ok(id),
            _ => match self.call(FsRequest::open(path)) {
                FsResponse::Handle { id } => Ok(id),
                other => Err(other),
            },
        }
    }

    /// Unlink any existing file at `path`, then create it fresh. The unlink
    /// keeps the create clean when the new content is shorter (the
    /// unlink-before-create data-loss fix). Returns the file handle.
    #[expect(
        clippy::result_large_err,
        reason = "Err is the FsResponse wire enum; callers need the full variant"
    )]
    pub fn create_clean(&self, path: &[u8]) -> Result<u8, FsResponse> {
        let _ = self.call(FsRequest::unlink(path));
        match self.call(FsRequest::create(path)) {
            FsResponse::Handle { id } => Ok(id),
            other => Err(other),
        }
    }
}
