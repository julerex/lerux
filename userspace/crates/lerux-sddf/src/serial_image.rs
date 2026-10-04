//! Configuration pages for the one-client serial image.
//!
//! The system template maps the queue and data regions at these virtual
//! addresses. [`driver_config`], [`virt_tx_config`], [`virt_rx_config`], and
//! [`client_config`] write those addresses and the Microkit channel ids into
//! the structs from `include/sddf/serial/config.h`. Each protection domain's
//! build script embeds those bytes. The installed Microkit kit cannot prefill
//! a memory region, so the domain copies the bytes itself and reads the queues
//! from the mapped pages.

use crate::serial::{
    region_resource_t, serial_client_config_t, serial_connection_resource_t,
    serial_driver_config_t, serial_virt_rx_config_t, serial_virt_tx_client_config_t,
    serial_virt_tx_config_t, SDDF_NAME_LENGTH, SDDF_SERIAL_MAGIC,
};

/// Mapped size of one queue region and one data region.
///
/// The data-region size is the queue capacity passed to `serial_queue_init`.
pub const SERIAL_REGION_SIZE: u64 = 0x1000;

/// Virtual address of the serial device registers. Only the driver maps them.
pub const UART_VADDR: u64 = 0x2_000_000;

pub const TX_DRIVER_QUEUE_VADDR: u64 = 0x3_000_000;
pub const TX_DRIVER_DATA_VADDR: u64 = 0x3_001_000;
pub const RX_DRIVER_QUEUE_VADDR: u64 = 0x3_002_000;
pub const RX_DRIVER_DATA_VADDR: u64 = 0x3_003_000;
pub const TX_CLIENT_QUEUE_VADDR: u64 = 0x3_004_000;
pub const TX_CLIENT_DATA_VADDR: u64 = 0x3_005_000;
pub const RX_CLIENT_QUEUE_VADDR: u64 = 0x3_006_000;
pub const RX_CLIENT_DATA_VADDR: u64 = 0x3_007_000;

/// Microkit channel ids. Each value is the id on that protection domain.
///
/// The device interrupt is not a field of `serial_driver_config_t`. The system
/// template's `<irq id>` uses [`DRIVER_IRQ_CHANNEL`].
pub const DRIVER_IRQ_CHANNEL: u8 = 0;
pub const DRIVER_TX_CHANNEL: u8 = 1;
pub const DRIVER_RX_CHANNEL: u8 = 2;
pub const VIRT_TX_DRIVER_CHANNEL: u8 = 0;
pub const VIRT_TX_CLIENT_CHANNEL: u8 = 1;
pub const VIRT_RX_DRIVER_CHANNEL: u8 = 0;
pub const VIRT_RX_CLIENT_CHANNEL: u8 = 1;
pub const CLIENT_TX_CHANNEL: u8 = 0;
pub const CLIENT_RX_CHANNEL: u8 = 1;

pub const DEFAULT_BAUD: u64 = 115_200;

/// `serial_virt_rx` switches the active client when it reads this byte.
pub const SWITCH_CHAR: u8 = 0x1c;

/// Ends the decimal client number that follows [`SWITCH_CHAR`].
pub const TERMINATE_NUM_CHAR: u8 = b'\r';

const CLIENT_NAME: &str = "serial_client";

fn region(vaddr: u64, size: u64) -> region_resource_t {
    region_resource_t {
        vaddr: vaddr as *mut u8,
        size,
    }
}

/// The serial config structs are plain data. Zero is false for every `bool`,
/// null for every pointer, and the zero byte for every integer and array.
fn zeroed_config<T>() -> T {
    unsafe { core::mem::zeroed() }
}

fn connection(queue: u64, data: u64, id: u8) -> serial_connection_resource_t {
    // A struct literal leaves the padding after `id` uninitialised. Start from
    // zeros and assign fields so two calls write the same bytes.
    let mut connection: serial_connection_resource_t = zeroed_config();
    connection.queue = region(queue, SERIAL_REGION_SIZE);
    connection.data = region(data, SERIAL_REGION_SIZE);
    connection.id = id;
    connection
}

fn write_name(dst: &mut [u8], text: &str) {
    let bytes = text.as_bytes();
    assert!(
        bytes.len() < dst.len(),
        "name fits in the fixed field with a terminating zero"
    );
    dst[..bytes.len()].copy_from_slice(bytes);
}

