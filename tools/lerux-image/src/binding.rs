//! Configuration bytes for one protection domain.
//!
//! The protection-domain build script and the host writer both call
//! [`emit_pd_binding`]. The page list lives here.

use std::{mem::size_of, path::Path};

use lerux_sddf::{blk_image, fs_image, net_image, sched, serial_image, timer};

use crate::model::ImageError;

#[derive(Clone, Copy)]
pub(crate) enum Page {
    SerialDriver,
    SerialVirtTx,
    SerialVirtRx,
    SerialClient,
    BlkDriver,
    BlkVirt,
    BlkClient,
    FsServer,
    FsClient,
    FsShell,
    NetDriver,
    NetVirtTx,
    NetVirtRx,
    NetCopy,
    NetClient,
    NetShell,
    NetRequest,
    TimerShell,
    FsSchedule,
}

/// Configuration bytes for `crate_name` under `board_features`.
///
/// # Errors
///
/// Returns [`ImageError::UnknownCrate`] when `crate_name` has no page list.
/// Returns [`ImageError::ConflictingFeatures`] when `sddf-shell` enables a
/// filesystem board feature and `net-client` together.
pub fn pd_binding_bytes(crate_name: &str, board_features: &[&str]) -> Result<Vec<u8>, ImageError> {
    let mut bytes = Vec::new();
    for page in pages_for(crate_name, board_features)? {
        append_page(&mut bytes, page);
    }
    Ok(bytes)
}

/// Write [`pd_binding_bytes`] to `config_path`.
///
/// # Errors
///
/// Returns the errors of [`pd_binding_bytes`]. Returns [`ImageError::WriteConfig`]
/// when the file cannot be created.
pub fn emit_pd_binding(
    crate_name: &str,
    board_features: &[&str],
    config_path: &Path,
) -> Result<(), ImageError> {
    let bytes = pd_binding_bytes(crate_name, board_features)?;
    std::fs::write(config_path, bytes).map_err(|error| ImageError::WriteConfig {
        path: config_path.display().to_string(),
        detail: error.to_string(),
    })
}

#[cfg(test)]
pub(crate) fn bytes_for_pages(pages: &[Page]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for page in pages {
        append_page(&mut bytes, *page);
    }
    bytes
}

fn pages_for(crate_name: &str, board_features: &[&str]) -> Result<Vec<Page>, ImageError> {
    let pages = match crate_name {
        "sddf-serial-driver" => vec![Page::SerialDriver],
        "sddf-serial-virt-tx" => vec![Page::SerialVirtTx],
        "sddf-serial-virt-rx" => vec![Page::SerialVirtRx],
        "sddf-serial-client" | "sddf-grok" | "sddf-browser-ui" => vec![Page::SerialClient],
        "sddf-blk-driver" | "sddf-ahci-driver" => vec![Page::BlkDriver],
        "sddf-blk-virt" => vec![Page::BlkVirt],
        "sddf-fatfs" => vec![Page::BlkClient, Page::FsServer],
        "sddf-fs-client" => vec![Page::FsClient],
        "sddf-net-driver" | "sddf-e1000e-driver" => vec![Page::NetDriver],
        "sddf-net-virt-tx" => vec![Page::NetVirtTx],
        "sddf-net-virt-rx" => vec![Page::NetVirtRx],
        "sddf-net-copy" => vec![Page::NetCopy],
        "sddf-net-client" => vec![Page::SerialClient, Page::NetClient],
        "sddf-agent" => vec![Page::FsShell, Page::NetShell],
        "sddf-request-server" => vec![Page::NetRequest],
        "sddf-program" => vec![Page::SerialClient, Page::FsShell, Page::NetShell],
        "sddf-shell" => shell_pages(board_features)?,
        _ => {
            return Err(ImageError::UnknownCrate {
                name: crate_name.to_string(),
            });
        }
    };
    Ok(pages)
}

