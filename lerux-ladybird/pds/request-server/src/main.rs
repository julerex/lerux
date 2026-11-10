//! Request server for `qemu_virt_aarch64_ladybird`.
//!
//! `web_content` writes one URL into the shared request page and waits.
//! This domain performs one GET and writes the body into the shared region.
//! It owns the network client queues. It does not own serial or graphics.

#![no_std]
#![no_main]

extern crate alloc;

mod net;
mod net_device;

use core::{
    cell::UnsafeCell,
    mem::size_of,
    sync::atomic::{fence, Ordering},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_ladybird_http::{
    Load, RequestPage, BODY_CAP, BODY_VADDR, REQUEST_NET_RX_CHANNEL, REQUEST_NET_TX_CHANNEL,
    REQUEST_SERVER_CHANNEL, REQUEST_STATE_ASK, REQUEST_STATE_ERR, REQUEST_STATE_OK, REQUEST_VADDR,
};
use lerux_sddf::{net_client_config_t, net_config_check_magic, net_image, SDDF_NET_MAGIC};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const STACK_SIZE: usize = 0x4000;
const REQUEST_CH: usize = REQUEST_SERVER_CHANNEL as usize;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

struct HandlerImpl;

#[protection_domain(heap_size = 1024 * 1024)]
fn init() -> HandlerImpl {
    assert_eq!(CONFIG.len(), size_of::<net_client_config_t>());
    // SAFETY: the build script wrote one `net_client_config_t`. It has no bool field.
    let net = unsafe { net_image::net_config_from_bytes::<net_client_config_t>(CONFIG) };
    assert_eq!(net.magic, SDDF_NET_MAGIC);
    assert!(net_config_check_magic(CONFIG));
    assert_eq!(net.rx.id, REQUEST_NET_RX_CHANNEL);
    assert_eq!(net.tx.id, REQUEST_NET_TX_CHANNEL);
    net::install(&net);
    // SAFETY: `STACKS` is exclusive to this init and the spawned worker.
    let stacks = unsafe { &mut *STACKS.0.get() };
    let ptrs = core::array::from_fn(|index| stacks[index].0.as_mut_ptr());
    // SAFETY: the static stacks outlive the protection domain and do not overlap.
    unsafe { microkit_cothread_init(STACK_SIZE, &ptrs) };
    let handle = microkit_cothread_spawn(worker, core::ptr::null_mut());
    assert_ne!(handle, NULL_HANDLE, "worker cothread slot is free");
    microkit_cothread_yield();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(Channel::new(REQUEST_CH)) {
            microkit_cothread_recv_ntfn(REQUEST_CH);
        }
        net::on_notify(channels);
        Ok(())
    }
}

extern "C" fn worker() {
    loop {
        microkit_cothread_wait_on_channel(REQUEST_CH);
        let ok = serve();
        finish(ok);
    }
}

fn serve() -> bool {
    let page = request_page();
    fence(Ordering::Acquire);
    if page.state != REQUEST_STATE_ASK {
        return false;
    }
    let url = page.url().to_vec();
    let Ok(text) = core::str::from_utf8(&url) else {
        return false;
    };
    let Ok(load) = Load::parse(text) else {
        return false;
    };
    let mut request = [0u8; 192];
    let Ok(len) = load.write_get(&mut request) else {
        return false;
    };
    match net::fetch(&request[..len], load.port(), body_region()) {
        Ok(body) => page.accept_body(body),
        Err(()) => false,
    }
}

fn finish(ok: bool) {
    let page = request_page();
    if !ok {
        page.clear_body();
    }
    fence(Ordering::Release);
    page.state = if ok {
        REQUEST_STATE_OK
    } else {
        REQUEST_STATE_ERR
    };
    Channel::new(REQUEST_CH).notify();
}

fn request_page() -> &'static mut RequestPage {
    // SAFETY: the system description maps the request page at `REQUEST_VADDR`.
    // Microkit zeroes it. This domain and `web_content` take turns writing it.
    unsafe { &mut *(REQUEST_VADDR as *mut RequestPage) }
}

fn body_region() -> &'static mut [u8] {
    // SAFETY: the system description maps `BODY_CAP` bytes at `BODY_VADDR`.
    unsafe { core::slice::from_raw_parts_mut(BODY_VADDR as *mut u8, BODY_CAP) }
}
