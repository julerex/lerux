//! Writes the one-client block and filesystem configuration pages next to a board build.
//!
//! The protection domains embed the same bytes from their build scripts. These
//! files are the copy the host checks against the rendered system description.

use std::{fs, mem::size_of, path::Path};

use anyhow::{Context, Result};
use lerux_sddf::{
    blk_image::{self, driver_config, virt_config},
    fs_image::{self, server_config},
};

pub fn write_configs(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    write_blk(dir, "blk_driver_config.bin", &driver_config())?;
    write_blk(dir, "blk_virt_config.bin", &virt_config())?;
    write_blk(dir, "blk_client_config.bin", &blk_image::client_config())?;
    write_fs(dir, "fs_server_config.bin", &server_config())?;
    write_fs(dir, "fs_client_config.bin", &fs_image::client_config())?;
    write_fs(dir, "fs_shell_config.bin", &fs_image::shell_client_config())?;
    Ok(())
}

fn write_blk<T>(dir: &Path, name: &str, value: &T) -> Result<()> {
    write_encoded(dir, name, value, blk_image::blk_config_to_bytes)
}

fn write_fs<T>(dir: &Path, name: &str, value: &T) -> Result<()> {
    write_encoded(dir, name, value, fs_image::fs_config_to_bytes)
}

