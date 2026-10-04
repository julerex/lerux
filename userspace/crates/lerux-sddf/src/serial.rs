//! Serial queue metadata and configuration pages.
//!
//! Field order matches `include/sddf/serial/queue.h` and `include/sddf/serial/config.h`.

use core::sync::atomic::AtomicU32;

/// `SDDF_NAME_LENGTH`, which Microkit 2.3.0 defines as `MICROKIT_PD_NAME_LENGTH`.
pub const SDDF_NAME_LENGTH: usize = 64;

pub const SDDF_SERIAL_MAX_CLIENTS: usize = 64;
pub const SDDF_SERIAL_BEGIN_STR_MAX_LEN: usize = 128;
pub const SDDF_SERIAL_MAGIC_LEN: usize = 5;

/// Header magic `sDDF` followed by `0x03`.
pub const SDDF_SERIAL_MAGIC: [u8; SDDF_SERIAL_MAGIC_LEN] = [b's', b'D', b'D', b'F', 0x03];

/// Shared serial queue indexes. The producer owns `tail`. The consumer owns `head`.
#[repr(C)]
pub struct serial_queue_t {
    pub tail: AtomicU32,
    pub head: AtomicU32,
    pub producer_signalled: AtomicU32,
}

/// Local pointer to a shared serial queue. Not itself shared memory.
#[repr(C)]
pub struct serial_queue_handle_t {
    pub queue: *mut serial_queue_t,
    pub capacity: u32,
    pub data_region: *mut u8,
}

/// `region_resource_t` from `include/sddf/resources/common.h`.
#[repr(C)]
pub struct region_resource_t {
    pub vaddr: *mut u8,
    pub size: u64,
}

#[repr(C)]
pub struct serial_connection_resource_t {
    pub queue: region_resource_t,
    pub data: region_resource_t,
    pub id: u8,
}

#[repr(C)]
pub struct serial_driver_config_t {
    pub magic: [u8; SDDF_SERIAL_MAGIC_LEN],
    pub rx: serial_connection_resource_t,
    pub tx: serial_connection_resource_t,
    pub default_baud: u64,
    pub rx_enabled: bool,
}

#[repr(C)]
pub struct serial_virt_rx_config_t {
    pub magic: [u8; SDDF_SERIAL_MAGIC_LEN],
    pub driver: serial_connection_resource_t,
    pub clients: [serial_connection_resource_t; SDDF_SERIAL_MAX_CLIENTS],
    pub num_clients: u8,
    pub switch_char: u8,
    pub terminate_num_char: u8,
}

#[repr(C)]
pub struct serial_virt_tx_client_config_t {
    pub conn: serial_connection_resource_t,
    pub name: [u8; SDDF_NAME_LENGTH],
}

#[repr(C)]
pub struct serial_virt_tx_config_t {
    pub magic: [u8; SDDF_SERIAL_MAGIC_LEN],
    pub driver: serial_connection_resource_t,
    pub clients: [serial_virt_tx_client_config_t; SDDF_SERIAL_MAX_CLIENTS],
    pub num_clients: u8,
    pub begin_str: [u8; SDDF_SERIAL_BEGIN_STR_MAX_LEN],
    pub enable_colour: bool,
    pub enable_rx: bool,
}

#[repr(C)]
pub struct serial_client_config_t {
    pub magic: [u8; SDDF_SERIAL_MAGIC_LEN],
    pub rx: serial_connection_resource_t,
    pub tx: serial_connection_resource_t,
}

/// True when the configuration page starts with [`SDDF_SERIAL_MAGIC`].
pub fn serial_config_check_magic(config: &[u8]) -> bool {
    config.len() >= SDDF_SERIAL_MAGIC_LEN && config[..SDDF_SERIAL_MAGIC_LEN] == SDDF_SERIAL_MAGIC
}
