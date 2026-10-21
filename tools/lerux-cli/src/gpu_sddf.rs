//! Host check for the browser image system description.
//!
//! The graphics headers have no configuration struct, so this image adds no
//! config page. The shell still embeds the serial, filesystem, and network
//! pages written by [`crate::fetch_sddf`].

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lerux_sddf::{blk_image, fs_image, gpu_image, net_image, serial_image};

    use crate::{serial_sddf::hex_group, system::render_system};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn rendered_browser_sddf_system_matches_the_graphics_constants() {
        let root = repo_root();
        let shell_manifest =
            std::fs::read_to_string(root.join("userspace/pds/sddf-shell/Cargo.toml"))
                .expect("shell manifest");
        assert!(
            shell_manifest.contains("board-qemu_virt_aarch64_browser_sddf"),
            "shell manifest declares the browser board"
        );
        assert!(
            shell_manifest.contains("dep:lerux-web"),
            "browser board links lerux-web"
        );
        assert!(
            !shell_manifest.contains("lerux-interface-types"),
            "shell manifest stays off the postcard interface crate"
        );
        assert!(
            !shell_manifest.contains("lerux-logging"),
            "shell manifest stays off the logging crate"
        );

        let xml =
            render_system(&root, "qemu_virt_aarch64_browser_sddf").expect("render browser image");

        assert_eq!(xml.matches("<protection_domain ").count(), 13);
        assert_eq!(xml.matches("<channel>").count(), 14);
        assert_eq!(xml.matches("program_image").count(), 13);
        assert_eq!(xml.matches("<irq ").count(), 4);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("display-server"));
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
            "<irq irq=\"77\" id=\"{}\" />",
            gpu_image::DRIVER_IRQ_CHANNEL
        )));
        assert!(xml.contains(&format!(
            "<irq irq=\"33\" id=\"{}\" />",
            serial_image::DRIVER_IRQ_CHANNEL
        )));
        assert!(
            xml.contains(&format!(
                "name=\"gpu_client_data\" size=\"{}\"",
                hex_group(gpu_image::GPU_CLIENT_DATA_SIZE)
            )),
            "client pixel region size"
        );

        for (name, priority) in [
            ("blk_driver", 4),
            ("net_driver", 4),
            ("gpu_driver", 4),
            ("serial_driver", 4),
            ("blk_virt", 3),
            ("net_virt_tx", 3),
            ("net_virt_rx", 3),
            ("serial_virt_tx", 3),
            ("serial_virt_rx", 3),
            ("gpu_virt", 3),
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
            "sddf-gpu-driver.elf",
            "sddf-gpu-virt.elf",
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
            ("gpu_driver", gpu_image::DRIVER_VIRT_CHANNEL),
            ("gpu_virt", gpu_image::VIRT_DRIVER_CHANNEL),
            ("gpu_virt", gpu_image::VIRT_CLIENT_CHANNEL),
            ("shell", gpu_image::SHELL_GPU_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }

        let shell = domain(&xml, "shell");
        assert!(!shell.contains("virtio"));
        assert!(!shell.contains("gpu_driver"));
        assert!(!shell.contains("serial_mmio"));
        for (region, addr) in [
            ("gpu_client_events", gpu_image::GPU_CLIENT_EVENTS_VADDR),
            ("gpu_client_req", gpu_image::GPU_CLIENT_REQ_VADDR),
            ("gpu_client_resp", gpu_image::GPU_CLIENT_RESP_VADDR),
            ("gpu_client_data", gpu_image::GPU_CLIENT_DATA_VADDR),
        ] {
            assert!(
                shell.contains(&format!("mr=\"{region}\" vaddr=\"{}\"", hex_group(addr))),
                "{region}"
            );
        }

        let driver = domain(&xml, "gpu_driver");
        assert!(driver.contains(&format!(
            "mr=\"virtio_mmio\" vaddr=\"{}\"",
            hex_group(gpu_image::GPU_VIRTIO_MMIO_VADDR)
        )));
        assert!(driver.contains(&format!(
            "mr=\"virtio_gpu_driver_dma\" vaddr=\"{}\"",
            hex_group(gpu_image::GPU_DRIVER_DMA_VADDR)
        )));
        assert!(driver.contains("setvar_vaddr=\"virtio_gpu_mmio_vaddr\""));
        assert!(driver.contains("setvar_vaddr=\"virtio_gpu_driver_dma_vaddr\""));
        assert!(driver.contains("symbol=\"virtio_gpu_driver_dma_paddr\""));
        assert!(driver.contains(&format!(
            "mr=\"gpu_client_data\" vaddr=\"{}\"",
            hex_group(gpu_image::GPU_CLIENT_DATA_VADDR)
        )));
        for (region, addr) in [
            ("gpu_driver_events", gpu_image::GPU_DRIVER_EVENTS_VADDR),
            ("gpu_driver_req", gpu_image::GPU_DRIVER_REQ_VADDR),
            ("gpu_driver_resp", gpu_image::GPU_DRIVER_RESP_VADDR),
            ("gpu_driver_data", gpu_image::GPU_DRIVER_DATA_VADDR),
        ] {
            assert!(
                driver.contains(&format!("mr=\"{region}\" vaddr=\"{}\"", hex_group(addr))),
                "{region}"
            );
        }
    }

    fn domain<'a>(xml: &'a str, name: &str) -> &'a str {
        let at = xml
            .find(&format!("name=\"{name}\""))
            .unwrap_or_else(|| panic!("{name}"));
        let end = xml[at..]
            .find("</protection_domain>")
            .unwrap_or_else(|| panic!("{name} end"))
            + at;
        &xml[at..end]
    }
}