fn shell_pages(board_features: &[&str]) -> Result<Vec<Page>, ImageError> {
    let filesystem = board_features.contains(&"board-qemu_virt_aarch64_fs_sddf")
        || board_features.contains(&"board-qemu_virt_riscv64_fs_sddf")
        || board_features.contains(&"board-x86_64_generic_fs_sddf");
    let network = board_features.contains(&"net-client");
    if filesystem && network {
        return Err(ImageError::ConflictingFeatures {
            crate_name: "sddf-shell".to_string(),
        });
    }
    let mut pages = vec![Page::SerialClient, Page::FsShell];
    if filesystem {
        pages.push(Page::TimerShell);
        pages.push(Page::FsSchedule);
    } else if network {
        pages.push(Page::NetShell);
    }
    Ok(pages)
}

fn append_page(bytes: &mut Vec<u8>, page: Page) {
    match page {
        Page::SerialDriver => append_serial(bytes, &serial_image::driver_config()),
        Page::SerialVirtTx => append_serial(bytes, &serial_image::virt_tx_config()),
        Page::SerialVirtRx => append_serial(bytes, &serial_image::virt_rx_config()),
        Page::SerialClient => append_serial(bytes, &serial_image::client_config()),
        Page::BlkDriver => append_blk(bytes, &blk_image::driver_config()),
        Page::BlkVirt => append_blk(bytes, &blk_image::virt_config()),
        Page::BlkClient => append_blk(bytes, &blk_image::client_config()),
        Page::FsServer => append_fs(bytes, &fs_image::server_config()),
        Page::FsClient => append_fs(bytes, &fs_image::client_config()),
        Page::FsShell => append_fs(bytes, &fs_image::shell_client_config()),
        Page::NetDriver => append_net(bytes, &net_image::driver_config()),
        Page::NetVirtTx => append_net(bytes, &net_image::virt_tx_config()),
        Page::NetVirtRx => append_net(bytes, &net_image::virt_rx_config()),
        Page::NetCopy => append_net(bytes, &net_image::copy_config()),
        Page::NetClient => append_net(bytes, &net_image::client_config()),
        Page::NetShell => append_net(bytes, &net_image::shell_client_config()),
        Page::NetRequest => append_net(bytes, &request_net_config()),
        Page::TimerShell => append_timer(bytes, &timer::shell_client_config()),
        Page::FsSchedule => append_schedule(bytes),
    }
}

fn append_serial<T>(bytes: &mut Vec<u8>, value: &T) {
    push(bytes, value, serial_image::serial_config_to_bytes);
}

fn append_blk<T>(bytes: &mut Vec<u8>, value: &T) {
    push(bytes, value, blk_image::blk_config_to_bytes);
}

fn append_fs<T>(bytes: &mut Vec<u8>, value: &T) {
    push(bytes, value, fs_image::fs_config_to_bytes);
}

fn request_net_config() -> lerux_sddf::net_client_config_t {
    net_image::client_config_for(
        lerux_ladybird_http::REQUEST_NET_RX_CHANNEL,
        lerux_ladybird_http::REQUEST_NET_TX_CHANNEL,
    )
}

fn append_net<T>(bytes: &mut Vec<u8>, value: &T) {
    push(bytes, value, net_image::net_config_to_bytes);
}

fn append_timer<T>(bytes: &mut Vec<u8>, value: &T) {
    push(bytes, value, timer::timer_config_to_bytes);
}

fn append_schedule(bytes: &mut Vec<u8>) {
    let value = sched::fs_image_schedule();
    let start = bytes.len();
    bytes.resize(start + size_of::<sched::sched_config_t>(), 0);
    sched::sched_to_bytes(&value, &mut bytes[start..]);
}

fn push<T>(bytes: &mut Vec<u8>, value: &T, encode: fn(&T, &mut [u8])) {
    let start = bytes.len();
    bytes.resize(start + size_of::<T>(), 0);
    encode(value, &mut bytes[start..]);
}
