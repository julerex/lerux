//! Web content for `qemu_virt_aarch64_ladybird`.
//!
//! The worker loads the fixture, paints it, then loads the frames document.
//! That document's iframe is a second GET through the same request page.
//! The image document's `img` is another GET. The encoded bytes go to
//! `image_decoder`, which writes the pixels back. This domain maps no
//! network queue and does not decode the PNG.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
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
use lerux_ladybird_image::{
    ImagePage, ENCODED_CAP, ENCODED_VADDR, IMAGE_STATE_ASK, IMAGE_STATE_OK, IMAGE_VADDR, PIXEL_CAP,
    PIXEL_VADDR, WEB_CONTENT_IMAGE_CHANNEL,
};
use lerux_ladybird_url::Url;
use lerux_ladybird_web::{Page, Raster};

const FIXTURE_URL: &str = "https://host:8443/fixture.html";
const FRAMES_URL: &str = "https://host:8443/frames.html";
const IMAGE_URL: &str = "https://host:8443/image.html";
const STACK_SIZE: usize = 0x4000;
const PAINT_CH: usize = WEB_CONTENT_PAINT_CHANNEL as usize;
const REQUEST_CH: usize = WEB_CONTENT_REQUEST_CHANNEL as usize;
const IMAGE_CH: usize = WEB_CONTENT_IMAGE_CHANNEL as usize;

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
        recv(channels, IMAGE_CH);
        Ok(())
    }
}

extern "C" fn worker() {
    let mut page = Page::new();
    if !open(&mut page, FIXTURE_URL) {
        return;
    }
    let _ = open(&mut page, FRAMES_URL);
    let _ = open(&mut page, IMAGE_URL);
}

fn open(page: &mut Page, text: &str) -> bool {
    let Ok(url) = Url::parse(text, None) else {
        return false;
    };
    if !page.navigate_with(url, fetch, load_image) {
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
    let bytes = fetch_bytes(url)?;
    let text = core::str::from_utf8(&bytes).ok()?;
    Some(String::from(text))
}

fn fetch_bytes(url: &Url) -> Option<Vec<u8>> {
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
    Some(body_region()[..page.body_len()].to_vec())
}

fn load_image(url: &Url) -> Option<Raster> {
    let bytes = fetch_bytes(url)?;
    if bytes.len() > ENCODED_CAP {
        return None;
    }
    encoded_region()[..bytes.len()].copy_from_slice(&bytes);
    let page = image_page();
    page.encoded_len = u32::try_from(bytes.len()).ok()?;
    page.width = 0;
    page.height = 0;
    fence(Ordering::Release);
    page.state = IMAGE_STATE_ASK;
    Channel::new(IMAGE_CH).notify();
    microkit_cothread_wait_on_channel(IMAGE_CH);
    fence(Ordering::Acquire);
    if page.state != IMAGE_STATE_OK {
        return None;
    }
    let width = i32::try_from(page.width).ok()?;
    let height = i32::try_from(page.height).ok()?;
    let pixels = width.checked_mul(height)?.checked_mul(4)?;
    let pixels = usize::try_from(pixels).ok()?;
    if pixels > PIXEL_CAP {
        return None;
    }
    Some(Raster {
        width,
        height,
        pixels: pixel_region()[..pixels].to_vec(),
    })
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

fn image_page() -> &'static mut ImagePage {
    // SAFETY: the system description maps the image control page at `IMAGE_VADDR`.
    // Microkit zeroes it. This domain and `image_decoder` take turns writing it.
    unsafe { &mut *(IMAGE_VADDR as *mut ImagePage) }
}

fn encoded_region() -> &'static mut [u8] {
    // SAFETY: the system description maps `ENCODED_CAP` bytes at `ENCODED_VADDR`.
    unsafe { core::slice::from_raw_parts_mut(ENCODED_VADDR as *mut u8, ENCODED_CAP) }
}

fn pixel_region() -> &'static [u8] {
    // SAFETY: the system description maps `PIXEL_CAP` bytes at `PIXEL_VADDR`.
    // `image_decoder` finished writing them before the acquire fence above.
    unsafe { core::slice::from_raw_parts(PIXEL_VADDR as *const u8, PIXEL_CAP) }
}

fn body_region() -> &'static [u8] {
    // SAFETY: the system description maps `BODY_CAP` bytes at `BODY_VADDR`.
    // `request_server` finished writing them before the acquire fence above.
    unsafe { core::slice::from_raw_parts(BODY_VADDR as *const u8, BODY_CAP) }
}
