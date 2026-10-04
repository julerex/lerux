//! Configuration pages for the one-client block image.
//!
//! The system template maps these regions at the virtual addresses below.
//! Each protection domain's build script embeds the bytes. The domain copies
//! them into an aligned value. `include_bytes!` has alignment 1.

use crate::{
    blk::{
        blk_client_config_t, blk_connection_resource_t, blk_driver_config_t,
        blk_virt_config_client_t, blk_virt_config_driver_t, blk_virt_config_t, SDDF_BLK_MAGIC,
    },
    device_region_resource_t, region_resource_t,
};

pub const BLK_PAGE_SIZE: u64 = 0x1_000;

pub const BLK_STORAGE_INFO_VADDR: u64 = 0x3_000_000;
pub const BLK_STORAGE_INFO_SIZE: u64 = 0x1_000;
pub const BLK_DRIVER_REQ_QUEUE_VADDR: u64 = 0x3_001_000;
pub const BLK_DRIVER_RESP_QUEUE_VADDR: u64 = 0x3_002_000;
pub const BLK_CLIENT_REQ_QUEUE_VADDR: u64 = 0x3_003_000;
pub const BLK_CLIENT_RESP_QUEUE_VADDR: u64 = 0x3_004_000;
pub const BLK_DATA_VADDR: u64 = 0x3_010_000;
pub const BLK_DATA_SIZE: u64 = 0x10_000;

pub const BLK_VIRTIO_MMIO_VADDR: u64 = 0x6_000_000_000;
pub const BLK_VIRTIO_DMA_VADDR: u64 = 0x8_000_000_000;
pub const BLK_VIRTIO_DMA_SIZE: u64 = 0x200_000;

/// Queue capacity. One page holds the queue prefix plus this many entries.
pub const BLK_QUEUE_CAPACITY: u16 = 16;

/// Channel ids. Each value is the id on that protection domain.
pub const BLK_DRIVER_IRQ_CHANNEL: u8 = 0;
pub const BLK_DRIVER_VIRT_CHANNEL: u8 = 1;
pub const BLK_VIRT_DRIVER_CHANNEL: u8 = 0;
pub const BLK_VIRT_CLIENT_CHANNEL: u8 = 1;
pub const BLK_CLIENT_VIRT_CHANNEL: u8 = 0;

pub const BLK_VIRT_NUM_CLIENTS: u64 = 1;
pub const BLK_CLIENT_PARTITION: u32 = 0;

fn region(vaddr: u64, size: u64) -> region_resource_t {
    region_resource_t {
        vaddr: vaddr as *mut u8,
        size,
    }
}

fn zeroed_config<T>() -> T {
    // A struct literal leaves padding uninitialised.
    unsafe { core::mem::zeroed() }
}

fn connection(req: u64, resp: u64, id: u8) -> blk_connection_resource_t {
    let mut connection: blk_connection_resource_t = zeroed_config();
    connection.storage_info = region(BLK_STORAGE_INFO_VADDR, BLK_STORAGE_INFO_SIZE);
    connection.req_queue = region(req, BLK_PAGE_SIZE);
    connection.resp_queue = region(resp, BLK_PAGE_SIZE);
    connection.num_buffers = BLK_QUEUE_CAPACITY;
    connection.id = id;
    connection
}

fn device_data() -> device_region_resource_t {
    let mut data: device_region_resource_t = zeroed_config();
    data.region = region(BLK_DATA_VADDR, BLK_DATA_SIZE);
    data.io_addr = 0;
    data
}

/// `blk_driver_config_t` for the one-client image.
pub fn driver_config() -> blk_driver_config_t {
    let mut config: blk_driver_config_t = zeroed_config();
    config.magic = SDDF_BLK_MAGIC;
    config.virt = connection(
        BLK_DRIVER_REQ_QUEUE_VADDR,
        BLK_DRIVER_RESP_QUEUE_VADDR,
        BLK_DRIVER_VIRT_CHANNEL,
    );
    config
}

/// `blk_virt_config_t` for one client on partition 0.
pub fn virt_config() -> blk_virt_config_t {
    let mut config: blk_virt_config_t = zeroed_config();
    config.magic = SDDF_BLK_MAGIC;
    config.num_clients = BLK_VIRT_NUM_CLIENTS;
    let mut driver: blk_virt_config_driver_t = zeroed_config();
    driver.conn = connection(
        BLK_DRIVER_REQ_QUEUE_VADDR,
        BLK_DRIVER_RESP_QUEUE_VADDR,
        BLK_VIRT_DRIVER_CHANNEL,
    );
    driver.data = device_data();
    config.driver = driver;
    let mut client: blk_virt_config_client_t = zeroed_config();
    client.conn = connection(
        BLK_CLIENT_REQ_QUEUE_VADDR,
        BLK_CLIENT_RESP_QUEUE_VADDR,
        BLK_VIRT_CLIENT_CHANNEL,
    );
    client.data = device_data();
    client.partition = BLK_CLIENT_PARTITION;
    config.clients[0] = client;
    config
}

/// `blk_client_config_t` for the filesystem server.
pub fn client_config() -> blk_client_config_t {
    let mut config: blk_client_config_t = zeroed_config();
    config.magic = SDDF_BLK_MAGIC;
    config.virt = connection(
        BLK_CLIENT_REQ_QUEUE_VADDR,
        BLK_CLIENT_RESP_QUEUE_VADDR,
        BLK_CLIENT_VIRT_CHANNEL,
    );
    config.data = region(BLK_DATA_VADDR, BLK_DATA_SIZE);
    config
}

/// Copy a configuration struct into `dst`.
pub fn blk_config_to_bytes<T>(value: &T, dst: &mut [u8]) {
    assert_eq!(dst.len(), core::mem::size_of::<T>());
    // SAFETY: `dst` is exactly one `T`, and the source is an initialised `T`.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(value).cast::<u8>(),
            dst.as_mut_ptr(),
            dst.len(),
        );
    }
}

/// Read a configuration struct out of bytes that came from [`blk_config_to_bytes`].
///
/// # Safety
///
/// `bytes` must be a valid representation of `T`. Each `bool` field must be 0 or 1.
pub unsafe fn blk_config_from_bytes<T>(bytes: &[u8]) -> T {
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

/// Copy configuration bytes into `dst` without returning the value.
///
/// # Safety
///
/// `dst` must be valid for a `T` and suitably aligned. `bytes` must be a valid
/// representation of `T`. Each `bool` field must be 0 or 1.
pub unsafe fn blk_config_write_bytes<T>(bytes: &[u8], dst: *mut T) {
    assert_eq!(bytes.len(), core::mem::size_of::<T>());
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst.cast::<u8>(), bytes.len());
    }
}
