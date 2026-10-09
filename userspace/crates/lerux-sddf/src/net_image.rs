//! Configuration pages for the one-client network image.
//!
//! The system template maps the queues and data regions at these virtual
//! addresses. The constructors write those addresses and the Microkit channel
//! ids into the structs from `include/sddf/network/config.h`. Each protection
//! domain's build script embeds those bytes. The installed Microkit kit cannot
//! prefill a memory region, so the domain copies them at start.
//!
//! Network queues start at `0x5_000_000`. Serial queues stay at `0x3_000_000`
//! and block regions stay at `0x4_000_000`.
//!
//! `device_region_resource_t.io_addr` stays 0. Microkit 2.2.0 assigns RAM
//! physical addresses at load time. Descriptors between the virtualisers and
//! the driver carry byte offsets, and the virtio driver translates those
//! offsets through the regions it maps.

use crate::{
    device_region_resource_t, mac_addr_t, net_client_config_t, net_connection_resource_t,
    net_copy_config_t, net_driver_config_t, net_virt_rx_client_config_t, net_virt_rx_config_t,
    net_virt_tx_config_t, net_virt_tx_data_region_t, region_resource_t, SDDF_NET_MAGIC,
};

/// Queue capacity. The header's full check is `tail - head == capacity`.
/// Sixteen descriptors fit in [`NET_QUEUE_REGION_SIZE`] after the queue prefix.
pub const NET_QUEUE_CAPACITY: u16 = 16;

/// Bytes in one data region: capacity times the 2048-byte buffer.
pub const NET_DATA_SIZE: u64 = 0x8000;

/// Mapped size of one queue region. The descriptor array starts after the fixed prefix.
pub const NET_QUEUE_REGION_SIZE: u64 = 0x1000;

/// Virtio ring direct-memory-access region. Packet bytes are copied here.
pub const NET_DRIVER_DMA_SIZE: u64 = 0x200_000;

/// QEMU `virt` on AArch64 places virtio-net at `+0xe00` in the page at
/// `0xa003000`, interrupt 79. Virtio-blk, when an image also maps it, occupies
/// `+0xc00` and ends at this offset. The interrupt is not a field of
/// `net_driver_config_t`. The system template binds it to [`DRIVER_IRQ_CHANNEL`].
pub const NET_VIRTIO_MMIO_OFFSET: usize = 0xe00;
pub const NET_VIRTIO_MMIO_SIZE: usize = 0x200;

/// Same virtio window as the block image. This image does not map the block regions.
pub const NET_VIRTIO_MMIO_VADDR: u64 = 0x6_000_000_000;
pub const NET_DRIVER_DMA_VADDR: u64 = 0x8_000_000_000;

pub const NET_RX_DRV_FREE_VADDR: u64 = 0x5_000_000;
pub const NET_RX_DRV_ACTIVE_VADDR: u64 = 0x5_001_000;
pub const NET_TX_DRV_FREE_VADDR: u64 = 0x5_002_000;
pub const NET_TX_DRV_ACTIVE_VADDR: u64 = 0x5_003_000;
pub const NET_RX_COPY_FREE_VADDR: u64 = 0x5_004_000;
pub const NET_RX_COPY_ACTIVE_VADDR: u64 = 0x5_005_000;
pub const NET_RX_CLIENT_FREE_VADDR: u64 = 0x5_006_000;
pub const NET_RX_CLIENT_ACTIVE_VADDR: u64 = 0x5_007_000;
pub const NET_TX_CLIENT_FREE_VADDR: u64 = 0x5_008_000;
pub const NET_TX_CLIENT_ACTIVE_VADDR: u64 = 0x5_009_000;
pub const NET_RX_DATA_VADDR: u64 = 0x5_010_000;
pub const NET_TX_DATA_VADDR: u64 = 0x5_020_000;
pub const NET_RX_CLIENT_DATA_VADDR: u64 = 0x5_030_000;
pub const NET_RX_META_VADDR: u64 = 0x5_040_000;

/// Channel on `net_driver` for the virtio interrupt. It is not a config field.
pub const DRIVER_IRQ_CHANNEL: u8 = 0;
pub const DRIVER_RX_CHANNEL: u8 = 1;
pub const DRIVER_TX_CHANNEL: u8 = 2;
pub const VIRT_RX_DRIVER_CHANNEL: u8 = 0;
pub const VIRT_RX_COPY_CHANNEL: u8 = 1;
pub const VIRT_TX_DRIVER_CHANNEL: u8 = 0;
pub const VIRT_TX_CLIENT_CHANNEL: u8 = 1;
pub const COPY_VIRT_CHANNEL: u8 = 0;
pub const COPY_CLIENT_CHANNEL: u8 = 1;
// The client keeps serial transmit on channel 0 and serial receive on channel 1.
pub const CLIENT_NET_RX_CHANNEL: u8 = 2;
pub const CLIENT_NET_TX_CHANNEL: u8 = 3;

/// QEMU's default virtio-net address for the first device.
pub const CLIENT_MAC: mac_addr_t = mac_addr_t {
    addr: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
};

fn zeroed_config<T>() -> T {
    // A struct literal leaves padding uninitialised.
    unsafe { core::mem::zeroed() }
}

fn region(vaddr: u64, size: u64) -> region_resource_t {
    region_resource_t {
        vaddr: vaddr as *mut u8,
        size,
    }
}

fn connection(free: u64, active: u64, id: u8) -> net_connection_resource_t {
    // A struct literal leaves the padding after `id` uninitialised.
    let mut conn: net_connection_resource_t = zeroed_config();
    conn.free_queue = region(free, NET_QUEUE_REGION_SIZE);
    conn.active_queue = region(active, NET_QUEUE_REGION_SIZE);
    conn.num_buffers = NET_QUEUE_CAPACITY;
    conn.id = id;
    conn
}

