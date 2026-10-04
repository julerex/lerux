#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_enqueue, serial_handle_from_connection,
    serial_image, serial_queue_free, serial_queue_handle_t, serial_queue_length_consumer,
    serial_request_consumer_signal, serial_require_consumer_signal, serial_virt_tx_config_t,
    SDDF_SERIAL_MAGIC, SDDF_SERIAL_MAX_CLIENTS,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

fn empty_handle() -> serial_queue_handle_t {
    serial_queue_handle_t {
        queue: core::ptr::null_mut(),
        capacity: 0,
        data_region: core::ptr::null_mut(),
    }
}

struct HandlerImpl {
    driver: serial_queue_handle_t,
    driver_ch: Channel,
    clients: [serial_queue_handle_t; SDDF_SERIAL_MAX_CLIENTS],
    client_ch: [Channel; SDDF_SERIAL_MAX_CLIENTS],
    pending: [bool; SDDF_SERIAL_MAX_CLIENTS],
    num_clients: usize,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `serial_virt_tx_config_t`.
    let config =
        unsafe { serial_image::serial_config_from_bytes::<serial_virt_tx_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC);
    assert!(
        !config.enable_colour,
        "colour transfer is not part of this image"
    );
    let num_clients = usize::from(config.num_clients);
    assert!(num_clients >= 1, "transmit virtualiser has a client");
    assert!(
        num_clients <= SDDF_SERIAL_MAX_CLIENTS,
        "client count fits the config array"
    );

    // SAFETY: the template maps the driver and client transmit regions here.
    let driver = unsafe { serial_handle_from_connection(&config.driver) };
    let driver_ch = Channel::new(usize::from(config.driver.id));
    let mut clients = core::array::from_fn(|_| empty_handle());
    let mut client_ch = [Channel::new(0); SDDF_SERIAL_MAX_CLIENTS];
    for i in 0..num_clients {
        clients[i] = unsafe { serial_handle_from_connection(&config.clients[i].conn) };
        client_ch[i] = Channel::new(usize::from(config.clients[i].conn.id));
    }

    if config.enable_rx {
        let len = config
            .begin_str
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(config.begin_str.len());
        for &byte in &config.begin_str[..len] {
            // SAFETY: `driver` points at the mapped driver transmit queue.
            let status = unsafe { serial_enqueue(&driver, byte) };
            assert_eq!(status, 0, "begin string fits in the driver transmit queue");
        }
        if len > 0 {
            driver_ch.notify();
        }
    }

    log::info!("serial_virt_tx: ready");
    HandlerImpl {
        driver,
        driver_ch,
        clients,
        client_ch,
        pending: [false; SDDF_SERIAL_MAX_CLIENTS],
        num_clients,
    }
}

impl HandlerImpl {
    /// Move one client's queued bytes into the driver queue.
    ///
    /// The whole counted length moves, or none of it does. Colour bytes are
    /// not inserted. Returns true when bytes were published to the driver.
    fn move_client(&mut self, client: usize) -> bool {
        // SAFETY: the client and driver handles point at mapped transmit queues.
        // This domain consumes the client queue and produces the driver queue.
        let length = unsafe { serial_queue_length_consumer(&self.clients[client]) };
        if length == 0 {
            return false;
        }
        if length > unsafe { serial_queue_free(&self.driver) } {
            unsafe { serial_request_consumer_signal(&self.driver) };
        }
        if length > unsafe { serial_queue_free(&self.driver) } {
            self.pending[client] = true;
            return false;
        }
        for _ in 0..length {
            let mut byte = 0;
            let got = unsafe { serial_dequeue(&self.clients[client], &mut byte) };
            assert_eq!(got, 0, "counted client byte is present");
            let put = unsafe { serial_enqueue(&self.driver, byte) };
            assert_eq!(put, 0, "counted driver space is present");
        }
        true
    }

    fn wake_client(&mut self, client: usize) {
        // SAFETY: the client handle points at that client's mapped transmit queue.
        if unsafe { serial_require_consumer_signal(&self.clients[client]) } {
            unsafe { serial_cancel_consumer_signal(&self.clients[client]) };
            self.client_ch[client].notify();
        }
    }

    fn provide(&mut self, client: usize) {
        if self.move_client(client) {
            self.driver_ch.notify();
            self.wake_client(client);
        }
    }

    #[expect(
        clippy::needless_range_loop,
        reason = "the index selects several fields, and the body needs &mut self"
    )]
    fn resume_pending(&mut self) {
        let mut moved_any = false;
        let mut moved = [false; SDDF_SERIAL_MAX_CLIENTS];
        for client in 0..self.num_clients {
            if !self.pending[client] {
                continue;
            }
            self.pending[client] = false;
            if self.move_client(client) {
                moved[client] = true;
                moved_any = true;
            }
        }
        if moved_any {
            self.driver_ch.notify();
        }
        for client in 0..self.num_clients {
            if moved[client] {
                self.wake_client(client);
            }
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.driver_ch) {
            self.resume_pending();
        }
        for client in 0..self.num_clients {
            if channels.contains(self.client_ch[client]) {
                self.provide(client);
            }
        }
        Ok(())
    }
}
