//! Host check for the ladybird system description.
//!
//! `web_content` maps the control page and the bitmap. It maps no network queue.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use lerux_ladybird_gfx::{
        BITMAP_VADDR, BROWSER_UI_GPU_CHANNEL, BROWSER_UI_PAINT_CHANNEL, CONTROL_VADDR, FRAME_BYTES,
        WEB_CONTENT_PAINT_CHANNEL,
    };
    use lerux_sddf::{gpu_image, serial_image};

    use crate::{serial_sddf::hex_group, system::render_system};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn rendered_ladybird_system_has_no_network_queue() {
        let root = repo_root();
        assert_manifests(&root);
        let xml = render_system(&root, "qemu_virt_aarch64_ladybird").expect("render ladybird");
        assert_programs(&xml);
        assert_irqs(&xml);
        assert_budgets(&xml);
        assert_channels(&xml);
        assert_graphics_regions(&xml);
        assert_web_content_maps_only_the_shared_pages(&xml);
        assert_browser_ui_maps_the_client_regions(&xml);
    }

    fn assert_manifests(root: &Path) {
        let web = std::fs::read_to_string(root.join("lerux-ladybird/pds/web-content/Cargo.toml"))
            .expect("web content manifest");
        let ui = std::fs::read_to_string(root.join("lerux-ladybird/pds/browser-ui/Cargo.toml"))
            .expect("browser ui manifest");
        let web_src =
            std::fs::read_to_string(root.join("lerux-ladybird/pds/web-content/src/main.rs"))
                .expect("web content source");
        assert!(!web.contains("lerux-logging"), "web content manifest");
        assert!(
            !web.contains("lerux-interface-types"),
            "web content manifest"
        );
        assert!(!web.contains("lerux-sddf"), "web content manifest");
        assert!(!ui.contains("lerux-ladybird-web"), "browser ui manifest");
        assert!(!ui.contains("lerux-ladybird-html"), "browser ui manifest");
        assert!(!ui.contains("lerux-logging"), "browser ui manifest");
        assert!(!ui.contains("lerux-interface-types"), "browser ui manifest");
        assert!(
            web_src.contains("heap_size = 1024 * 1024"),
            "web content heap"
        );
        assert!(
            web_src.contains("STACK_SIZE: usize = 0x4000"),
            "worker stack"
        );
    }

    fn assert_programs(xml: &str) {
        for image in [
            "sddf-browser-ui.elf",
            "sddf-web-content.elf",
            "sddf-gpu-driver.elf",
            "sddf-gpu-virt.elf",
            "sddf-serial-driver.elf",
            "sddf-serial-virt-tx.elf",
            "sddf-serial-virt-rx.elf",
        ] {
            assert!(xml.contains(&format!("path=\"{image}\"")), "{image}");
        }
        for absent in [
            "sddf-blk-driver.elf",
            "sddf-net-driver.elf",
            "sddf-fatfs.elf",
            "sddf-shell.elf",
            "sddf-net-copy.elf",
        ] {
            assert!(!xml.contains(absent), "{absent}");
        }
        assert_eq!(xml.matches("<protection_domain ").count(), 7);
        assert_eq!(xml.matches("<channel>").count(), 7);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("name=\"net_"));
        assert!(!xml.contains("name=\"blk_"));
    }

    fn assert_irqs(xml: &str) {
        assert!(xml.contains(&format!(
            "<irq irq=\"77\" id=\"{}\" />",
            gpu_image::DRIVER_IRQ_CHANNEL
        )));
        assert!(xml.contains(&format!(
            "<irq irq=\"33\" id=\"{}\" />",
            serial_image::DRIVER_IRQ_CHANNEL
        )));
        assert!(!xml.contains("irq=\"78\""));
        assert!(!xml.contains("irq=\"79\""));
        assert!(xml.contains("phys_addr=\"0xa003000\""));
    }

    fn assert_budgets(xml: &str) {
        for (name, priority, budget) in [
            ("gpu_driver", 4, 4000),
            ("serial_driver", 4, 4000),
            ("serial_virt_tx", 3, 3000),
            ("serial_virt_rx", 3, 3000),
            ("gpu_virt", 3, 3000),
            ("web_content", 2, 2000),
            ("browser_ui", 1, 1000),
        ] {
            let line = format!(
                "<protection_domain name=\"{name}\" priority=\"{priority}\" budget=\"{budget}\" period=\"{budget}\" stack_size=\"0x10_000\">"
            );
            assert!(xml.contains(&line), "{name}");
        }
    }

    fn assert_channels(xml: &str) {
        for (pd, id) in [
            ("serial_driver", serial_image::DRIVER_TX_CHANNEL),
            ("serial_virt_tx", serial_image::VIRT_TX_DRIVER_CHANNEL),
            ("serial_driver", serial_image::DRIVER_RX_CHANNEL),
            ("serial_virt_rx", serial_image::VIRT_RX_DRIVER_CHANNEL),
            ("serial_virt_tx", serial_image::VIRT_TX_CLIENT_CHANNEL),
            ("browser_ui", serial_image::CLIENT_TX_CHANNEL),
            ("serial_virt_rx", serial_image::VIRT_RX_CLIENT_CHANNEL),
            ("browser_ui", serial_image::CLIENT_RX_CHANNEL),
            ("gpu_driver", gpu_image::DRIVER_VIRT_CHANNEL),
            ("gpu_virt", gpu_image::VIRT_DRIVER_CHANNEL),
            ("gpu_virt", gpu_image::VIRT_CLIENT_CHANNEL),
            ("browser_ui", BROWSER_UI_GPU_CHANNEL),
            ("web_content", WEB_CONTENT_PAINT_CHANNEL),
            ("browser_ui", BROWSER_UI_PAINT_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }
    }

    fn assert_graphics_regions(xml: &str) {
        for (region, addr) in [
            ("gpu_driver_events", gpu_image::GPU_DRIVER_EVENTS_VADDR),
            ("gpu_driver_req", gpu_image::GPU_DRIVER_REQ_VADDR),
            ("gpu_driver_resp", gpu_image::GPU_DRIVER_RESP_VADDR),
            ("gpu_driver_data", gpu_image::GPU_DRIVER_DATA_VADDR),
            ("gpu_client_events", gpu_image::GPU_CLIENT_EVENTS_VADDR),
            ("gpu_client_req", gpu_image::GPU_CLIENT_REQ_VADDR),
            ("gpu_client_resp", gpu_image::GPU_CLIENT_RESP_VADDR),
            ("gpu_client_data", gpu_image::GPU_CLIENT_DATA_VADDR),
        ] {
            assert!(xml.contains(&format!("name=\"{region}\"")), "{region}");
            assert!(
                xml.contains(&format!("mr=\"{region}\" vaddr=\"{}\"", hex_group(addr))),
                "{region}"
            );
        }
        let frame = u64::try_from(FRAME_BYTES).expect("frame bytes");
        assert!(xml.contains(&format!(
            "name=\"ladybird_bitmap\" size=\"{}\"",
            hex_group(frame)
        )));
        assert!(xml.contains(&format!(
            "mr=\"ladybird_control\" vaddr=\"{}\"",
            hex_group(CONTROL_VADDR)
        )));
        assert!(xml.contains(&format!(
            "mr=\"ladybird_bitmap\" vaddr=\"{}\"",
            hex_group(BITMAP_VADDR)
        )));
    }

    fn assert_web_content_maps_only_the_shared_pages(xml: &str) {
        let web = domain(xml, "web_content");
        assert_eq!(web.matches("<map ").count(), 2);
        assert!(web.contains("mr=\"ladybird_control\""));
        assert!(web.contains("mr=\"ladybird_bitmap\""));
        assert!(!web.contains("net_"));
        assert!(!web.contains("virtio"));
        assert!(!web.contains("gpu_"));
        assert!(!web.contains("serial_"));
        assert!(!web.contains("blk_"));
        assert!(!web.contains("<irq "));
    }

    fn assert_browser_ui_maps_the_client_regions(xml: &str) {
        let ui = domain(xml, "browser_ui");
        assert!(!ui.contains("virtio"));
        assert!(!ui.contains("gpu_driver"));
        assert!(!ui.contains("serial_mmio"));
        assert!(!ui.contains("net_"));
        for region in [
            "serial_tx_client_queue",
            "serial_tx_client_data",
            "serial_rx_client_queue",
            "serial_rx_client_data",
            "gpu_client_events",
            "gpu_client_req",
            "gpu_client_resp",
            "gpu_client_data",
            "ladybird_control",
            "ladybird_bitmap",
        ] {
            assert!(ui.contains(&format!("mr=\"{region}\"")), "{region}");
        }
        assert_eq!(ui.matches("<map ").count(), 10);
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
