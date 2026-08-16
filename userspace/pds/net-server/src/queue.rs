//! Phase 64: per-client in-flight slot so two net clients can overlap.

use lerux_interface_types::{NetRequest, NetResponse};

pub const MAX_NET_CLIENTS: usize = 8;

#[derive(Clone, Copy)]
pub struct ClientSlot {
    pub inflight: bool,
    pub completed: Option<NetResponse>,
    pub queued: Option<NetRequest>,
}

impl ClientSlot {
    const EMPTY: Self = Self {
        inflight: false,
        completed: None,
        queued: None,
    };
}

pub struct ClientQueue {
    slots: [ClientSlot; MAX_NET_CLIENTS],
    pub current: Option<usize>,
}

impl ClientQueue {
    pub const fn new() -> Self {
        Self {
            slots: [ClientSlot::EMPTY; MAX_NET_CLIENTS],
            current: None,
        }
    }

    fn slot(&mut self, idx: usize) -> Option<&mut ClientSlot> {
        self.slots.get_mut(idx)
    }

    /// Reserve this client. Returns `false` if they already have work.
    pub fn begin(&mut self, idx: usize) -> bool {
        let Some(s) = self.slot(idx) else {
            return false;
        };
        if s.inflight || s.completed.is_some() || s.queued.is_some() {
            return false;
        }
        s.inflight = true;
        true
    }

    pub fn queue_request(&mut self, idx: usize, req: NetRequest) {
        if let Some(s) = self.slot(idx) {
            s.queued = Some(req);
        }
    }

    pub fn take_queued(&mut self, idx: usize) -> Option<NetRequest> {
        self.slot(idx).and_then(|s| s.queued.take())
    }

    pub fn finish(&mut self, idx: usize) {
        if let Some(s) = self.slot(idx) {
            s.inflight = false;
            s.queued = None;
        }
        if self.current == Some(idx) {
            self.current = None;
        }
    }

    pub fn abort(&mut self, idx: usize) {
        if let Some(s) = self.slot(idx) {
            *s = ClientSlot::EMPTY;
        }
        if self.current == Some(idx) {
            self.current = None;
        }
    }

    pub fn stash(&mut self, idx: usize, resp: NetResponse) {
        if let Some(s) = self.slot(idx) {
            s.completed = Some(resp);
        }
    }

    pub fn take_completed(&mut self, idx: usize) -> Option<NetResponse> {
        self.slot(idx).and_then(|s| s.completed.take())
    }

    /// Next client with a queued request, skipping `current`.
    pub fn next_queued(&self) -> Option<usize> {
        self.slots.iter().enumerate().find_map(|(i, s)| {
            if s.queued.is_some() && self.current != Some(i) {
                Some(i)
            } else {
                None
            }
        })
    }
}