fn tx_region(vaddr: u64) -> net_virt_tx_data_region_t {
    let mut region_desc: net_virt_tx_data_region_t = zeroed_config();
    region_desc.data = device_region_resource_t {
        region: region(vaddr, NET_DATA_SIZE),
        io_addr: 0,
    };
    region_desc.num_buffers = u32::from(NET_QUEUE_CAPACITY);
    region_desc
}

/// `net_driver_config_t` for the one-client image.
pub fn driver_config() -> net_driver_config_t {
    let mut config: net_driver_config_t = zeroed_config();
    config.magic = SDDF_NET_MAGIC;
    config.virt_rx = connection(
        NET_RX_DRV_FREE_VADDR,
        NET_RX_DRV_ACTIVE_VADDR,
        DRIVER_RX_CHANNEL,
    );
    config.virt_tx = connection(
        NET_TX_DRV_FREE_VADDR,
        NET_TX_DRV_ACTIVE_VADDR,
        DRIVER_TX_CHANNEL,
    );
    config
}

/// `net_virt_tx_config_t` for the one-client image.
///
/// The header fixes the client array at 64, so this value is larger than a
/// protection-domain stack (`0x10_000`). The build script calls this function.
/// The domain copies the embedded bytes into static storage and does not call it.
pub fn virt_tx_config() -> net_virt_tx_config_t {
    let mut config: net_virt_tx_config_t = zeroed_config();
    config.magic = SDDF_NET_MAGIC;
    config.driver = connection(
        NET_TX_DRV_FREE_VADDR,
        NET_TX_DRV_ACTIVE_VADDR,
        VIRT_TX_DRIVER_CHANNEL,
    );
    config.num_clients = 1;
    config.clients[0].conn = connection(
        NET_TX_CLIENT_FREE_VADDR,
        NET_TX_CLIENT_ACTIVE_VADDR,
        VIRT_TX_CLIENT_CHANNEL,
    );
    config.clients[0].regions[0] = tx_region(NET_TX_DATA_VADDR);
    config.clients[0].num_regions = 1;
    config
}

/// `net_virt_rx_config_t` for the one-client image.
pub fn virt_rx_config() -> net_virt_rx_config_t {
    let mut config: net_virt_rx_config_t = zeroed_config();
    config.magic = SDDF_NET_MAGIC;
    config.driver = connection(
        NET_RX_DRV_FREE_VADDR,
        NET_RX_DRV_ACTIVE_VADDR,
        VIRT_RX_DRIVER_CHANNEL,
    );
    config.data = device_region_resource_t {
        region: region(NET_RX_DATA_VADDR, NET_DATA_SIZE),
        io_addr: 0,
    };
    config.buffer_metadata = region(NET_RX_META_VADDR, NET_QUEUE_REGION_SIZE);
    config.num_clients = 1;
    config.clients[0] = rx_client();
    config
}

fn rx_client() -> net_virt_rx_client_config_t {
    let mut client: net_virt_rx_client_config_t = zeroed_config();
    client.conn = connection(
        NET_RX_COPY_FREE_VADDR,
        NET_RX_COPY_ACTIVE_VADDR,
        VIRT_RX_COPY_CHANNEL,
    );
    client.mac_addrs[0] = CLIENT_MAC;
    client.num_macs = 1;
    client
}

/// `net_copy_config_t` for the one-client image.
pub fn copy_config() -> net_copy_config_t {
    let mut config: net_copy_config_t = zeroed_config();
    config.magic = SDDF_NET_MAGIC;
    config.rx = connection(
        NET_RX_COPY_FREE_VADDR,
        NET_RX_COPY_ACTIVE_VADDR,
        COPY_VIRT_CHANNEL,
    );
    config.rx_data[0] = region(NET_RX_DATA_VADDR, NET_DATA_SIZE);
    config.client = connection(
        NET_RX_CLIENT_FREE_VADDR,
        NET_RX_CLIENT_ACTIVE_VADDR,
        COPY_CLIENT_CHANNEL,
    );
    config.client_data = region(NET_RX_CLIENT_DATA_VADDR, NET_DATA_SIZE);
    config
}

/// `net_client_config_t` for the one-client image.
pub fn client_config() -> net_client_config_t {
    let mut config: net_client_config_t = zeroed_config();
    config.magic = SDDF_NET_MAGIC;
    config.rx = connection(
        NET_RX_CLIENT_FREE_VADDR,
        NET_RX_CLIENT_ACTIVE_VADDR,
        CLIENT_NET_RX_CHANNEL,
    );
    config.rx_data = region(NET_RX_CLIENT_DATA_VADDR, NET_DATA_SIZE);
    config.tx = connection(
        NET_TX_CLIENT_FREE_VADDR,
        NET_TX_CLIENT_ACTIVE_VADDR,
        CLIENT_NET_TX_CHANNEL,
    );
    config.tx_data = region(NET_TX_DATA_VADDR, NET_DATA_SIZE);
    config.mac_addr = CLIENT_MAC;
    config
}

/// Copy a configuration struct into `dst`.
///
/// `dst` must be exactly `size_of::<T>()` bytes. Padding stays zero when
/// `value` was built by the functions in this module.
pub fn net_config_to_bytes<T>(value: &T, dst: &mut [u8]) {
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
/// `bytes` must be a valid representation of `T`. These configuration structs
/// contain no `bool` fields.
pub unsafe fn net_config_from_bytes<T>(bytes: &[u8]) -> T {
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
