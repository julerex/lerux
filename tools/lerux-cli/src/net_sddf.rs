//! Writes the one-client network configuration pages next to a board build.
//!
//! The protection domains embed the same bytes from their build scripts. These
//! files are the copy the host checks against the rendered system description.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use lerux_image::{board_registration, BoardId, NetBackend};

pub fn write_configs(dir: &Path, board: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let driver = match board_registration(board).map_err(|error| anyhow::anyhow!("{error}"))? {
        Some(BoardId {
            net: NetBackend::E1000e,
            ..
        }) => "sddf-e1000e-driver",
        Some(_) | None => "sddf-net-driver",
    };
    bind(dir, driver, "net_driver_config.bin")?;
    bind(dir, "sddf-net-virt-tx", "net_virt_tx_config.bin")?;
    bind(dir, "sddf-net-virt-rx", "net_virt_rx_config.bin")?;
    bind(dir, "sddf-net-copy", "net_copy_config.bin")?;
    bind(dir, "sddf-net-client", "net_client_config.bin")?;
    Ok(())
}

fn bind(dir: &Path, crate_name: &str, file_name: &str) -> Result<()> {
    let path = dir.join(file_name);
    lerux_image::emit_pd_binding(crate_name, &[], &path)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("bind {crate_name}"))
}

#[cfg(test)]
mod tests {
    use std::{mem::size_of, path::PathBuf};

    use lerux_sddf::{
        net_config_check_magic,
        net_image::{
            self, copy_config, driver_config, virt_rx_config, virt_tx_config, NET_DRIVER_DMA_VADDR,
            NET_RX_CLIENT_ACTIVE_VADDR, NET_RX_CLIENT_DATA_VADDR, NET_RX_CLIENT_FREE_VADDR,
            NET_RX_COPY_ACTIVE_VADDR, NET_RX_COPY_FREE_VADDR, NET_RX_DATA_VADDR,
            NET_RX_DRV_ACTIVE_VADDR, NET_RX_DRV_FREE_VADDR, NET_RX_META_VADDR,
            NET_TX_CLIENT_ACTIVE_VADDR, NET_TX_CLIENT_FREE_VADDR, NET_TX_DATA_VADDR,
            NET_TX_DRV_ACTIVE_VADDR, NET_TX_DRV_FREE_VADDR, NET_VIRTIO_MMIO_VADDR,
        },
        serial_image::{
            self, RX_CLIENT_DATA_VADDR, RX_CLIENT_QUEUE_VADDR, RX_DRIVER_DATA_VADDR,
            RX_DRIVER_QUEUE_VADDR, TX_CLIENT_DATA_VADDR, TX_CLIENT_QUEUE_VADDR,
            TX_DRIVER_DATA_VADDR, TX_DRIVER_QUEUE_VADDR, UART_VADDR,
        },
    };

    use super::*;
    use crate::{serial_sddf::hex_group, system::render_system};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn write_configs_starts_with_the_net_sddf_magic() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_configs(dir.path(), "qemu_virt_aarch64_net_sddf").expect("write configs");
        check(dir.path(), "net_driver_config.bin", &driver_config());
        check(dir.path(), "net_virt_tx_config.bin", &virt_tx_config());
        check(dir.path(), "net_virt_rx_config.bin", &virt_rx_config());
        check(dir.path(), "net_copy_config.bin", &copy_config());
        let client = fs::read(dir.path().join("net_client_config.bin")).expect("client config");
        let serial_len = size_of::<lerux_sddf::serial_client_config_t>();
        let net_len = size_of::<lerux_sddf::net_client_config_t>();
        assert_eq!(client.len(), serial_len + net_len);
        assert!(lerux_sddf::serial_config_check_magic(&client[..serial_len]));
        assert!(net_config_check_magic(&client[serial_len..]));
        let mut net = vec![0u8; net_len];
        net_image::net_config_to_bytes(&net_image::client_config(), &mut net);
        assert_eq!(&client[serial_len..], net.as_slice());