fn write_encoded<T>(dir: &Path, name: &str, value: &T, encode: fn(&T, &mut [u8])) -> Result<()> {
    let mut bytes = vec![0u8; size_of::<T>()];
    encode(value, &mut bytes);
    let path = dir.join(name);
    fs::write(&path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lerux_sddf::{
        blk_config_check_magic,
        blk_image::{
            self, driver_config, virt_config, BLK_CLIENT_REQ_QUEUE_VADDR,
            BLK_CLIENT_RESP_QUEUE_VADDR, BLK_CLIENT_VIRT_CHANNEL, BLK_DATA_SIZE, BLK_DATA_VADDR,
            BLK_DRIVER_IRQ_CHANNEL, BLK_DRIVER_REQ_QUEUE_VADDR, BLK_DRIVER_RESP_QUEUE_VADDR,
            BLK_DRIVER_VIRT_CHANNEL, BLK_STORAGE_INFO_VADDR, BLK_VIRTIO_DMA_VADDR,
            BLK_VIRTIO_MMIO_VADDR, BLK_VIRT_CLIENT_CHANNEL, BLK_VIRT_DRIVER_CHANNEL,
        },
        fs_config_check_magic,
        fs_image::{
            self, server_config, FS_CLIENT_SERVER_CHANNEL, FS_COMMAND_QUEUE_VADDR,
            FS_COMPLETION_QUEUE_VADDR, FS_REGION_SIZE, FS_SERVER_CLIENT_CHANNEL, FS_SHARE_VADDR,
            FS_SHELL_SERVER_CHANNEL,
        },
        serial_image::{
            CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL, DRIVER_IRQ_CHANNEL, DRIVER_RX_CHANNEL,
            DRIVER_TX_CHANNEL, RX_CLIENT_DATA_VADDR, RX_CLIENT_QUEUE_VADDR, RX_DRIVER_DATA_VADDR,
            RX_DRIVER_QUEUE_VADDR, TX_CLIENT_DATA_VADDR, TX_CLIENT_QUEUE_VADDR,
            TX_DRIVER_DATA_VADDR, TX_DRIVER_QUEUE_VADDR, UART_VADDR, VIRT_RX_CLIENT_CHANNEL,
            VIRT_RX_DRIVER_CHANNEL, VIRT_TX_CLIENT_CHANNEL, VIRT_TX_DRIVER_CHANNEL,
        },
    };

    use super::*;
    use crate::{serial_sddf::hex_group, system::render_system};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn write_configs_starts_with_the_fs_sddf_magic() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_configs(dir.path()).expect("write configs");
        check_blk(dir.path(), "blk_driver_config.bin", &driver_config());
        check_blk(dir.path(), "blk_virt_config.bin", &virt_config());
        check_blk(
            dir.path(),
            "blk_client_config.bin",
            &blk_image::client_config(),
        );
        check_fs(dir.path(), "fs_server_config.bin", &server_config());
        check_fs(
            dir.path(),
            "fs_client_config.bin",
            &fs_image::client_config(),
        );
        check_fs(
            dir.path(),
            "fs_shell_config.bin",
            &fs_image::shell_client_config(),
        );
    }

    fn check_blk<T>(dir: &Path, name: &str, value: &T) {
        let bytes = fs::read(dir.join(name)).expect(name);
        assert!(blk_config_check_magic(&bytes), "{name}");
        let mut expected = vec![0u8; size_of::<T>()];
        blk_image::blk_config_to_bytes(value, &mut expected);
        assert_eq!(bytes, expected, "{name}");
    }

    fn check_fs<T>(dir: &Path, name: &str, value: &T) {
        let bytes = fs::read(dir.join(name)).expect(name);
        assert!(fs_config_check_magic(&bytes), "{name}");
        let mut expected = vec![0u8; size_of::<T>()];
        fs_image::fs_config_to_bytes(value, &mut expected);
        assert_eq!(bytes, expected, "{name}");
    }

    #[test]
    fn rendered_fs_sddf_system_matches_the_config_constants() {
        assert_eq!(hex_group(BLK_STORAGE_INFO_VADDR), "0x4_000_000");
        assert_eq!(hex_group(BLK_VIRTIO_MMIO_VADDR), "0x6_000_000_000");
        assert_eq!(hex_group(BLK_VIRTIO_DMA_VADDR), "0x8_000_000_000");
        assert_eq!(hex_group(FS_REGION_SIZE), "0x8_000");
        assert_eq!(hex_group(BLK_DATA_SIZE), "0x10_000");

        let root = repo_root();
        let shell_manifest =
            std::fs::read_to_string(root.join("userspace/pds/sddf-shell/Cargo.toml"))
                .expect("shell manifest");
        assert!(
            !shell_manifest.contains("lerux-interface-types"),
            "shell manifest stays off the postcard interface crate"
        );
        assert!(
            !shell_manifest.contains("lerux-logging"),
            "shell manifest stays off the logging crate"
        );

        let xml =
            render_system(&root, "qemu_virt_aarch64_fs_sddf").expect("render filesystem image");

        assert_eq!(xml.matches("<protection_domain ").count(), 7);
        assert_eq!(xml.matches("<channel>").count(), 7);
        assert_eq!(xml.matches("program_image").count(), 7);
        assert_eq!(xml.matches("stack_size=\"0x10_000\"").count(), 7);
        assert_eq!(xml.matches("cached=\"true\"").count(), 36);
        assert_eq!(xml.matches("cached=\"false\"").count(), 1);
        assert_eq!(xml.matches("<irq ").count(), 2);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("fs_client"));
        assert!(!xml.contains("sddf-fs-client"));
        assert!(xml.contains("phys_addr=\"0xa003000\""));
        assert!(xml.contains("phys_addr=\"0x9_000_000\""));
        assert!(xml.contains(&format!(
            "<irq irq=\"78\" id=\"{BLK_DRIVER_IRQ_CHANNEL}\" />"
        )));
        assert!(xml.contains(&format!("<irq irq=\"33\" id=\"{DRIVER_IRQ_CHANNEL}\" />")));

        for (name, priority) in [
            ("blk_driver", 4),
            ("serial_driver", 4),
            ("blk_virt", 3),
            ("serial_virt_tx", 3),
            ("serial_virt_rx", 3),
            ("fatfs", 2),
            ("shell", 1),
        ] {
            assert!(
                xml.contains(&format!(
                    "<protection_domain name=\"{name}\" priority=\"{priority}\" stack_size=\"0x10_000\">"
                )),
                "{name}"
            );
        }

        for image in [
            "sddf-blk-driver.elf",
            "sddf-blk-virt.elf",
            "sddf-fatfs.elf",
            "sddf-serial-driver.elf",
            "sddf-serial-virt-tx.elf",
            "sddf-serial-virt-rx.elf",
            "sddf-shell.elf",
        ] {
            assert!(xml.contains(&format!("<program_image path=\"{image}\" />")));
        }

        for (pd, id) in [
            ("blk_driver", BLK_DRIVER_VIRT_CHANNEL),
            ("blk_virt", BLK_VIRT_DRIVER_CHANNEL),
            ("blk_virt", BLK_VIRT_CLIENT_CHANNEL),
            ("fatfs", BLK_CLIENT_VIRT_CHANNEL),
            ("fatfs", FS_SERVER_CLIENT_CHANNEL),
            ("shell", FS_SHELL_SERVER_CHANNEL),
            ("serial_driver", DRIVER_TX_CHANNEL),
            ("serial_virt_tx", VIRT_TX_DRIVER_CHANNEL),
            ("serial_driver", DRIVER_RX_CHANNEL),
            ("serial_virt_rx", VIRT_RX_DRIVER_CHANNEL),
            ("serial_virt_tx", VIRT_TX_CLIENT_CHANNEL),
            ("shell", CLIENT_TX_CHANNEL),
            ("serial_virt_rx", VIRT_RX_CLIENT_CHANNEL),
            ("shell", CLIENT_RX_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }
        assert_ne!(FS_CLIENT_SERVER_CHANNEL, FS_SHELL_SERVER_CHANNEL);

        let maps = [
            ("virtio_mmio", BLK_VIRTIO_MMIO_VADDR, 1),
            ("virtio_blk_driver_dma", BLK_VIRTIO_DMA_VADDR, 1),
            ("blk_storage_info", BLK_STORAGE_INFO_VADDR, 3),
            ("blk_driver_req_queue", BLK_DRIVER_REQ_QUEUE_VADDR, 2),
            ("blk_driver_resp_queue", BLK_DRIVER_RESP_QUEUE_VADDR, 2),
            ("blk_client_req_queue", BLK_CLIENT_REQ_QUEUE_VADDR, 2),
            ("blk_client_resp_queue", BLK_CLIENT_RESP_QUEUE_VADDR, 2),
            ("blk_data", BLK_DATA_VADDR, 2),
            ("fs_command_queue", FS_COMMAND_QUEUE_VADDR, 2),
            ("fs_completion_queue", FS_COMPLETION_QUEUE_VADDR, 2),
            ("fs_share", FS_SHARE_VADDR, 2),
            ("serial_mmio", UART_VADDR, 1),
            ("serial_tx_driver_queue", TX_DRIVER_QUEUE_VADDR, 2),
            ("serial_tx_driver_data", TX_DRIVER_DATA_VADDR, 2),
            ("serial_rx_driver_queue", RX_DRIVER_QUEUE_VADDR, 2),
            ("serial_rx_driver_data", RX_DRIVER_DATA_VADDR, 2),
            ("serial_tx_client_queue", TX_CLIENT_QUEUE_VADDR, 2),
            ("serial_tx_client_data", TX_CLIENT_DATA_VADDR, 2),
            ("serial_rx_client_queue", RX_CLIENT_QUEUE_VADDR, 2),
            ("serial_rx_client_data", RX_CLIENT_DATA_VADDR, 2),
        ];
        for (name, vaddr, count) in maps {
            let needle = format!("mr=\"{name}\" vaddr=\"{}\"", hex_group(vaddr));
            assert_eq!(xml.matches(&needle).count(), count, "{needle}");
        }

        let driver_at = xml.find("name=\"blk_driver\"").expect("driver");
        let driver_end = xml[driver_at..]
            .find("</protection_domain>")
            .expect("driver end")
            + driver_at;
        assert!(xml[driver_at..driver_end].contains("mr=\"virtio_mmio\""));
        assert!(xml[driver_at..driver_end].contains("setvar_vaddr=\"virtio_blk_mmio_vaddr\""));
        assert!(xml[driver_at..driver_end].contains("mr=\"virtio_blk_driver_dma\""));
        assert!(!xml[driver_end..].contains("mr=\"virtio_mmio\""));
        assert!(!xml[driver_end..].contains("mr=\"virtio_blk_driver_dma\""));

        let serial_at = xml.find("name=\"serial_driver\"").expect("serial driver");
        let serial_end = xml[serial_at..]
            .find("</protection_domain>")
            .expect("serial driver end")
            + serial_at;
        assert!(xml[serial_at..serial_end].contains("mr=\"serial_mmio\""));
        assert!(!xml[serial_end..].contains("mr=\"serial_mmio\""));
        assert!(!xml[serial_at..serial_end].contains("cached=\"true\" mr=\"serial_mmio\""));

        let shell_at = xml.find("name=\"shell\"").expect("shell");
        let shell_end = xml[shell_at..]
            .find("</protection_domain>")
            .expect("shell end")
            + shell_at;
        let shell = &xml[shell_at..shell_end];
        assert!(!shell.contains("mr=\"serial_mmio\""));
        assert!(!shell.contains("mr=\"virtio_mmio\""));
        assert!(!shell.contains("mr=\"blk_data\""));
    }
}
