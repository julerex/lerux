#![no_std]
#![no_main]

use core::cell::UnsafeCell;

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

// Channel 0: serial-driver (<end pd="cothread_client" id="0" pp="true" />).
const SERIAL_DRIVER: Channel = Channel::new(0);
// Channel 1: cothread_peer (<end pd="cothread_client" id="1" />).
const PEER_CH: usize = 1;
const PEER: Channel = Channel::new(PEER_CH);

/// Room for the log call, which is a protected procedure call into the serial driver.
const STACK_SIZE: usize = 0x4000;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

struct HandlerImpl;

extern "C" fn worker() {
    log::info!("cothread waiting");
    PEER.notify();
    microkit_cothread_wait_on_channel(PEER_CH);
    log::info!("cothread resumed");
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).expect("serial logger");
    // SAFETY: `STACKS` is exclusive to this init and the spawned worker.
    let stacks = unsafe { &mut *STACKS.0.get() };
    let ptrs = core::array::from_fn(|index| stacks[index].0.as_mut_ptr());
    // SAFETY: the static stacks outlive the protection domain and do not overlap.
    unsafe { microkit_cothread_init(STACK_SIZE, &ptrs) };
    let handle = microkit_cothread_spawn(worker, core::ptr::null_mut());
    assert_ne!(handle, NULL_HANDLE, "worker cothread slot is free");
    // Runs the worker until it blocks, then returns here so the event loop can wait.
    microkit_cothread_yield();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(PEER) {
            microkit_cothread_recv_ntfn(PEER_CH);
        }
        Ok(())
    }
}
