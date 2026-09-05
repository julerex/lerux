#![no_std]
#![no_main]

mod fwcfg;

use lerux_driver_protocols::serial::{
    NonBlocking, Request as SerialRequest, Response as SerialResponse, SuccessResponse,
};
use lerux_interface_types::{
    DisplayRequest, DisplayResponse, InputEvent, DISPLAY_BYTES_PER_PIXEL, DISPLAY_FORMAT_XRGB8888,
    DISPLAY_HEIGHT, DISPLAY_STRIDE, DISPLAY_VISIBLE_BYTES, DISPLAY_WIDTH,
};
use lerux_ipc::{call, recv, send, send_unspecified_error};
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, var, Channel, Handler, Infallible, MessageInfo};

/// Channel 0: serial-virt (`<end pd="display_server" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: display-demo (`<end pd="display_server" id="1" />`).
const CLIENT: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();

    let mmio = *var!(fw_cfg_mmio_vaddr: usize = 0) as *mut u8;
    let dma = *var!(fw_cfg_dma_vaddr: usize = 0) as *mut u8;
    let dma_paddr = *var!(fw_cfg_dma_paddr: usize = 0) as u64;
    let framebuffer = *var!(framebuffer_vaddr: usize = 0) as *mut u8;
    let fb_paddr = *var!(framebuffer_paddr: usize = 0) as u64;
    let bitmap = *var!(bitmap_vaddr: usize = 0) as *mut u8;

    assert!(!mmio.is_null() && !dma.is_null() && !framebuffer.is_null() && !bitmap.is_null());
    fwcfg::configure_ramfb(
        mmio,
        dma,
        dma_paddr,
        fb_paddr,
        fwcfg::RamfbCfg {
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            stride: DISPLAY_STRIDE,
            fourcc: DISPLAY_FORMAT_XRGB8888,
        },
    )
    .expect("ramfb configure");
    log::info!("lerux-display: ramfb ok");

    HandlerImpl {
        bitmap,
        framebuffer,
    }
}

struct HandlerImpl {
    bitmap: *mut u8,
    framebuffer: *mut u8,
}

impl HandlerImpl {
    fn blit(&self) {
        // SAFETY: both MRs are `DISPLAY_FB_MR_BYTES` and mapped RW into this PD;
        // the visible prefix is `DISPLAY_VISIBLE_BYTES`.
        unsafe {
            core::ptr::copy_nonoverlapping(self.bitmap, self.framebuffer, DISPLAY_VISIBLE_BYTES);
        }
    }

    fn poll_key(&self) -> InputEvent {
        match call::<SerialRequest, SerialResponse>(SERIAL_DRIVER, SerialRequest::Read) {
            Ok(Ok(SuccessResponse::Read(NonBlocking::Ready(code)))) => InputEvent::Key {
                code,
                pressed: true,
            },
            _ => InputEvent::None,
        }
    }

    fn handle(&self, req: DisplayRequest) -> DisplayResponse {
        match req {
            DisplayRequest::GetMode => DisplayResponse::Mode {
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                stride: DISPLAY_STRIDE,
                format: DISPLAY_FORMAT_XRGB8888,
            },
            DisplayRequest::Present => {
                self.blit();
                DisplayResponse::Ok
            }
            DisplayRequest::PollInput => DisplayResponse::Input(self.poll_key()),
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT {
            unreachable!();
        }
        Ok(match recv::<DisplayRequest>(msg_info) {
            Ok(req) => send(self.handle(req)),
            Err(_) => send_unspecified_error(),
        })
    }
}

// Pixel size is part of the blit contract; keep it referenced so a bpp change
// fails this crate instead of silently copying the wrong number of bytes.
const _: () = assert!(DISPLAY_BYTES_PER_PIXEL == 4);
