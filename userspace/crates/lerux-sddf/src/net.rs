//! Network buffer descriptors, queues, and configuration pages.
//!
//! Field order matches `include/sddf/network/queue.h` and
//! `include/sddf/network/config.h`. `oid` is the low 6 bits of its byte, which
//! is the `uint8_t oid : 6` bit-field. Clients store 0.

use core::sync::atomic::{AtomicU16, AtomicU32};

use crate::{device_region_resource_t, region_resource_t};

pub const NET_BUFFER_SIZE: u32 = 2048;

pub const SDDF_NET_MAX_CLIENTS: usize = 64;
pub const SDDF_NET_MAGIC_LEN: usize = 5;

/// Header magic `sDDF` followed by `0x05`.
pub const SDDF_NET_MAGIC: [u8; SDDF_NET_MAGIC_LEN] = [b's', b'D', b'D', b'F', 0x05];

/// `MAC802_BYTES` from `include/sddf/network/mac802.h`.
pub const MAC802_BYTES: usize = 6;

/// `mac_addr_t` from `include/sddf/network/mac802.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mac_addr_t {
    pub addr: [u8; MAC802_BYTES],
}

/// One buffer descriptor. The trailing pad keeps the C size and alignment.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct net_buff_desc_t {
    pub io_or_offset: u64,
    pub len: u16,
    oid: u8,
    _pad: [u8; 5],
}

impl net_buff_desc_t {
    pub const fn new(io_or_offset: u64, len: u16, oid: u8) -> Self {
        Self {
            io_or_offset,
            len,
            oid: oid & 0x3f,
            _pad: [0; 5],
        }
    }

    /// Ownership identifier. Only the low 6 bits are defined.
    pub const fn oid(self) -> u8 {
        self.oid & 0x3f
    }

    /// Store an ownership identifier. Bits above the low 6 are discarded.
    pub const fn set_oid(&mut self, oid: u8) {
        self.oid = oid & 0x3f;
    }
}

/// Fixed prefix of `net_queue_t`. Descriptors follow this prefix in memory.
///
/// The C type ends in a flexible array of `net_buff_desc_t`. That array is not
/// part of the prefix, but it raises the alignment to 8. `tail` and `head` are
/// adjacent `uint16_t` fields. The producer is the only writer of `tail`. The
/// consumer is the only writer of `head`.
#[repr(C, align(8))]
pub struct net_queue_t {
    pub tail: AtomicU16,
    pub head: AtomicU16,
    pub consumer_signalled: AtomicU32,
}

#[repr(C)]
pub struct net_queue_handle_t {
    pub free: *mut net_queue_t,
    pub active: *mut net_queue_t,
    pub capacity: u32,
}

/// `net_connection_resource_t`. One free queue and one active queue.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct net_connection_resource_t {
    pub free_queue: region_resource_t,
    pub active_queue: region_resource_t,
    pub num_buffers: u16,
    pub id: u8,
}

/// `net_driver_config_t`. The device interrupt is not a field.
#[repr(C)]
pub struct net_driver_config_t {
    pub magic: [u8; SDDF_NET_MAGIC_LEN],
    pub virt_rx: net_connection_resource_t,
    pub virt_tx: net_connection_resource_t,
}

/// One transmit data region owned by a client.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct net_virt_tx_data_region_t {
    pub data: device_region_resource_t,
    pub num_buffers: u32,
}

/// `net_virt_tx_client_config_t`.
#[repr(C)]
pub struct net_virt_tx_client_config_t {
    pub conn: net_connection_resource_t,
    pub regions: [net_virt_tx_data_region_t; SDDF_NET_MAX_CLIENTS],
    pub num_regions: u8,
}

/// `net_virt_tx_config_t`. The client array is the header's fixed maximum.
#[repr(C)]
pub struct net_virt_tx_config_t {
    pub magic: [u8; SDDF_NET_MAGIC_LEN],
    pub driver: net_connection_resource_t,
    pub clients: [net_virt_tx_client_config_t; SDDF_NET_MAX_CLIENTS],
    pub num_clients: u8,
}

/// `net_virt_rx_client_config_t`.
#[repr(C)]
pub struct net_virt_rx_client_config_t {
    pub conn: net_connection_resource_t,
    pub mac_addrs: [mac_addr_t; SDDF_NET_MAX_CLIENTS],
    pub num_macs: u8,
}

/// `net_virt_rx_config_t`.
#[repr(C)]
pub struct net_virt_rx_config_t {
    pub magic: [u8; SDDF_NET_MAGIC_LEN],
    pub driver: net_connection_resource_t,
    pub data: device_region_resource_t,
    /// One reference count per receive buffer. The region is read-write and
    /// starts zeroed. A broadcast buffer returns to the driver only after
    /// every client frees it.
    pub buffer_metadata: region_resource_t,
    pub clients: [net_virt_rx_client_config_t; SDDF_NET_MAX_CLIENTS],
    pub num_clients: u8,
}

/// `net_copy_config_t`.
#[repr(C)]
pub struct net_copy_config_t {
    pub magic: [u8; SDDF_NET_MAGIC_LEN],
    pub rx: net_connection_resource_t,
    pub rx_data: [region_resource_t; SDDF_NET_MAX_CLIENTS],
    pub client: net_connection_resource_t,
    pub client_data: region_resource_t,
}

/// `net_client_config_t`.
#[repr(C)]
pub struct net_client_config_t {
    pub magic: [u8; SDDF_NET_MAGIC_LEN],
    pub rx: net_connection_resource_t,
    pub rx_data: region_resource_t,
    pub tx: net_connection_resource_t,
    pub tx_data: region_resource_t,
    pub mac_addr: mac_addr_t,
}

/// `net_vswitch_port_config_t`.
///
/// `mac_addr` is ignored on the virtualiser port.
#[repr(C)]
pub struct net_vswitch_port_config_t {
    pub rx: net_connection_resource_t,
    pub tx: net_connection_resource_t,
    pub tx_data: region_resource_t,
    pub mac_addr: mac_addr_t,
    pub acl: u64,
}

/// `net_vswitch_config_t`.
///
/// Client ports occupy `ports[0 .. num_ports - 1]`. The last port's receive
/// connection is the transmit virtualiser, and its transmit connection is the
/// receive virtualiser. `buffer_metadata` holds one reference count per
/// transmit buffer and per receive direct-memory-access buffer.
#[repr(C)]
pub struct net_vswitch_config_t {
    pub magic: [u8; SDDF_NET_MAGIC_LEN],
    pub ports: [net_vswitch_port_config_t; SDDF_NET_MAX_CLIENTS],
    pub num_ports: u8,
    pub buffer_metadata: region_resource_t,
}

/// True when the configuration page starts with [`SDDF_NET_MAGIC`].
pub fn net_config_check_magic(config: &[u8]) -> bool {
    config.len() >= SDDF_NET_MAGIC_LEN && config[..SDDF_NET_MAGIC_LEN] == SDDF_NET_MAGIC
}
