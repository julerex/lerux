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
    TimerShell,
    FsSchedule,
}

pub fn pd_binding_bytes(crate_name: &str, board_features: &[&str]) -> Result<Vec<u8>, ImageError> {
    let mut bytes = Vec::new();
    for page in pages_for(crate_name, board_features)? {
        append_page(&mut bytes, page);
    }
    Ok(bytes)
}

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
        "sddf-serial-client" => vec![Page::SerialClient],
        "sddf-blk-driver" => vec![Page::BlkDriver],
        "sddf-ahci-driver" => vec![Page::BlkDriver],
        "sddf-blk-virt" => vec![Page::BlkVirt],
        "sddf-fatfs" => vec![Page::BlkClient, Page::FsServer],
        "sddf-fs-client" => vec![Page::FsClient],
        "sddf-net-driver" => vec![Page::NetDriver],
        "sddf-net-virt-tx" => vec![Page::NetVirtTx],
        "sddf-net-virt-rx" => vec![Page::NetVirtRx],
        "sddf-net-copy" => vec![Page::NetCopy],
        "sddf-net-client" => vec![Page::SerialClient, Page::NetClient],
        "sddf-grok" => vec![Page::SerialClient],
        "sddf-agent" => vec![Page::FsShell, Page::NetShell],
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
        Page::SerialDriver => push(
            bytes,
            &serial_image::driver_config(),
            serial_image::serial_config_to_bytes,
        ),
        Page::SerialVirtTx => push(
            bytes,
            &serial_image::virt_tx_config(),
            serial_image::serial_config_to_bytes,
        ),
        Page::SerialVirtRx => push(
            bytes,
            &serial_image::virt_rx_config(),
            serial_image::serial_config_to_bytes,
        ),
        Page::SerialClient => push(
            bytes,
            &serial_image::client_config(),
            serial_image::serial_config_to_bytes,
        ),
        Page::BlkDriver => push(
            bytes,
            &blk_image::driver_config(),
            blk_image::blk_config_to_bytes,
        ),
        Page::BlkVirt => push(
            bytes,
            &blk_image::virt_config(),
            blk_image::blk_config_to_bytes,
        ),
        Page::BlkClient => push(
            bytes,
            &blk_image::client_config(),
            blk_image::blk_config_to_bytes,
        ),
        Page::FsServer => push(
            bytes,
            &fs_image::server_config(),
            fs_image::fs_config_to_bytes,
        ),
        Page::FsClient => push(
            bytes,
            &fs_image::client_config(),
            fs_image::fs_config_to_bytes,
        ),
        Page::FsShell => push(
            bytes,
            &fs_image::shell_client_config(),
            fs_image::fs_config_to_bytes,
        ),
        Page::NetDriver => push(
            bytes,
            &net_image::driver_config(),
            net_image::net_config_to_bytes,
        ),
        Page::NetVirtTx => push(
            bytes,
            &net_image::virt_tx_config(),
            net_image::net_config_to_bytes,
        ),
        Page::NetVirtRx => push(
            bytes,
            &net_image::virt_rx_config(),
            net_image::net_config_to_bytes,
        ),
        Page::NetCopy => push(
            bytes,
            &net_image::copy_config(),
            net_image::net_config_to_bytes,
        ),
        Page::NetClient => push(
            bytes,
            &net_image::client_config(),
            net_image::net_config_to_bytes,
        ),
        Page::NetShell => push(
            bytes,
            &net_image::shell_client_config(),
            net_image::net_config_to_bytes,
        ),
        Page::TimerShell => push(
            bytes,
            &timer::shell_client_config(),
            timer::timer_config_to_bytes,
        ),
        Page::FsSchedule => {
            let value = sched::fs_image_schedule();
            let start = bytes.len();
            bytes.resize(start + size_of::<sched::sched_config_t>(), 0);
            sched::sched_to_bytes(&value, &mut bytes[start..]);
        }
    }
}

fn push<T>(bytes: &mut Vec<u8>, value: &T, encode: fn(&T, &mut [u8])) {
    let start = bytes.len();
    bytes.resize(start + size_of::<T>(), 0);
    encode(value, &mut bytes[start..]);
}
