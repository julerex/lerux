//! Configuration pages for the one-client filesystem image.
//!
//! Command and completion rings are two separate `fs_queue_t` regions.
//! Each protection domain's build script embeds the bytes.

use crate::{
    fs::{
        fs_client_config_t, fs_connection_resource_t, fs_server_config_t, FS_QUEUE_CAPACITY,
        LIONS_FS_MAGIC,
    },
    region_resource_t,
};

/// Filesystem regions follow the block regions at `0x4_000_000`.
pub const FS_COMMAND_QUEUE_VADDR: u64 = 0x4_020_000;
pub const FS_COMPLETION_QUEUE_VADDR: u64 = 0x4_028_000;
pub const FS_SHARE_VADDR: u64 = 0x4_030_000;
pub const FS_REGION_SIZE: u64 = 0x8000;

pub const FS_QUEUE_LEN: u16 = FS_QUEUE_CAPACITY as u16;

/// Channel ids. Each value is the id on that protection domain.
pub const FS_SERVER_CLIENT_CHANNEL: u8 = 1;
pub const FS_CLIENT_SERVER_CHANNEL: u8 = 0;
/// The shell also owns the serial client channels 0 and 1.
pub const FS_SHELL_SERVER_CHANNEL: u8 = 2;

fn region(vaddr: u64) -> region_resource_t {
    region_resource_t {
        vaddr: vaddr as *mut u8,
        size: FS_REGION_SIZE,
    }
}

fn zeroed_config<T>() -> T {
    // A struct literal leaves padding uninitialised.
    unsafe { core::mem::zeroed() }
}

fn connection(id: u8) -> fs_connection_resource_t {
    let mut connection: fs_connection_resource_t = zeroed_config();
    connection.command_queue = region(FS_COMMAND_QUEUE_VADDR);
    connection.completion_queue = region(FS_COMPLETION_QUEUE_VADDR);
    connection.share = region(FS_SHARE_VADDR);
    connection.queue_len = FS_QUEUE_LEN;
    connection.id = id;
    connection
}

/// `fs_server_config_t` for the one client.
pub fn server_config() -> fs_server_config_t {
    let mut config: fs_server_config_t = zeroed_config();
    config.magic = LIONS_FS_MAGIC;
    config.client = connection(FS_SERVER_CLIENT_CHANNEL);
    config
}

/// `fs_client_config_t` for the one client.
pub fn client_config() -> fs_client_config_t {
    let mut config: fs_client_config_t = zeroed_config();
    config.magic = LIONS_FS_MAGIC;
    config.server = connection(FS_CLIENT_SERVER_CHANNEL);
    config
}

/// `fs_client_config_t` for the shell, whose serial channels already use 0 and 1.
pub fn shell_client_config() -> fs_client_config_t {
    let mut config: fs_client_config_t = zeroed_config();
    config.magic = LIONS_FS_MAGIC;
    config.server = connection(FS_SHELL_SERVER_CHANNEL);
    config
}

/// Copy a configuration struct into `dst`.
pub fn fs_config_to_bytes<T>(value: &T, dst: &mut [u8]) {
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

/// Read a configuration struct out of bytes that came from [`fs_config_to_bytes`].
///
/// # Safety
///
/// `bytes` must be a valid representation of `T`. Each `bool` field must be 0 or 1.
pub unsafe fn fs_config_from_bytes<T>(bytes: &[u8]) -> T {
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
