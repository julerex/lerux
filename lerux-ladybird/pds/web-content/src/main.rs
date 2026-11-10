//! Web content for `qemu_virt_aarch64_ladybird`.
//!
//! The worker parses the baked fixture, paints the shared bitmap, writes the
//! title into the control page, and notifies `browser_ui`. This domain maps
//! neither a network queue nor the graphics device.

#![no_std]
#![no_main]

extern crate alloc;

use core::{
    cell::UnsafeCell,
    sync::atomic::{fence, Ordering},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_spawn, microkit_cothread_yield, NULL_HANDLE,
    STACK_SLOTS,
};
use lerux_ladybird_gfx::{
    Bitmap, ControlPage, BITMAP_VADDR, CONTROL_STATE_PAINTED, CONTROL_VADDR, FRAME_BYTES,
    WEB_CONTENT_PAINT_CHANNEL,
};
use lerux_ladybird_html::Document;
use lerux_ladybird_web::{StyleSheet, StyledDocument};

const FIXTURE: &str = include_str!("../../../../support/browser/fixture.html");
const STACK_SIZE: usize = 0x4000;
const PAINT_CH: usize = WEB_CONTENT_PAINT_CHANNEL as usize;

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

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        Ok(())
    }
}

extern "C" fn worker() {
    let document = Document::parse(FIXTURE);
    let Ok(sheet) = StyleSheet::parse("") else {
        return;
    };
    if paint_fixture(&document, &sheet) {
        publish(document.title_text().as_bytes());
    }
}

fn paint_fixture(document: &Document, sheet: &StyleSheet) -> bool {
    let styled = StyledDocument::new(document, sheet);
    let layout = styled.layout();
    let tree = layout.paintables();
    let Some(mut bitmap) = Bitmap::frame(shared_bitmap()) else {
        return false;
    };
    tree.paint(&mut bitmap);
    true
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
