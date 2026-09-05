#![no_std]
#![no_main]

use lerux_interface_types::{
    display_test_pixel, DisplayRequest, DisplayResponse, InputEvent, DISPLAY_BYTES_PER_PIXEL,
    DISPLAY_HEIGHT, DISPLAY_STRIDE, DISPLAY_WIDTH,
};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, var, Channel, Handler, Infallible};

/// Channel 0: serial-virt (`<end pd="display_demo" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: display-server (`<end pd="display_demo" id="1" pp="true" />`).
const DISPLAY_SERVER: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();

    let bitmap = *var!(bitmap_vaddr: usize = 0) as *mut u8;
    assert!(!bitmap.is_null());

    let mode = call::<DisplayRequest, DisplayResponse>(DISPLAY_SERVER, DisplayRequest::GetMode)
        .expect("GetMode");
    match mode {
        DisplayResponse::Mode { width, height, .. } => {
            log::info!("lerux-display: mode {width}x{height}")
        }
        _ => panic!("GetMode"),
    }

    fill_pattern(bitmap);

    match call::<DisplayRequest, DisplayResponse>(DISPLAY_SERVER, DisplayRequest::Present)
        .expect("Present")
    {
        DisplayResponse::Ok => log::info!("lerux-display: pattern ok"),
        _ => panic!("Present"),
    }

    match call::<DisplayRequest, DisplayResponse>(DISPLAY_SERVER, DisplayRequest::PollInput)
        .expect("PollInput")
    {
        DisplayResponse::Input(InputEvent::None) => log::info!("lerux-display: input idle"),
        DisplayResponse::Input(InputEvent::Key { code, .. }) => {
            log::info!("lerux-display: key {code}");
            log::info!("lerux-display: input idle");
        }
        _ => panic!("PollInput"),
    }

    HandlerImpl
}

fn fill_pattern(bitmap: *mut u8) {
    for y in 0..DISPLAY_HEIGHT {
        for x in 0..DISPLAY_WIDTH {
            let px = display_test_pixel(x, y);
            let off = (y * DISPLAY_STRIDE + x * DISPLAY_BYTES_PER_PIXEL) as usize;
            // SAFETY: `bitmap` is the 2 MiB shared MR; `off` is 4-byte aligned
            // and inside the visible 800×600 region.
            unsafe {
                bitmap.add(off).cast::<u32>().write(px);
            }
        }
    }
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}
