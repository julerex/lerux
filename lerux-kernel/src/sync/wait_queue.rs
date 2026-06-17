//! Wait queue for kernel synchronization.
//!
//! Provides a queue of values (typically for IPC or event delivery) that supports
//! blocking receive (via associated WaitCondition) and non-blocking or blocking send.
//! Used by schemes and other kernel components for user-visible queues (e.g. events,
//! pipes, etc.).
//!
//! The queue is protected by a level-2 mutex and integrates with the lock token
//! system for safe nesting with other kernel locks.

use crate::syscall::{EAGAIN, EINTR};
use alloc::collections::VecDeque;

use crate::{
    sync::{CleanLockToken, LockToken, Mutex, WaitCondition, L1, L2},
    syscall::{
        error::{Error, Result, EINVAL},
        usercopy::UserSliceWo,
    },
};

#[derive(Debug)]
pub struct WaitQueue<T> {
    inner: Mutex<L2, VecDeque<T>>,
    pub condition: WaitCondition,
}

impl<T> WaitQueue<T> {
    /// Create a new empty wait queue.
    pub const fn new() -> WaitQueue<T> {
        WaitQueue {
            inner: Mutex::new(VecDeque::new()),
            condition: WaitCondition::new(),
        }
    }

    /// Check whether the queue currently has no pending items (under the lock).
    pub fn is_currently_empty(&self, token: &mut CleanLockToken) -> bool {
        self.inner.lock(token.token()).is_empty()
    }

    /// Receive items from the queue into a userspace buffer.
    ///
    /// If `block` is true and the queue is empty, waits using the condition
    /// (can return EINTR on signal). Copies as many items as fit, draining them.
    /// Returns the number of bytes copied or an error (EINVAL for undersized buf,
    /// EAGAIN for non-block empty, etc.).
    pub fn receive_into_user(
        &self,
        buf: UserSliceWo,
        block: bool,
        reason: &'static str,
        token: &mut CleanLockToken,
    ) -> Result<usize> {
        loop {
            let inner = self.inner.lock(token.token());
            let (mut inner, mut token) = inner.into_split();

            if inner.is_empty() {
                if block {
                    // SAFETY: Uses wait_inner because this inner is L2. It's guaranteed there's no other
                    // lock held at this point because clean token is provided from caller.
                    if !self.condition.wait_inner(inner, reason, &mut token) {
                        return Err(Error::new(EINTR));
                    }
                    continue;
                } else if buf.is_empty() {
                    return Ok(0);
                } else if buf.len() < size_of::<T>() {
                    return Err(Error::new(EINVAL));
                } else {
                    // TODO: EWOULDBLOCK?
                    return Err(Error::new(EAGAIN));
                }
            }

            let (s1, s2) = inner.as_slices();
            let s1_bytes =
                unsafe { core::slice::from_raw_parts(s1.as_ptr().cast::<u8>(), size_of_val(s1)) };
            let s2_bytes =
                unsafe { core::slice::from_raw_parts(s2.as_ptr().cast::<u8>(), size_of_val(s2)) };

            let mut bytes_copied = buf.copy_common_bytes_from_slice(s1_bytes)?;

            if let Some(buf_for_s2) = buf.advance(s1_bytes.len()) {
                bytes_copied += buf_for_s2.copy_common_bytes_from_slice(s2_bytes)?;
            }

            let _ = inner.drain(..bytes_copied / size_of::<T>());

            return Ok(bytes_copied);
        }
    }

    /// Send a value into the queue (non-locked variant).
    /// Returns the new length after the send.
    pub fn send(&self, value: T, token: &mut CleanLockToken) -> usize {
        self.send_locked(value, token.token().downgrade())
    }

    /// Send a value while holding a downgraded lock token.
    /// Notifies waiters and returns the new length.
    pub fn send_locked(&self, value: T, mut token: LockToken<'_, L1>) -> usize {
        let len = {
            let mut inner = self.inner.lock(token.token());
            inner.push_back(value);
            inner.len()
        };
        self.condition.notify_locked(token);
        len
    }
}
