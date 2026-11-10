//! Browser UI for `qemu_virt_aarch64_ladybird`.
//!
//! The worker waits for `web_content` to paint, prints the title, copies the
//! shared bitmap through the graphics client queues, and prints the sampled
//! pixel. This domain does not parse HTML.

#![no_std]
#![no_main]

mod gpu;

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{fence, Ordering},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_ladybird_gfx::{
    Bitmap, ControlPage, BITMAP_VADDR, BROWSER_UI_GPU_CHANNEL, BROWSER_UI_PAINT_CHANNEL,
    CONTROL_STATE_PAINTED, CONTROL_VADDR, FRAME_BYTES, TITLE_CAP,
};
use lerux_sddf::{
    serial_client_config_t, serial_enqueue, serial_handle_from_connection, serial_image,
    serial_queue_handle_t, serial_request_consumer_signal, SDDF_SERIAL_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const STACK_SIZE: usize = 0x4000;
const TX_CH: usize = serial_image::CLIENT_TX_CHANNEL as usize;
const GPU_CH: usize = BROWSER_UI_GPU_CHANNEL as usize;
const PAINT_CH: usize = BROWSER_UI_PAINT_CHANNEL as usize;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

struct TxCell(UnsafeCell<MaybeUninit<serial_queue_handle_t>>);

// SAFETY: init writes this cell before the worker starts. The worker is the only caller afterwards.
unsafe impl Sync for TxCell {}

static TX: TxCell = TxCell(UnsafeCell::new(MaybeUninit::uninit()));

struct Title {
    bytes: [u8; TITLE_CAP],
    len: usize,
}

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    // SAFETY: the build script wrote a `serial_client_config_t`.
    let config =
        unsafe { serial_image::serial_config_from_bytes::<serial_client_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC, "serial client magic");
    assert_eq!(
        config.tx.id,
        serial_image::CLIENT_TX_CHANNEL,
        "serial transmit id"
    );
    assert_eq!(
        config.rx.id,
        serial_image::CLIENT_RX_CHANNEL,
        "serial receive id"
    );
    install_serial(&config);
    gpu::install();
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
        recv(channels, TX_CH);
        recv(channels, GPU_CH);
        recv(channels, PAINT_CH);
        Ok(())
    }
}

fn recv(channels: ChannelSet, index: usize) {
    if channels.contains(Channel::new(index)) {
        microkit_cothread_recv_ntfn(index);
    }
}

fn install_serial(config: &serial_client_config_t) {
    // SAFETY: the system description maps this client's transmit queue.
    let tx = unsafe { serial_handle_from_connection(&config.tx) };
    unsafe { (*TX.0.get()).write(tx) };
}

extern "C" fn worker() {
    microkit_cothread_wait_on_channel(PAINT_CH);
    fence(Ordering::Acquire);
    let Some(title) = painted_title() else {
        return;
    };
    say_title(&title);
    if gpu::present().is_err() {
        return;
    }
    say_pixel();
}

fn painted_title() -> Option<Title> {
    let page = control_page();
    if page.state != CONTROL_STATE_PAINTED {
        return None;
    }
    let src = page.title();
    let mut bytes = [0u8; TITLE_CAP];
    bytes[..src.len()].copy_from_slice(src);
    Some(Title {
        bytes,
        len: src.len(),
    })
}

fn say_title(title: &Title) {
    let mut line = [0u8; TITLE_CAP + 8];
    let prefix = b"title ";
    line[..prefix.len()].copy_from_slice(prefix);
    let start = prefix.len();
    line[start..start + title.len].copy_from_slice(&title.bytes[..title.len]);
    let end = start + title.len;
    line[end] = b'\n';
    emit(&line[..=end]);
}

fn say_pixel() {
    let Some(bitmap) = Bitmap::frame(shared_bitmap()) else {
        return;
    };
    let Some(line) = bitmap.pixel_line() else {
        return;
    };
    emit(line.as_bytes());
}

fn shared_bitmap() -> &'static mut [u8] {
    // SAFETY: the system description maps `FRAME_BYTES` at `BITMAP_VADDR` for this domain.
    unsafe { core::slice::from_raw_parts_mut(BITMAP_VADDR as *mut u8, FRAME_BYTES) }
}

fn control_page() -> &'static mut ControlPage {
    // SAFETY: the system description maps the control page at `CONTROL_VADDR`.
    // `web_content` is the only writer. This domain reads it after the paint signal.
    unsafe { &mut *(CONTROL_VADDR as *mut ControlPage) }
}

fn tx() -> &'static serial_queue_handle_t {
    // SAFETY: `install_serial` wrote this cell before the worker was spawned.
    unsafe { (*TX.0.get()).assume_init_ref() }
}

fn emit(bytes: &[u8]) {
    let queue = tx();
    for &byte in bytes {
        // SAFETY: `queue` is this client's mapped transmit queue. This worker produces it.
        while unsafe { serial_enqueue(queue, byte) } != 0 {
            unsafe { serial_request_consumer_signal(queue) };
            Channel::new(TX_CH).notify();
            microkit_cothread_wait_on_channel(TX_CH);
        }
    }
    Channel::new(TX_CH).notify();
}
