//! Web content for `qemu_virt_aarch64_ladybird`.
//!
//! The worker loads the fixture, paints it, then loads the frames document.
//! That document's iframe is a second GET through the same request page.
//! This domain maps no network queue.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::{String, ToString};
use core::{
    cell::UnsafeCell,
    sync::atomic::{fence, Ordering},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_ladybird_gfx::{
    Bitmap, ControlPage, BITMAP_VADDR, CONTROL_STATE_PAINTED, CONTROL_VADDR, FRAME_BYTES,
    WEB_CONTENT_PAINT_CHANNEL,
};
use lerux_ladybird_http::{
    RequestPage, BODY_CAP, BODY_VADDR, REQUEST_STATE_ASK, REQUEST_STATE_OK, REQUEST_VADDR,
    WEB_CONTENT_REQUEST_CHANNEL,
};
use lerux_ladybird_url::Url;
use lerux_ladybird_web::Page;

const FIXTURE_URL: &str = "https://host:8443/fixture.html";
const FRAMES_URL: &str = "https://host:8443/frames.html";
const STACK_SIZE: usize = 0x4000;
const PAINT_CH: usize = WEB_CONTENT_PAINT_CHANNEL as usize;
const REQUEST_CH: usize = WEB_CONTENT_REQUEST_CHANNEL as usize;

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
        recv(channels, REQUEST_CH);
        recv(channels, PAINT_CH);
        Ok(())
    }
}

extern "C" fn worker() {
    let mut page = Page::new();
    if !open(&mut page, FIXTURE_URL) {
        return;
    }
    let _ = open(&mut page, FRAMES_URL);
}

fn open(page: &mut Page, text: &str) -> bool {
    let Ok(url) = Url::parse(text, None) else {
        return false;
    };
    if !page.navigate(url, fetch) {
        return false;
    }
    let Some(mut bitmap) = Bitmap::frame(shared_bitmap()) else {
        return false;
    };
    page.paint(&mut bitmap);
    publish(page.document().title_text().as_bytes());
    microkit_cothread_wait_on_channel(PAINT_CH);
    true
}

fn recv(channels: ChannelSet, index: usize) {
    if channels.contains(Channel::new(index)) {
        microkit_cothread_recv_ntfn(index);
    }
}

fn fetch(url: &Url) -> Option<String> {
    let text = url.to_string();
    let page = request_page();
    if !page.set_url(text.as_bytes()) {
        return None;
    }
    fence(Ordering::Release);
    page.state = REQUEST_STATE_ASK;
    Channel::new(REQUEST_CH).notify();
    microkit_cothread_wait_on_channel(REQUEST_CH);
    fence(Ordering::Acquire);
    if page.state != REQUEST_STATE_OK {
        return None;
    }
    let body = &body_region()[..page.body_len()];
    let text = core::str::from_utf8(body).ok()?;
    Some(String::from(text))
}

fn publish(title: &[u8]) {
    let page = control_page();
    page.set_title(title);
    fence(Ordering::Release);
    page.state = CONTROL_STATE_PAINTED;
    Channel::new(PAINT_CH).notify();
}

fn shared_bitmap() -> &'static mut [u8] {
    // SAFETY: the system description maps `FRAME_BYTES` at `BITMAP_VADDR` for this domain.
    unsafe { core::slice::from_raw_parts_mut(BITMAP_VADDR as *mut u8, FRAME_BYTES) }
}

fn control_page() -> &'static mut ControlPage {
    // SAFETY: the system description maps the control page at `CONTROL_VADDR`.
    // Microkit zeroes it, and this domain is the only writer.
    unsafe { &mut *(CONTROL_VADDR as *mut ControlPage) }
}

fn request_page() -> &'static mut RequestPage {
    // SAFETY: the system description maps the request page at `REQUEST_VADDR`.
    // Microkit zeroes it. This domain and `request_server` take turns writing it.
    unsafe { &mut *(REQUEST_VADDR as *mut RequestPage) }
}

fn body_region() -> &'static [u8] {
    // SAFETY: the system description maps `BODY_CAP` bytes at `BODY_VADDR`.
    // `request_server` finished writing them before the acquire fence above.
    unsafe { core::slice::from_raw_parts(BODY_VADDR as *const u8, BODY_CAP) }
}
