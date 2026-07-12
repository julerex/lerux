//! Stackless cooperative async for Microkit service PDs (Phase 45 / ADR-004).
//!
//! Microkit runs a single kernel thread per protection domain and delivers work
//! through [`Handler`](https://docs.rs/sel4-microkit) callbacks. This crate helps
//! express **sequential device I/O** as futures that are polled until stalled,
//! then resumed when a driver notification arrives — without stackful cothreads
//! (libmicrokitco) or a multi-task executor.
//!
//! # Pattern
//!
//! 1. Start one outstanding task (e.g. format volume, multi-sector write).
//! 2. On `protected` / `notified`, call [`run_until_stalled`].
//! 3. Device completion paths call [`WakeCell::wake`] so the task is polled again.
//! 4. When the future returns [`Poll::Ready`], reply to the client RPC.
//!
//! Clients may keep poll-based postcard RPC (`FsRequest::Poll`); this crate is
//! for **server-internal** structure only.

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

/// Store a [`Waker`] so driver notify paths can resume a stalled task.
#[derive(Default)]
pub struct WakeCell {
    waker: core::cell::Cell<Option<Waker>>,
}

impl WakeCell {
    pub const fn new() -> Self {
        Self {
            waker: core::cell::Cell::new(None),
        }
    }

    /// Register the waker from the current [`Context`] (replaces any previous).
    pub fn set(&self, waker: &Waker) {
        self.waker.set(Some(waker.clone()));
    }

    /// Wake the registered task, if any.
    pub fn wake(&self) {
        if let Some(w) = self.waker.take() {
            w.wake();
        }
    }

    pub fn clear(&self) {
        self.waker.set(None);
    }
}

/// Latch set by `notified` and observed by futures waiting on a channel.
#[derive(Default)]
pub struct EventFlag {
    set: AtomicBool,
}

impl EventFlag {
    pub const fn new() -> Self {
        Self {
            set: AtomicBool::new(false),
        }
    }

    pub fn signal(&self) {
        self.set.store(true, Ordering::Release);
    }

    /// Returns true if the flag was set (and clears it).
    pub fn take(&self) -> bool {
        self.set.swap(false, Ordering::AcqRel)
    }

    pub fn is_set(&self) -> bool {
        self.set.load(Ordering::Acquire)
    }
}

/// Poll `f` once with the given context.
pub fn poll_fn<T, F>(f: F) -> PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    PollFn { f }
}

/// Future returned by [`poll_fn`].
pub struct PollFn<F> {
    f: F,
}

impl<T, F> Future for PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        // Safety: we never move `f` out; only call it.
        let this = unsafe { self.get_unchecked_mut() };
        (this.f)(cx)
    }
}

/// One outstanding boxed future, driven from Handler callbacks.
pub struct SingleTask<T> {
    fut: Option<Pin<Box<dyn Future<Output = T> + 'static>>>,
}

impl<T> SingleTask<T> {
    pub fn empty() -> Self {
        Self { fut: None }
    }

    pub fn is_idle(&self) -> bool {
        self.fut.is_none()
    }

    pub fn is_running(&self) -> bool {
        self.fut.is_some()
    }

    /// Install a new task. Panics if one is already running.
    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = T> + 'static,
    {
        assert!(self.fut.is_none(), "SingleTask already running");
        self.fut = Some(Box::pin(future));
    }

    /// Poll until pending or complete. Returns [`Some`] when the task finishes.
    pub fn run_until_stalled(&mut self) -> Option<T> {
        let fut = self.fut.as_mut()?;
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(out) => {
                self.fut = None;
                Some(out)
            }
            Poll::Pending => None,
        }
    }

    /// Drop the task without completing it (e.g. abort).
    pub fn cancel(&mut self) {
        self.fut = None;
    }
}

fn noop_raw_waker() -> RawWaker {
    fn clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    fn wake(_: *const ()) {}
    fn wake_by_ref(_: *const ()) {}
    fn drop(_: *const ()) {}
    RawWaker::new(
        core::ptr::null(),
        &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
    )
}

/// Waker that does nothing when woken (polling is driven explicitly by Handler).
pub fn noop_waker() -> Waker {
    // Safety: vtable is no-op and never dereferences the data pointer.
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

/// Poll a pinned future once with a no-op waker.
pub fn poll_once<F: Future>(fut: Pin<&mut F>) -> Poll<F::Output> {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    fut.poll(&mut cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::rc::Rc;
    use core::cell::Cell;

    #[test]
    fn single_task_completes_immediately() {
        let mut task = SingleTask::empty();
        task.spawn(async { 42u32 });
        assert_eq!(task.run_until_stalled(), Some(42));
        assert!(task.is_idle());
    }

    #[test]
    fn single_task_pending_then_ready() {
        let ready = Rc::new(Cell::new(false));
        let ready2 = ready.clone();
        let mut task = SingleTask::empty();
        task.spawn(async move {
            poll_fn(|_| {
                if ready2.get() {
                    Poll::Ready(7u8)
                } else {
                    Poll::Pending
                }
            })
            .await
        });
        assert_eq!(task.run_until_stalled(), None);
        ready.set(true);
        assert_eq!(task.run_until_stalled(), Some(7));
    }

    #[test]
    fn event_flag_take() {
        let e = EventFlag::new();
        assert!(!e.take());
        e.signal();
        assert!(e.take());
        assert!(!e.take());
    }

    #[test]
    fn wake_cell_invokes_waker() {
        use alloc::sync::Arc;
        use core::{
            sync::atomic::{AtomicUsize, Ordering},
            task::{RawWaker, RawWakerVTable, Waker},
        };

        struct Count(AtomicUsize);
        fn clone(p: *const ()) -> RawWaker {
            // Safety: pointer is Arc::into_raw
            unsafe { Arc::increment_strong_count(p as *const Count) };
            RawWaker::new(p, &VTABLE)
        }
        fn wake(p: *const ()) {
            wake_by_ref(p);
            // Safety: balances into_raw from waker construction
            unsafe { Arc::from_raw(p as *const Count) };
        }
        fn wake_by_ref(p: *const ()) {
            let c = unsafe { &*(p as *const Count) };
            c.0.fetch_add(1, Ordering::SeqCst);
        }
        fn drop_w(p: *const ()) {
            unsafe { Arc::from_raw(p as *const Count) };
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_w);

        let count = Arc::new(Count(AtomicUsize::new(0)));
        let ptr = Arc::into_raw(count.clone()) as *const ();
        let waker = unsafe { Waker::from_raw(RawWaker::new(ptr, &VTABLE)) };

        let cell = WakeCell::new();
        cell.set(&waker);
        cell.wake();
        assert_eq!(count.0.load(Ordering::SeqCst), 1);
    }
}
