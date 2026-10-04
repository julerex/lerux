#![no_std]
#![no_main]

use core::sync::atomic::Ordering;

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_enqueue_local,
    serial_handle_from_connection, serial_image, serial_queue_full, serial_queue_handle_t,
    serial_require_consumer_signal, serial_update_shared_tail, serial_virt_rx_config_t,
    SDDF_SERIAL_MAGIC, SDDF_SERIAL_MAX_CLIENTS,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const MAX_CLI_BASE_10: usize = 4;

fn empty_handle() -> serial_queue_handle_t {
    serial_queue_handle_t {
        queue: core::ptr::null_mut(),
        capacity: 0,
        data_region: core::ptr::null_mut(),
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Normal,
    Switched,
    Number,
}

struct HandlerImpl {
    driver: serial_queue_handle_t,
    driver_ch: Channel,
    clients: [serial_queue_handle_t; SDDF_SERIAL_MAX_CLIENTS],
    client_ch: [Channel; SDDF_SERIAL_MAX_CLIENTS],
    num_clients: usize,
    mode: Mode,
    current: usize,
    digits: [u8; MAX_CLI_BASE_10],
    digit_len: usize,
    switch_char: u8,
    terminate: u8,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `serial_virt_rx_config_t`.
    let config =
        unsafe { serial_image::serial_config_from_bytes::<serial_virt_rx_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC);
    let num_clients = usize::from(config.num_clients);
    assert!(num_clients >= 1, "receive virtualiser has a client");
    assert!(
        num_clients <= SDDF_SERIAL_MAX_CLIENTS,
        "client count fits the config array"
    );

    // SAFETY: the template maps the driver and client receive regions here.
    let driver = unsafe { serial_handle_from_connection(&config.driver) };
    let mut clients = core::array::from_fn(|_| empty_handle());
    let mut client_ch = [Channel::new(0); SDDF_SERIAL_MAX_CLIENTS];
    for i in 0..num_clients {
        clients[i] = unsafe { serial_handle_from_connection(&config.clients[i]) };
        client_ch[i] = Channel::new(usize::from(config.clients[i].id));
    }

    log::info!("serial_virt_rx: ready");
    HandlerImpl {
        driver,
        driver_ch: Channel::new(usize::from(config.driver.id)),
        clients,
        client_ch,
        num_clients,
        mode: Mode::Normal,
        current: 0,
        digits: [0; MAX_CLI_BASE_10],
        digit_len: 0,
        switch_char: config.switch_char,
        terminate: config.terminate_num_char,
    }
}

fn parse_client_index(digits: &[u8]) -> Option<usize> {
    if digits.is_empty() {
        return None;
    }
    let mut value: usize = 0;
    for &digit in digits {
        if !digit.is_ascii_digit() {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add(usize::from(digit - b'0'))?;
    }
    Some(value)
}

/// The receive virtualiser reads the producer tail the same way `virt_rx.c` does.
fn driver_queue_full(handle: &serial_queue_handle_t) -> bool {
    unsafe {
        let tail = (*handle.queue).tail.load(Ordering::Relaxed);
        serial_queue_full(handle, tail)
    }
}

impl HandlerImpl {
    fn reset(&mut self) {
        self.digits = [0; MAX_CLI_BASE_10];
        self.digit_len = 0;
        self.mode = Mode::Normal;
    }

    fn enqueue_current(&mut self, byte: u8, local_tail: &mut u32, transferred: &mut bool) {
        let current = self.current;
        // SAFETY: the current client handle points at that client's mapped receive queue.
        let status = unsafe { serial_enqueue_local(&self.clients[current], local_tail, byte) };
        if status == 0 {
            *transferred = true;
        }
    }

    fn push_digit(&mut self, byte: u8) {
        self.digits[self.digit_len] = byte;
        self.digit_len += 1;
    }

    fn accept_normal(&mut self, byte: u8, local_tail: &mut u32, transferred: &mut bool) {
        if byte == self.switch_char {
            self.mode = Mode::Switched;
            return;
        }
        self.enqueue_current(byte, local_tail, transferred);
    }

    fn accept_switched(&mut self, byte: u8, local_tail: &mut u32, transferred: &mut bool) {
        if byte.is_ascii_digit() {
            self.push_digit(byte);
            self.mode = Mode::Number;
            return;
        }
        if byte == self.switch_char {
            self.enqueue_current(byte, local_tail, transferred);
        }
        self.reset();
    }

    fn commit_number(&mut self, local_tail: &mut u32, transferred: &mut bool) {
        let Some(next) = parse_client_index(&self.digits[..self.digit_len]) else {
            self.reset();
            return;
        };
        if next < self.num_clients {
            if *transferred {
                let current = self.current;
                // SAFETY: the current client handle points at that client's mapped queue.
                unsafe { serial_update_shared_tail(&self.clients[current], *local_tail) };
                self.client_ch[current].notify();
            }
            self.current = next;
            // SAFETY: `next` names an initialised client queue.
            *local_tail = unsafe { (*self.clients[next].queue).tail.load(Ordering::Relaxed) };
            *transferred = false;
        }
        self.reset();
    }

    fn accept_number(&mut self, byte: u8, local_tail: &mut u32, transferred: &mut bool) {
        if byte == self.terminate {
            self.commit_number(local_tail, transferred);
            return;
        }
        if self.digit_len < MAX_CLI_BASE_10 && byte.is_ascii_digit() {
            self.push_digit(byte);
            return;
        }
        self.reset();
    }

    fn accept_byte(&mut self, byte: u8, local_tail: &mut u32, transferred: &mut bool) {
        match self.mode {
            Mode::Normal => self.accept_normal(byte, local_tail, transferred),
            Mode::Switched => self.accept_switched(byte, local_tail, transferred),
            Mode::Number => self.accept_number(byte, local_tail, transferred),
        }
    }

    fn rx_return(&mut self) {
        let mut transferred = false;
        let current = self.current;
        // SAFETY: the current client handle points at that client's mapped receive queue.
        let mut local_tail = unsafe { (*self.clients[current].queue).tail.load(Ordering::Relaxed) };
        let mut byte = 0;
        // SAFETY: `driver` points at the mapped driver receive queue. This domain consumes it.
        while unsafe { serial_dequeue(&self.driver, &mut byte) } == 0 {
            self.accept_byte(byte, &mut local_tail, &mut transferred);
        }
        let current = self.current;
        unsafe { serial_update_shared_tail(&self.clients[current], local_tail) };
        if !driver_queue_full(&self.driver)
            && unsafe { serial_require_consumer_signal(&self.driver) }
        {
            unsafe { serial_cancel_consumer_signal(&self.driver) };
            self.driver_ch.notify();
        }
        if transferred {
            self.client_ch[self.current].notify();
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.driver_ch) {
            self.rx_return();
        }
        Ok(())
    }
}
