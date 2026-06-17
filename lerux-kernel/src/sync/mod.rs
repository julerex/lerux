//! Kernel synchronization primitives.
//!
//! In a kernel, several CPUs run concurrently and interrupts can preempt code at
//! almost any point, so shared data must be protected. This module provides the
//! kernel's locking tools:
//!
//! - [`ordered`] — `Mutex`/`RwLock` types tagged with a compile-time **lock
//!   level** (`L0`..`L6`). A [`LockToken`] threads through the code proving you
//!   only ever take locks in increasing level order. Because every CPU acquires
//!   locks in the same order, a classic deadlock (two CPUs each holding what the
//!   other wants) becomes impossible — and the compiler enforces it.
//! - [`WaitCondition`] / [`WaitQueue`] — let a context block until some
//!   condition is signalled, instead of busy-waiting. These are how blocking
//!   syscalls put a process to sleep and wake it later.
//!
//! `CleanLockToken` (re-exported from [`ordered`]) represents "this thread holds
//! no locks" and is the starting point you split tokens from.
//!
//! See also: [`docs/kernel/architecture.md`] section 5 (locking and the
//! scheduler).
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

pub use self::{ordered::*, wait_condition::WaitCondition, wait_queue::WaitQueue};

pub mod ordered;
pub mod wait_condition;
pub mod wait_queue;
