//! Image decoder for `qemu_virt_aarch64_ladybird`.
//!
//! `web_content` writes one encoded image and waits. This domain decodes that
//! PNG, writes the pixels, notifies, and waits again before the next image.
//! It maps no network queue and no graphics queue.

#![no_std]
#![no_main]

extern crate alloc;

use core::{
    cell::UnsafeCell,
    sync::atomic::{fence, Ordering},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_ladybird_image::{
    decode_png, ImagePage, ENCODED_CAP, ENCODED_VADDR, IMAGE_DECODER_CHANNEL, IMAGE_STATE_ASK,
    IMAGE_STATE_ERR, IMAGE_STATE_OK, IMAGE_VADDR, PIXEL_CAP, PIXEL_VADDR,
};

const STACK_SIZE: usize = 0x4000;
const IMAGE_CH: usize = IMAGE_DECODER_CHANNEL as usize;

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
        if channels.contains(Channel::new(IMAGE_CH)) {
            microkit_cothread_recv_ntfn(IMAGE_CH);
        }
        Ok(())
    }
}

extern "C" fn worker() {
    loop {
        microkit_cothread_wait_on_channel(IMAGE_CH);
        let ok = decode_one();
        finish(ok);
    }
}

fn decode_one() -> bool {
    let page = image_page();
    fence(Ordering::Acquire);
    if page.state != IMAGE_STATE_ASK {
        return false;
    }
    let Ok(len) = usize::try_from(page.encoded_len) else {
        return false;
    };
    if len > ENCODED_CAP {
        return false;
    }
    let encoded = encoded_region()[..len].to_vec();
    let Ok(image) = decode_png(&encoded) else {
        return false;
    };
    let pixels = image.pixels();
    if pixels.len() > PIXEL_CAP {
        return false;
    }
    pixel_region()[..pixels.len()].copy_from_slice(pixels);
    page.width = image.width();
    page.height = image.height();
    true
}

fn finish(ok: bool) {
    let page = image_page();
    if !ok {
        page.width = 0;
        page.height = 0;
    }
    fence(Ordering::Release);
    page.state = if ok { IMAGE_STATE_OK } else { IMAGE_STATE_ERR };
    Channel::new(IMAGE_CH).notify();
}

fn image_page() -> &'static mut ImagePage {
    // SAFETY: the system description maps the control page at `IMAGE_VADDR`.
    // Microkit zeroes it. This domain and `web_content` take turns writing it.
    unsafe { &mut *(IMAGE_VADDR as *mut ImagePage) }
}

fn encoded_region() -> &'static [u8] {
    // SAFETY: the system description maps `ENCODED_CAP` bytes at `ENCODED_VADDR`.
    // `web_content` finished writing them before the acquire fence above.
    unsafe { core::slice::from_raw_parts(ENCODED_VADDR as *const u8, ENCODED_CAP) }
}

fn pixel_region() -> &'static mut [u8] {
    // SAFETY: the system description maps `PIXEL_CAP` bytes at `PIXEL_VADDR`.
    unsafe { core::slice::from_raw_parts_mut(PIXEL_VADDR as *mut u8, PIXEL_CAP) }
}
