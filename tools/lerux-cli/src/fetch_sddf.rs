//! Writes the fetch shell configuration page next to a board build.
//!
//! The shell embeds the same bytes from its build script: the serial client
//! page, the filesystem client page, then the network client page. The host
//! checks those bytes and the rendered system description.

use std::{fs, path::Path};

use anyhow::{Context, Result};

fn shell_board_feature(board: &str) -> &'static str {
    match board {
        "qemu_virt_riscv64_fetch_sddf" => "board-qemu_virt_riscv64_fetch_sddf",
        "qemu_virt_riscv64_web_sddf" => "board-qemu_virt_riscv64_web_sddf",
        "x86_64_generic_fetch_sddf" => "board-x86_64_generic_fetch_sddf",
        "x86_64_generic_web_sddf" => "board-x86_64_generic_web_sddf",
        "qemu_virt_aarch64_web_sddf" => "board-qemu_virt_aarch64_web_sddf",
        _ => "board-qemu_virt_aarch64_fetch_sddf",
    }
}

pub fn write_configs(dir: &Path, board: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let path = dir.join("shell_config.bin");
    let board_feature = shell_board_feature(board);
    lerux_image::emit_pd_binding("sddf-shell", &[board_feature, "net-client"], &path)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| "bind sddf-shell")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{mem::size_of, path::PathBuf};

    use lerux_sddf::{
        blk_image, fs_client_config_t, fs_image, net_client_config_t, net_config_check_magic,
        net_image, serial_client_config_t, serial_config_check_magic, serial_image, LIONS_FS_MAGIC,
        SDDF_NET_MAGIC, SDDF_SERIAL_MAGIC,
    };

    use super::*;
    use crate::{serial_sddf::hex_group, system::render_system};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn write_configs_is_serial_then_filesystem_then_network() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_configs(dir.path(), "qemu_virt_aarch64_fetch_sddf").expect("write configs");
        let bytes = fs::read(dir.path().join("shell_config.bin")).expect("read shell config");

        let serial_len = size_of::<serial_client_config_t>();
        let fs_len = size_of::<fs_client_config_t>();
        let net_len = size_of::<net_client_config_t>();
        assert_eq!(bytes.len(), serial_len + fs_len + net_len);
        assert!(serial_config_check_magic(&bytes[..serial_len]));
        assert!(net_config_check_magic(&bytes[serial_len + fs_len..]));

        // SAFETY: `write_configs` filled each page with the image constructors.
        let serial = unsafe {
            serial_image::serial_config_from_bytes::<serial_client_config_t>(&bytes[..serial_len])
        };
        let fs = unsafe {
            fs_image::fs_config_from_bytes::<fs_client_config_t>(
                &bytes[serial_len..serial_len + fs_len],
            )
        };
        let net = unsafe {
            net_image::net_config_from_bytes::<net_client_config_t>(&bytes[serial_len + fs_len..])
        };
        assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
        assert_eq!(serial.tx.id, serial_image::CLIENT_TX_CHANNEL);
        assert_eq!(serial.rx.id, serial_image::CLIENT_RX_CHANNEL);
        assert_eq!(fs.magic, LIONS_FS_MAGIC);
        assert_eq!(fs.server.id, fs_image::FS_SHELL_SERVER_CHANNEL);
        assert_eq!(net.magic, SDDF_NET_MAGIC);
        assert_eq!(net.rx.id, net_image::SHELL_NET_RX_CHANNEL);
        assert_eq!(net.tx.id, net_image::SHELL_NET_TX_CHANNEL);
    }

    #[test]
    fn rendered_fetch_sddf_system_matches_the_config_constants() {
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

        let xml = render_system(&root, "qemu_virt_aarch64_fetch_sddf").expect("render fetch image");

        assert_eq!(xml.matches("<protection_domain ").count(), 11);
        assert_eq!(xml.matches("<channel>").count(), 12);
        assert_eq!(xml.matches("program_image").count(), 11);
        assert_eq!(xml.matches("<irq ").count(), 3);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("net_server"));
        assert!(!xml.contains("net-server"));
        assert!(xml.contains("phys_addr=\"0xa003000\""));
        assert!(xml.contains("phys_addr=\"0x9_000_000\""));
        assert!(xml.contains(&format!(
            "<irq irq=\"78\" id=\"{}\" />",
            blk_image::BLK_DRIVER_IRQ_CHANNEL
        )));
        assert!(xml.contains(&format!(
            "<irq irq=\"79\" id=\"{}\" />",
            net_image::DRIVER_IRQ_CHANNEL
        )));
        assert!(xml.contains(&format!(
            "<irq irq=\"33\" id=\"{}\" />",
            serial_image::DRIVER_IRQ_CHANNEL
        )));

        for (name, priority) in [
            ("blk_driver", 4),
            ("net_driver", 4),
            ("serial_driver", 4),
            ("blk_virt", 3),
            ("net_virt_tx", 3),
            ("net_virt_rx", 3),
            ("serial_virt_tx", 3),
            ("serial_virt_rx", 3),
            ("fatfs", 2),
            ("net_copy", 2),
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
            "sddf-net-driver.elf",
            "sddf-net-virt-tx.elf",
            "sddf-net-virt-rx.elf",
            "sddf-net-copy.elf",
            "sddf-serial-driver.elf",
            "sddf-serial-virt-tx.elf",
            "sddf-serial-virt-rx.elf",
            "sddf-shell.elf",
        ] {
            assert!(xml.contains(&format!("<program_image path=\"{image}\" />")));
        }

        for (pd, id) in [
            ("blk_driver", blk_image::BLK_DRIVER_VIRT_CHANNEL),
            ("blk_virt", blk_image::BLK_VIRT_DRIVER_CHANNEL),
            ("blk_virt", blk_image::BLK_VIRT_CLIENT_CHANNEL),
            ("fatfs", blk_image::BLK_CLIENT_VIRT_CHANNEL),
            ("fatfs", fs_image::FS_SERVER_CLIENT_CHANNEL),
            ("shell", fs_image::FS_SHELL_SERVER_CHANNEL),
            ("serial_driver", serial_image::DRIVER_TX_CHANNEL),
            ("serial_virt_tx", serial_image::VIRT_TX_DRIVER_CHANNEL),
            ("serial_driver", serial_image::DRIVER_RX_CHANNEL),
            ("serial_virt_rx", serial_image::VIRT_RX_DRIVER_CHANNEL),
            ("serial_virt_tx", serial_image::VIRT_TX_CLIENT_CHANNEL),
            ("shell", serial_image::CLIENT_TX_CHANNEL),
            ("serial_virt_rx", serial_image::VIRT_RX_CLIENT_CHANNEL),
            ("shell", serial_image::CLIENT_RX_CHANNEL),
            ("net_driver", net_image::DRIVER_RX_CHANNEL),
            ("net_virt_rx", net_image::VIRT_RX_DRIVER_CHANNEL),
            ("net_driver", net_image::DRIVER_TX_CHANNEL),
            ("net_virt_tx", net_image::VIRT_TX_DRIVER_CHANNEL),
            ("net_virt_rx", net_image::VIRT_RX_COPY_CHANNEL),
            ("net_copy", net_image::COPY_VIRT_CHANNEL),
            ("net_copy", net_image::COPY_CLIENT_CHANNEL),
            ("shell", net_image::SHELL_NET_RX_CHANNEL),
            ("net_virt_tx", net_image::VIRT_TX_CLIENT_CHANNEL),
            ("shell", net_image::SHELL_NET_TX_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }

        let shell_at = xml.find("name=\"shell\"").expect("shell");
        let shell_end = xml[shell_at..]
            .find("</protection_domain>")
            .expect("shell end")
            + shell_at;
        let shell = &xml[shell_at..shell_end];
        assert!(!shell.contains("virtio"));
        assert!(!shell.contains("serial_mmio"));
        assert!(!shell.contains("net_rx_drv"));
        assert!(!shell.contains("net_tx_drv"));
        assert!(!shell.contains("net_rx_copy"));
        assert!(!shell.contains("mr=\"net_rx_data\""));
        assert!(!shell.contains("net_rx_meta"));
        assert!(shell.contains(&format!(
            "mr=\"net_tx_data\" vaddr=\"{}\"",
            hex_group(net_image::NET_TX_DATA_VADDR)
        )));
        assert!(shell.contains(&format!(
            "mr=\"net_rx_client_data\" vaddr=\"{}\"",
            hex_group(net_image::NET_RX_CLIENT_DATA_VADDR)
        )));
        assert!(shell.contains(&format!(
            "mr=\"fs_share\" vaddr=\"{}\"",
            hex_group(fs_image::FS_SHARE_VADDR)
        )));
    }
}