/// `serial_driver_config_t` for the one-client image.
pub fn driver_config() -> serial_driver_config_t {
    let mut config: serial_driver_config_t = zeroed_config();
    config.magic = SDDF_SERIAL_MAGIC;
    config.rx = connection(
        RX_DRIVER_QUEUE_VADDR,
        RX_DRIVER_DATA_VADDR,
        DRIVER_RX_CHANNEL,
    );
    config.tx = connection(
        TX_DRIVER_QUEUE_VADDR,
        TX_DRIVER_DATA_VADDR,
        DRIVER_TX_CHANNEL,
    );
    config.default_baud = DEFAULT_BAUD;
    config.rx_enabled = true;
    config
}

/// `serial_virt_tx_config_t` for the one-client image.
///
/// Colour is off. The colour transfer in the header is not part of this image.
/// `enable_rx` is off, so this virtualiser does not write `begin_str`. The
/// client writes the smoke string itself.
pub fn virt_tx_config() -> serial_virt_tx_config_t {
    let mut config: serial_virt_tx_config_t = zeroed_config();
    config.magic = SDDF_SERIAL_MAGIC;
    config.driver = connection(
        TX_DRIVER_QUEUE_VADDR,
        TX_DRIVER_DATA_VADDR,
        VIRT_TX_DRIVER_CHANNEL,
    );
    let mut client: serial_virt_tx_client_config_t = zeroed_config();
    client.conn = connection(
        TX_CLIENT_QUEUE_VADDR,
        TX_CLIENT_DATA_VADDR,
        VIRT_TX_CLIENT_CHANNEL,
    );
    write_name(&mut client.name, CLIENT_NAME);
    config.clients[0] = client;
    config.num_clients = 1;
    config.enable_colour = false;
    config.enable_rx = false;
    config
}

/// `serial_virt_rx_config_t` for the one-client image.
pub fn virt_rx_config() -> serial_virt_rx_config_t {
    let mut config: serial_virt_rx_config_t = zeroed_config();
    config.magic = SDDF_SERIAL_MAGIC;
    config.driver = connection(
        RX_DRIVER_QUEUE_VADDR,
        RX_DRIVER_DATA_VADDR,
        VIRT_RX_DRIVER_CHANNEL,
    );
    config.clients[0] = connection(
        RX_CLIENT_QUEUE_VADDR,
        RX_CLIENT_DATA_VADDR,
        VIRT_RX_CLIENT_CHANNEL,
    );
    config.num_clients = 1;
    config.switch_char = SWITCH_CHAR;
    config.terminate_num_char = TERMINATE_NUM_CHAR;
    config
}

/// `serial_client_config_t` for the one client.
pub fn client_config() -> serial_client_config_t {
    let mut config: serial_client_config_t = zeroed_config();
    config.magic = SDDF_SERIAL_MAGIC;
    config.rx = connection(
        RX_CLIENT_QUEUE_VADDR,
        RX_CLIENT_DATA_VADDR,
        CLIENT_RX_CHANNEL,
    );
    config.tx = connection(
        TX_CLIENT_QUEUE_VADDR,
        TX_CLIENT_DATA_VADDR,
        CLIENT_TX_CHANNEL,
    );
    config
}

/// Copy a configuration struct into `dst`.
///
/// `dst` must be exactly `size_of::<T>()` bytes. Padding bytes stay zero when
/// `value` was built by the functions in this module.
pub fn serial_config_to_bytes<T>(value: &T, dst: &mut [u8]) {
    assert_eq!(dst.len(), core::mem::size_of::<T>());
    // SAFETY: `dst` is exactly one `T`, and the source is an initialized `T`.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(value).cast::<u8>(),
            dst.as_mut_ptr(),
            dst.len(),
        );
    }
}

/// Read a configuration struct out of a byte page.
///
/// # Safety
///
/// `bytes` must be a valid representation of `T`. Each `bool` field must be 0 or 1.
pub unsafe fn serial_config_from_bytes<T>(bytes: &[u8]) -> T {
    assert_eq!(bytes.len(), core::mem::size_of::<T>());
    let mut value = core::mem::MaybeUninit::<T>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            value.as_mut_ptr().cast::<u8>(),
            bytes.len(),
        );
        value.assume_init()
    }
}

/// Client name stored in [`virt_tx_config`], as the fixed field without the trailing zeros.
pub fn client_name_bytes() -> &'static [u8] {
    CLIENT_NAME.as_bytes()
}

/// The fixed name field is `SDDF_NAME_LENGTH` bytes.
pub const fn client_name_field_len() -> usize {
    SDDF_NAME_LENGTH
}
