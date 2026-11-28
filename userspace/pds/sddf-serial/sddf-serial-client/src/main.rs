#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_sddf::{
    serial_client_config_t, serial_dequeue, serial_enqueue, serial_handle_from_connection,
    serial_image, serial_queue_handle_t, SDDF_SERIAL_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const READY: &[u8] = b"lerux shell ready\n";

struct HandlerImpl {
    rx: serial_queue_handle_t,
    rx_ch: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    // SAFETY: the build script wrote a `serial_client_config_t`.
    let config =
        unsafe { serial_image::serial_config_from_bytes::<serial_client_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC);
    // SAFETY: the template maps this client's transmit and receive regions here.
    let tx = unsafe { serial_handle_from_connection(&config.tx) };
    let rx = unsafe { serial_handle_from_connection(&config.rx) };
    for &byte in READY {
        // SAFETY: `tx` points at this client's mapped transmit queue.
        let status = unsafe { serial_enqueue(&tx, byte) };
        assert_eq!(status, 0, "ready line fits in the client transmit queue");
    }
    Channel::new(usize::from(config.tx.id)).notify();
    HandlerImpl {
        rx,
        rx_ch: Channel::new(usize::from(config.rx.id)),
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.rx_ch) {
            let mut byte = 0;
            // SAFETY: `rx` points at this client's mapped receive queue.
            while unsafe { serial_dequeue(&self.rx, &mut byte) } == 0 {}
        }
        Ok(())
    }
}