        let e1000e = tempfile::tempdir().expect("tempdir");
        write_configs(e1000e.path(), "x86_64_generic_net_e1000e").expect("e1000e configs");
        check(e1000e.path(), "net_driver_config.bin", &driver_config());
        let rejected = tempfile::tempdir().expect("tempdir");
        let error = write_configs(rejected.path(), "qemu_virt_aarch64_net_e1000e")
            .expect_err("non-x86 e1000e");
        assert!(format!("{error:#}").contains("unsupported"), "{error:#}");
        assert!(!rejected.path().join("net_driver_config.bin").exists());
    }

    fn check<T>(dir: &Path, name: &str, value: &T) {
        let bytes = fs::read(dir.join(name)).expect(name);
        assert!(net_config_check_magic(&bytes), "{name}");
        let mut expected = vec![0u8; size_of::<T>()];
        net_image::net_config_to_bytes(value, &mut expected);
        assert_eq!(bytes, expected, "{name}");
    }

    #[test]
    fn rendered_net_sddf_system_matches_the_config_constants() {
        assert_eq!(hex_group(NET_RX_DRV_FREE_VADDR), "0x5_000_000");
        assert_eq!(hex_group(NET_TX_DATA_VADDR), "0x5_020_000");
        assert_eq!(hex_group(NET_VIRTIO_MMIO_VADDR), "0x6_000_000_000");
        assert_eq!(hex_group(NET_DRIVER_DMA_VADDR), "0x8_000_000_000");
        assert_eq!(hex_group(UART_VADDR), "0x2_000_000");

        let root = repo_root();
        let client_manifest =
            std::fs::read_to_string(root.join("userspace/pds/sddf-net-client/Cargo.toml"))
                .expect("client manifest");
        assert!(
            !client_manifest.contains("lerux-interface-types"),
            "client manifest stays off the postcard interface crate"
        );
        assert!(
            !client_manifest.contains("lerux-logging"),
            "client manifest stays off the logging crate"
        );

        let xml = render_system(&root, "qemu_virt_aarch64_net_sddf").expect("render network image");

        assert_eq!(xml.matches("<protection_domain ").count(), 8);
        assert_eq!(xml.matches("<channel>").count(), 9);
        assert_eq!(xml.matches("program_image").count(), 8);
        assert_eq!(xml.matches("stack_size=\"0x10_000\"").count(), 8);
        assert_eq!(xml.matches("cached=\"true\"").count(), 46);
        assert_eq!(xml.matches("cached=\"false\"").count(), 1);
        assert_eq!(xml.matches("<irq ").count(), 2);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("net_server"));
        assert!(!xml.contains("net-server"));
        assert!(xml.contains("phys_addr=\"0xa003000\""));
        assert!(xml.contains("phys_addr=\"0x9_000_000\""));
        assert!(xml.contains(&format!(
            "<irq irq=\"79\" id=\"{}\" />",
            net_image::DRIVER_IRQ_CHANNEL
        )));
        assert!(xml.contains(&format!(
            "<irq irq=\"33\" id=\"{}\" />",
            serial_image::DRIVER_IRQ_CHANNEL
        )));

        for (name, priority) in [
            ("net_driver", 4),
            ("serial_driver", 4),
            ("net_virt_tx", 3),
            ("net_virt_rx", 3),
            ("serial_virt_tx", 3),
            ("serial_virt_rx", 3),
            ("net_copy", 2),
            ("net_client", 1),
        ] {
            assert!(
                xml.contains(&format!(
                    "<protection_domain name=\"{name}\" priority=\"{priority}\" stack_size=\"0x10_000\">"
                )),
                "{name}"
            );
        }

        for image in [
            "sddf-net-driver.elf",
            "sddf-net-virt-tx.elf",
            "sddf-net-virt-rx.elf",
            "sddf-net-copy.elf",
            "sddf-serial-driver.elf",
            "sddf-serial-virt-tx.elf",
            "sddf-serial-virt-rx.elf",
            "sddf-net-client.elf",
        ] {
            assert!(xml.contains(&format!("<program_image path=\"{image}\" />")));
        }

        for (pd, id) in [
            ("net_driver", net_image::DRIVER_RX_CHANNEL),
            ("net_virt_rx", net_image::VIRT_RX_DRIVER_CHANNEL),
            ("net_driver", net_image::DRIVER_TX_CHANNEL),
            ("net_virt_tx", net_image::VIRT_TX_DRIVER_CHANNEL),
            ("net_virt_rx", net_image::VIRT_RX_COPY_CHANNEL),
            ("net_copy", net_image::COPY_VIRT_CHANNEL),
            ("net_virt_tx", net_image::VIRT_TX_CLIENT_CHANNEL),
            ("net_client", net_image::CLIENT_NET_TX_CHANNEL),
            ("net_copy", net_image::COPY_CLIENT_CHANNEL),
            ("net_client", net_image::CLIENT_NET_RX_CHANNEL),
            ("serial_driver", serial_image::DRIVER_TX_CHANNEL),
            ("serial_virt_tx", serial_image::VIRT_TX_DRIVER_CHANNEL),
            ("serial_driver", serial_image::DRIVER_RX_CHANNEL),
            ("serial_virt_rx", serial_image::VIRT_RX_DRIVER_CHANNEL),
            ("serial_virt_tx", serial_image::VIRT_TX_CLIENT_CHANNEL),
            ("net_client", serial_image::CLIENT_TX_CHANNEL),
            ("serial_virt_rx", serial_image::VIRT_RX_CLIENT_CHANNEL),
            ("net_client", serial_image::CLIENT_RX_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }

        for (name, vaddr, count) in [
            ("serial_mmio", UART_VADDR, 1),
            ("serial_tx_driver_queue", TX_DRIVER_QUEUE_VADDR, 2),
            ("serial_tx_driver_data", TX_DRIVER_DATA_VADDR, 2),
            ("serial_rx_driver_queue", RX_DRIVER_QUEUE_VADDR, 2),
            ("serial_rx_driver_data", RX_DRIVER_DATA_VADDR, 2),
            ("serial_tx_client_queue", TX_CLIENT_QUEUE_VADDR, 2),
            ("serial_tx_client_data", TX_CLIENT_DATA_VADDR, 2),
            ("serial_rx_client_queue", RX_CLIENT_QUEUE_VADDR, 2),
            ("serial_rx_client_data", RX_CLIENT_DATA_VADDR, 2),
            ("virtio_mmio", NET_VIRTIO_MMIO_VADDR, 1),
            ("virtio_net_driver_dma", NET_DRIVER_DMA_VADDR, 1),
            ("net_rx_drv_free", NET_RX_DRV_FREE_VADDR, 2),
            ("net_rx_drv_active", NET_RX_DRV_ACTIVE_VADDR, 2),
            ("net_tx_drv_free", NET_TX_DRV_FREE_VADDR, 2),
            ("net_tx_drv_active", NET_TX_DRV_ACTIVE_VADDR, 2),
            ("net_rx_copy_free", NET_RX_COPY_FREE_VADDR, 2),
            ("net_rx_copy_active", NET_RX_COPY_ACTIVE_VADDR, 2),
            ("net_rx_client_free", NET_RX_CLIENT_FREE_VADDR, 2),
            ("net_rx_client_active", NET_RX_CLIENT_ACTIVE_VADDR, 2),
            ("net_tx_client_free", NET_TX_CLIENT_FREE_VADDR, 2),
            ("net_tx_client_active", NET_TX_CLIENT_ACTIVE_VADDR, 2),
            ("net_rx_data", NET_RX_DATA_VADDR, 3),
            ("net_tx_data", NET_TX_DATA_VADDR, 3),
            ("net_rx_client_data", NET_RX_CLIENT_DATA_VADDR, 2),
            ("net_rx_meta", NET_RX_META_VADDR, 1),
        ] {
            let needle = format!("mr=\"{name}\" vaddr=\"{}\"", hex_group(vaddr));
            assert_eq!(xml.matches(&needle).count(), count, "{needle}");
        }

        let driver_at = xml.find("name=\"net_driver\"").expect("driver");
        let driver_end = xml[driver_at..]
            .find("</protection_domain>")
            .expect("driver end")
            + driver_at;
        let driver = &xml[driver_at..driver_end];
        assert!(driver.contains("mr=\"virtio_mmio\""));
        assert!(driver.contains("setvar_vaddr=\"virtio_net_mmio_vaddr\""));
        assert!(driver.contains("mr=\"virtio_net_driver_dma\""));
        assert!(driver.contains("region_paddr=\"virtio_net_driver_dma\""));

        let client_at = xml.find("name=\"net_client\"").expect("client");
        let client_end = xml[client_at..]
            .find("</protection_domain>")
            .expect("client end")
            + client_at;
        let client = &xml[client_at..client_end];
        assert!(!client.contains("net_rx_drv"));
        assert!(!client.contains("net_tx_drv"));
        assert!(!client.contains("net_rx_copy"));
        assert!(!client.contains("net_rx_data"));
        assert!(!client.contains("net_rx_meta"));
        assert!(!client.contains("virtio"));
        assert!(client.contains("mr=\"net_tx_data\""));
        assert!(client.contains("mr=\"net_rx_client_data\""));
        assert!(client.contains("mr=\"serial_tx_client_queue\""));
    }
}
