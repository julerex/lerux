#![no_std]
#![no_main]

extern crate alloc;

use lerux_interface_types::{
    DisplayRequest, DisplayResponse, DISPLAY_HEIGHT, DISPLAY_STRIDE, DISPLAY_VISIBLE_BYTES,
    DISPLAY_WIDTH,
};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use lerux_web::{fixture_signatures_ok, render, Bitmap, PAINT_FIXTURE};
use sel4_microkit::{protection_domain, var, Channel, Handler, Infallible};

/// Channel 0: serial-virt (`<end pd="paint_demo" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: display-server (`<end pd="paint_demo" id="1" pp="true" />`).
const DISPLAY_SERVER: Channel = Channel::new(1);

#[protection_domain(heap_size = 128 * 1024)]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();

    let bitmap = *var!(bitmap_vaddr: usize = 0) as *mut u8;
    assert!(!bitmap.is_null());

    // SAFETY: `bitmap` is the 2 MiB shared MR; we only write the visible prefix.
    let slice = unsafe { core::slice::from_raw_parts_mut(bitmap, DISPLAY_VISIBLE_BYTES) };
    let mut bmp =
        Bitmap::new(slice, DISPLAY_WIDTH, DISPLAY_HEIGHT, DISPLAY_STRIDE).expect("bitmap mr");
    render(PAINT_FIXTURE, &mut bmp);

    match call::<DisplayRequest, DisplayResponse>(DISPLAY_SERVER, DisplayRequest::Present)
        .expect("Present")
    {
        DisplayResponse::Ok if fixture_signatures_ok(&bmp) => {
            log::info!("lerux-web: paint ok")
        }
        DisplayResponse::Ok => log::info!("lerux-web: paint mismatch"),
        _ => panic!("Present"),
    }

    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}
