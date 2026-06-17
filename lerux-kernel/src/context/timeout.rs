//! Timed wakeups: deliver an event when a deadline passes.
//!
//! When a context blocks "until time T" (for example a sleep, or a syscall with
//! a timeout), it registers a [`Timeout`] here. The time subsystem periodically
//! checks the registry and, once a deadline is reached, fires the associated
//! event so the waiting context becomes runnable again.
//!
//! This is what lets a [`Blocked`](crate::context::Status::Blocked) context wake
//! itself up without anyone explicitly unblocking it.
//!
//! See also: [`docs/kernel/architecture.md`] section 5.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use alloc::collections::VecDeque;

use crate::{
    event,
    scheme::SchemeId,
    sync::{CleanLockToken, LockToken, Mutex, MutexGuard, L0, L1},
    syscall::{
        data::TimeSpec,
        flag::{CLOCK_MONOTONIC, CLOCK_REALTIME, EVENT_READ},
    },
    time,
};

/// One registered deadline: when `time` (on `clock`) passes, fire `event_id`
/// on `scheme_id`.
#[derive(Debug)]
struct Timeout {
    pub scheme_id: SchemeId,
    pub event_id: usize,
    pub clock: usize,
    pub time: u128,
}

type Registry = VecDeque<Timeout>;

static REGISTRY: Mutex<L1, Registry> = Mutex::new(Registry::new());

/// Get the global timeouts list
fn registry(token: LockToken<'_, L0>) -> MutexGuard<'_, L1, Registry> {
    REGISTRY.lock(token)
}

pub fn get_timeout_stat(token: &mut CleanLockToken) -> usize {
    REGISTRY.lock(token.token()).len()
}

pub fn register(
    scheme_id: SchemeId,
    event_id: usize,
    clock: usize,
    time: TimeSpec,
    token: &mut CleanLockToken,
) {
    let mut registry = registry(token.token());
    registry.push_back(Timeout {
        scheme_id,
        event_id,
        clock,
        time: (time.tv_sec as u128 * time::NANOS_PER_SEC) + (time.tv_nsec as u128),
    });
}

pub fn trigger(token: &mut CleanLockToken) {
    let mono = time::monotonic(token);
    let real = time::realtime(token);

    let mut i = 0;
    loop {
        let mut registry = registry(token.token());
        let timeout = if i < registry.len() {
            let trigger = match registry[i].clock {
                CLOCK_MONOTONIC => {
                    let time = registry[i].time;
                    mono >= time
                }
                CLOCK_REALTIME => {
                    let time = registry[i].time;
                    real >= time
                }
                clock => {
                    println!("timeout::trigger: unknown clock {}", clock);
                    true
                }
            };

            if trigger {
                registry.swap_remove_back(i).unwrap()
            } else {
                i += 1;
                continue;
            }
        } else {
            break;
        };
        drop(registry);
        event::trigger(timeout.scheme_id, timeout.event_id, EVENT_READ, token);
    }
}
