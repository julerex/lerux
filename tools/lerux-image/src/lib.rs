//! Host registry for device-framework images.
//!
//! Seven images render from this table. Web reuses the fetch image.
//! Protection-domain build scripts and the host writer call [`emit_pd_binding`].

mod binding;
mod images;
mod model;

pub use binding::{emit_pd_binding, pd_binding_bytes};
pub use images::{
    ahci_disk_device, board_registration, e1000e_net_device, render_board, render_kind,
    x86_disk_backend, x86_net_backend, BoardId,
};
pub use model::{DiskBackend, ImageError, ImageKind, Mode, NetBackend};

/// System description for the one-client serial image.
pub fn render_serial_xml() -> Result<String, ImageError> {
    images::render_serial()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        mem::{offset_of, size_of},
        path::PathBuf,
    };

    use lerux_machine::{ahci_msi, e1000e::e1000e_map, e1000e_standalone_irq, Arch, IoApicVector};
    use lerux_sddf::{
        fs_client_config_t, fs_config_check_magic, net_client_config_t, net_config_check_magic,
        serial_client_config_t, serial_config_check_magic, serial_connection_resource_t,
        serial_driver_config_t,
        serial_image::{
            client_config, driver_config, serial_config_to_bytes, virt_rx_config, virt_tx_config,
        },
        serial_virt_rx_config_t, serial_virt_tx_client_config_t, serial_virt_tx_config_t,
    };

    use super::render_serial_xml;
    use crate::{
        ahci_disk_device,
        binding::{bytes_for_pages, Page},
        board_registration, e1000e_net_device,
        images::{render_with_extra_end, render_with_region_vaddr, BoardId},
        pd_binding_bytes, render_board, x86_disk_backend, x86_net_backend, DiskBackend, ImageError,
        ImageKind, Mode, NetBackend,
    };

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn board_section<'a>(boards: &'a str, board: &str) -> &'a str {
        let header = format!("[{board}]");
        let start = boards
            .find(&header)
            .unwrap_or_else(|| panic!("missing [{board}]"));
        let rest = &boards[start + header.len()..];
        let end = rest.find("\n[").unwrap_or(rest.len());
        &rest[..end]
    }

    fn filled_serial_template() -> &'static str {
        include_str!("serial_filled.xml")
    }

    fn compare_bytes(actual: &str, expected: &str) -> Result<(), String> {
        if actual.as_bytes() == expected.as_bytes() {
            return Ok(());
        }
        let actual_bytes = actual.as_bytes();
        let expected_bytes = expected.as_bytes();
        let mismatch = actual_bytes
            .iter()
            .zip(expected_bytes.iter())
            .position(|(left, right)| left != right)
            .unwrap_or(actual_bytes.len().min(expected_bytes.len()));
        let line = expected_bytes
            .get(..mismatch.min(expected_bytes.len()))
            .unwrap_or_default()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            + 1;
        let rendered_line = actual.lines().nth(line - 1).unwrap_or("");
        let template_line = expected.lines().nth(line - 1).unwrap_or("");
        Err(format!(
            "rendered xml differs from the filled template\nline {line}\nrendered: {rendered_line}\ntemplate: {template_line}"
        ))
    }

    fn channel_end_id(xml: &str, pd_a: &str, pd_b: &str, want: &str) -> u8 {
        let needle_a = format!("pd=\"{pd_a}\"");
        let needle_b = format!("pd=\"{pd_b}\"");
        let want_needle = format!("pd=\"{want}\" id=\"");
        for block in xml.split("<channel>").skip(1) {
            let Some(block) = block.split("</channel>").next() else {
                continue;
            };
            if !block.contains(&needle_a) || !block.contains(&needle_b) {
                continue;
            }
            let start = block
                .find(&want_needle)
                .unwrap_or_else(|| panic!("end {want} in {pd_a} {pd_b}"));
            let rest = &block[start + want_needle.len()..];
            let end = rest.find('"').unwrap_or_else(|| panic!("id quote"));
            return rest[..end]
                .parse()
                .unwrap_or_else(|_| panic!("id {}", &rest[..end]));
        }
        panic!("no channel between {pd_a} and {pd_b}");
    }

    fn page_byte<T>(value: &T, offset: usize) -> u8 {
        let mut bytes = vec![0u8; size_of::<T>()];
        serial_config_to_bytes(value, &mut bytes);
        bytes[offset]
    }

    fn connection_id() -> usize {
        offset_of!(serial_connection_resource_t, id)
    }

    #[test]
    fn riscv_serial_board_renders_mmio_and_irq() {
        let xml = render_board("qemu_virt_riscv64_serial_sddf")
            .expect("render")
            .expect("riscv serial image");
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(xml.contains("<irq irq=\"10\" id=\"0\" />"));
    }

    #[test]
    fn x86_serial_board_renders_port_and_apic_irq() {
        let xml = render_board("x86_64_generic_serial_sddf")
            .expect("render")
            .expect("x86 serial image");
        assert!(xml.contains(
            "<ioport id=\"0\" addr=\"0x3f8\" size=\"8\" setvar_id=\"com1_ioport_id\" setvar_addr=\"com1_ioport_addr\" />"
        ));
        assert!(xml.contains(
            "<irq pin=\"4\" vector=\"48\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(!xml.contains("serial_mmio"));
        assert!(!xml.contains("irq irq="));
    }

    #[test]
    fn board_without_sddf_suffix_is_none() {
        assert_eq!(render_board("pc_z97_d3h").expect("render"), None);
    }

    #[test]
    fn riscv_filesystem_board_renders_virtio_and_serial_mmio() {
        let xml = render_board("qemu_virt_riscv64_fs_sddf")
            .expect("render")
            .expect("riscv filesystem image");
        assert!(xml.contains("phys_addr=\"0x10_001_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(xml.contains("vaddr=\"0x10_001_000\""));
        assert!(xml.contains("vaddr=\"0x41_000_000\""));
        assert!(xml.contains("<irq irq=\"1\" id=\"0\" />"));
        assert!(xml.contains("<irq irq=\"10\" id=\"0\" />"));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("0x6_000_000_000"));
    }

    #[test]
    fn aarch64_net_board_keeps_virtio_window_and_irq() {
        let xml = render_board("qemu_virt_aarch64_net_sddf")
            .expect("render")
            .expect("aarch64 network image");
        assert!(xml.contains("phys_addr=\"0xa003000\""));
        assert!(xml.contains("<irq irq=\"79\" id=\"0\" />"));
        assert!(!xml.contains("0x10_002_000"));
    }

    #[test]
    fn riscv_net_board_renders_virtio_and_serial_mmio() {
        let xml = render_board("qemu_virt_riscv64_net_sddf")
            .expect("render")
            .expect("riscv network image");
        assert!(xml.contains("phys_addr=\"0x10_002_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(xml.contains("vaddr=\"0x10_002_000\""));
        assert!(xml.contains("vaddr=\"0x40_000_000\""));
        assert!(xml.contains("<irq irq=\"2\" id=\"0\" />"));
        assert!(xml.contains("<irq irq=\"10\" id=\"0\" />"));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("0x6_000_000_000"));
    }

    #[test]
    fn riscv_fetch_board_renders_virtio_and_serial_mmio() {
        let xml = render_board("qemu_virt_riscv64_fetch_sddf")
            .expect("render")
            .expect("riscv fetch image");
        assert!(xml.contains("phys_addr=\"0x10_001_000\""));
        assert!(xml.contains("phys_addr=\"0x10_002_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(!xml.contains("0xa003000"));
        let aarch64 = render_board("qemu_virt_aarch64_fetch_sddf")
            .expect("render")
            .expect("aarch64 fetch image");
        assert!(aarch64.contains("0xa003000"));
    }

    #[test]
    fn riscv_web_board_renders_virtio_and_serial_mmio() {
        let xml = render_board("qemu_virt_riscv64_web_sddf")
            .expect("render")
            .expect("riscv web image");
        assert!(xml.contains("phys_addr=\"0x10_001_000\""));
        assert!(xml.contains("phys_addr=\"0x10_002_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(!xml.contains("0xa003000"));
        let aarch64 = render_board("qemu_virt_aarch64_web_sddf")
            .expect("render")
            .expect("aarch64 web image");
        assert!(aarch64.contains("0xa003000"));
    }

    #[test]
    fn riscv_browser_board_renders_virtio_and_serial_mmio() {
        let xml = render_board("qemu_virt_riscv64_browser_sddf")
            .expect("render")
            .expect("riscv browser image");
        assert!(xml.contains("phys_addr=\"0x10_001_000\""));
        assert!(xml.contains("phys_addr=\"0x10_002_000\""));
        assert!(xml.contains("phys_addr=\"0x10_003_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(xml.contains("vaddr=\"0x42_000_000\""));
        assert!(xml.contains("<irq irq=\"3\" id=\"0\" />"));
        assert!(!xml.contains("0xa003000"));
        let aarch64 = render_board("qemu_virt_aarch64_browser_sddf")
            .expect("render")
            .expect("aarch64 browser image");
        assert!(aarch64.contains("0xa003000"));
        assert!(aarch64.contains("<irq irq=\"77\" id=\"0\" />"));
    }

    #[test]
    fn riscv_agent_board_renders_virtio_serial_and_tool_page() {
        let xml = render_board("qemu_virt_riscv64_agent_sddf")
            .expect("render")
            .expect("riscv agent image");
        assert!(xml.contains("phys_addr=\"0x10_001_000\""));
        assert!(xml.contains("phys_addr=\"0x10_002_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(xml.contains("vaddr=\"0xa_000_000\""));
        assert!(xml.contains("sddf-grok.elf"));
        assert!(xml.contains("sddf-agent.elf"));
        assert!(!xml.contains("0xa003000"));
        assert!(xml.contains(
            "<protection_domain name=\"net_driver\" priority=\"4\" stack_size=\"0x10_000\">"
        ));
        assert!(!xml.contains("budget=\"2000\""));
        let aarch64 = render_board("qemu_virt_aarch64_agent_sddf")
            .expect("render")
            .expect("aarch64 agent image");
        assert!(aarch64.contains("0xa003000"));
        assert!(!aarch64.contains("budget=\"2000\""));
    }

    #[test]
    fn x86_agent_board_renders_pci_virtio_and_tool_page() {
        let xml = render_board("x86_64_generic_agent_sddf")
            .expect("render")
            .expect("x86 agent image");
        assert!(xml.contains(
            "<ioport id=\"0\" addr=\"0xcf8\" size=\"8\" setvar_id=\"pci_net_config_ioport_id\" setvar_addr=\"pci_net_config_ioport_addr\" />"
        ));
        assert!(xml.contains("phys_addr=\"0xfed0_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed1_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed2_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed3_0000\""));
        assert!(xml.contains("pci_ecam"));
        assert!(xml.contains(
            "<irq pin=\"11\" vector=\"49\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(xml.contains(
            "<irq pin=\"10\" vector=\"50\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(xml.contains(
            "<protection_domain name=\"net_driver\" priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\">"
        ));
        assert!(xml.contains("sddf-grok.elf"));
        assert!(xml.contains("sddf-agent.elf"));
        assert!(xml.contains("vaddr=\"0xa_000_000\""));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("serial_mmio"));
        assert!(!xml.contains("pci_config_ioport_id"));
    }

    #[test]
    fn x86_browser_board_is_unsupported() {
        let err = render_board("x86_64_generic_browser_sddf").expect_err("unsupported");
        assert!(matches!(err, ImageError::Unsupported { .. }), "{err:?}");
    }

    #[test]
    fn x86_ahci_board_registers_with_ahci_disk_backend() {
        assert_eq!(
            board_registration("x86_64_generic_fs_ahci").expect("register"),
            Some(BoardId {
                kind: ImageKind::Fs,
                mode: None,
                arch: Arch::X86_64,
                disk: DiskBackend::Ahci,
                net: NetBackend::Virtio,
            })
        );
        assert_eq!(
            x86_disk_backend("x86_64_generic_fs_ahci").expect("backend"),
            DiskBackend::Ahci
        );
    }

    #[test]
    fn x86_virtio_filesystem_board_registers_with_virtio_disk() {
        assert_eq!(
            board_registration("x86_64_generic_fs_sddf").expect("register"),
            Some(BoardId {
                kind: ImageKind::Fs,
                mode: None,
                arch: Arch::X86_64,
                disk: DiskBackend::Virtio,
                net: NetBackend::Virtio,
            })
        );
    }

    #[test]
    fn fs_ahci_token_on_non_x86_is_unsupported() {
        let err = board_registration("qemu_virt_aarch64_fs_ahci").expect_err("unsupported");
        assert!(matches!(err, ImageError::Unsupported { .. }), "{err:?}");
    }

    #[test]
    fn ahci_disk_device_formats_ide_hd_on_port_zero() {
        assert_eq!(
            ahci_disk_device("blkdev0"),
            "ide-hd,bus=ide.0,drive=blkdev0,unit=0"
        );
        assert!(!ahci_disk_device("blkdev0").contains("ich9-ahci"));
        assert!(!ahci_disk_device("blkdev0").contains("virtio-blk"));
    }

    #[test]
    fn x86_ahci_board_render_ahci_block_driver() {
        use lerux_machine::{ahci_map, ahci_msi};

        let xml = render_board("x86_64_generic_fs_ahci")
            .expect("render")
            .expect("registered filesystem image");
        let map = ahci_map();
        let irq = ahci_msi();
        assert!(xml.contains("sddf-ahci-driver.elf"));
        assert!(!xml.contains("sddf-blk-driver.elf"));
        assert!(xml.contains(&format!("phys_addr=\"{}\"", map.abar_paddr.text())));
        assert!(xml.contains(&format!("phys_addr=\"{}\"", map.dma_paddr.text())));
        assert!(xml.contains(&format!(
            "<irq pcidev=\"00:1f.2\" handle=\"{}\" vector=\"{}\" id=\"0\" />",
            irq.handle, irq.vector.0
        )));
        assert!(xml.contains("<end pd=\"blk_driver\" id=\"1\" />"));
        assert!(xml.contains("setvar_id=\"pci_config_ioport_id\""));
        assert!(!xml.contains("pci_ecam"));
        assert!(!xml.contains("virtio-blk"));
        assert!(!xml.contains("0x6200_0000"));
        assert_eq!(
            pd_binding_bytes("sddf-ahci-driver", &[]).expect("ahci binding"),
            pd_binding_bytes("sddf-blk-driver", &[]).expect("virtio binding")
        );
    }

    #[test]
    fn unregistered_x86_board_disk_backend_defaults_virtio() {
        assert_eq!(
            x86_disk_backend("x86_64_generic").expect("backend"),
            DiskBackend::Virtio
        );
    }

    #[test]
    fn x86_filesystem_board_renders_pci_and_dma() {
        let xml = render_board("x86_64_generic_fs_sddf")
            .expect("render")
            .expect("x86 filesystem image");
        assert!(xml.contains(
            "<ioport id=\"0\" addr=\"0xcf8\" size=\"8\" setvar_id=\"pci_config_ioport_id\" setvar_addr=\"pci_config_ioport_addr\" />"
        ));
        assert!(xml.contains("phys_addr=\"0xfed0_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed1_0000\""));
        assert!(xml.contains(
            "<irq pin=\"10\" vector=\"49\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(xml.contains("phys_addr=\"0x6200_0000\""));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("serial_mmio"));
    }

    #[test]
    fn x86_net_board_renders_pci_port_and_apic_irq() {
        let xml = render_board("x86_64_generic_net_sddf")
            .expect("render")
            .expect("x86 network image");
        assert!(xml.contains(
            "<ioport id=\"0\" addr=\"0xcf8\" size=\"8\" setvar_id=\"pci_net_config_ioport_id\" setvar_addr=\"pci_net_config_ioport_addr\" />"
        ));
        assert!(xml.contains("phys_addr=\"0xfed2_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed3_0000\""));
        assert!(xml.contains(
            "<irq pin=\"11\" vector=\"50\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(!xml.contains("serial_mmio"));
        assert!(!xml.contains("irq irq="));
        assert!(!xml.contains("0xa003000"));
    }

    #[test]
    fn e1000e_registration_is_x86_only() {
        assert_eq!(
            board_registration("x86_64_generic_net_e1000e").expect("register"),
            Some(BoardId {
                kind: ImageKind::Net,
                mode: None,
                arch: Arch::X86_64,
                disk: DiskBackend::Virtio,
                net: NetBackend::E1000e,
            })
        );
        let unsupported =
            board_registration("qemu_virt_aarch64_net_e1000e").expect_err("unsupported");
        assert!(
            matches!(
                unsupported,
                ImageError::Unsupported {
                    kind: ImageKind::Net,
                    arch: Arch::Aarch64
                }
            ),
            "{unsupported:?}"
        );
        assert!(matches!(
            board_registration("not_a_real_net_e1000e"),
            Err(ImageError::UnknownBoard { .. })
        ));
        assert_eq!(
            board_registration("x86_64_generic_net_sddf")
                .expect("virtio net")
                .map(|id| id.net),
            Some(NetBackend::Virtio)
        );
        assert_eq!(
            x86_net_backend("x86_64_generic").expect("default"),
            NetBackend::Virtio
        );
    }

    #[test]
    fn e1000e_net_device_line_has_no_mac() {
        let line = e1000e_net_device();
        assert_eq!(
            line,
            "e1000e,id=net0,addr=4.0,netdev=netdev0,disable_vnet_hdr=1"
        );
        assert!(!line.contains("mac="));
        assert!(!line.contains("virtio"));
    }

    #[test]
    fn x86_e1000e_board_renders_the_82574_driver() {
        let xml = render_board("x86_64_generic_net_e1000e")
            .expect("render")
            .expect("e1000e image");
        let map = e1000e_map();
        let irq = e1000e_standalone_irq();
        assert_eq!(map.dma_vaddr, lerux_sddf::net_image::NET_DRIVER_DMA_VADDR);
        assert!(xml.contains("sddf-e1000e-driver.elf"));
        assert!(!xml.contains("sddf-net-driver.elf"));
        assert!(xml.contains(&format!("phys_addr=\"{}\"", map.regs_paddr.text())));
        assert!(xml.contains(&format!("phys_addr=\"{}\"", map.dma_paddr.text())));
        assert!(xml.contains("e1000e_regs"));
        assert!(xml.contains("e1000e_dma"));
        assert!(xml.contains("pci_net_config_ioport_id"));
        assert!(xml.contains(&format!(
            "<irq pin=\"{}\" vector=\"{}\" id=\"0\" trigger=\"level\" polarity=\"low\" />",
            irq.pin.0, irq.vector.0
        )));
        assert_eq!((irq.pin.0, irq.vector.0), (11, 50));
        assert!(!xml.contains("virtio_net"));
        assert!(!xml.contains("0xfed3_0000"));
        assert!(!xml.contains("pcidev="));
        assert!(!xml.contains("0x6400_0000"));
        assert!(xml.contains(
            "<protection_domain name=\"net_driver\" priority=\"4\" stack_size=\"0x10_000\">"
        ));
        assert!(!xml.contains("budget="));
        assert!(xml.contains(&format!(
            "<end pd=\"net_driver\" id=\"{}\" />",
            lerux_sddf::net_image::DRIVER_RX_CHANNEL
        )));
        assert!(xml.contains(&format!(
            "<end pd=\"net_driver\" id=\"{}\" />",
            lerux_sddf::net_image::DRIVER_TX_CHANNEL
        )));
        let driver_at = xml.find("name=\"net_driver\"").expect("driver");
        let driver_end = xml[driver_at..]
            .find("</protection_domain>")
            .expect("driver end")
            + driver_at;
        let driver = &xml[driver_at..driver_end];
        assert!(!driver.contains("<setvar "));
        assert_eq!(
            pd_binding_bytes("sddf-e1000e-driver", &[]).expect("e1000e binding"),
            pd_binding_bytes("sddf-net-driver", &[]).expect("virtio binding")
        );
    }

    #[test]
    fn x86_fetch_native_board_row_stays_out_of_ci() {
        let root = repo_root();
        let boards = fs::read_to_string(root.join("support/boards.toml")).expect("boards");
        let header = boards.split("\n[").next().expect("header");
        assert!(header.contains("x86_64_generic_fetch_native"));
        let section = board_section(&boards, "x86_64_generic_fetch_native");
        assert!(section.contains("sddf-ahci-driver"));
        assert!(section.contains("sddf-e1000e-driver"));
        assert!(!section.contains("sddf-blk-driver"));
        assert!(!section.contains("sddf-net-driver"));
        assert!(section.contains("ci = false"));
        assert!(section.contains("disk = \"rw\""));
        assert!(section.contains("https_one = true"));
        let smokes = fs::read_to_string(root.join("support/smoke-expects.toml")).expect("smokes");
        let native = board_section(&smokes, "boards.x86_64_generic_fetch_native");
        let virtio = board_section(&smokes, "boards.x86_64_generic_fetch_sddf");
        assert_eq!(native, virtio);
        assert!(!native.contains("reboot"));
        let docs = fs::read_to_string(root.join("docs/system-generation.md")).expect("docs");
        assert!(docs.contains("x86_64_generic_fetch_native"));
    }

    #[test]
    fn e1000e_board_row_swaps_the_driver_and_stays_out_of_ci() {
        let boards = fs::read_to_string(repo_root().join("support/boards.toml")).expect("boards");
        let header = boards.split("\n[").next().expect("header");
        assert!(header.contains("x86_64_generic_net_e1000e"));
        let section = board_section(&boards, "x86_64_generic_net_e1000e");
        assert!(section.contains("sddf-e1000e-driver"));
        assert!(!section.contains("sddf-net-driver"));
        assert!(section.contains("ci = false"));
        assert!(section.contains("net = \"user\""));
    }

    #[test]
    fn x86_fetch_stays_on_the_virtio_net_driver() {
        let xml = render_board("x86_64_generic_fetch_sddf")
            .expect("render")
            .expect("fetch");
        assert!(xml.contains("sddf-net-driver.elf"));
        assert!(!xml.contains("sddf-e1000e-driver.elf"));
        assert!(xml.contains("pin=\"11\""));
    }

    fn pd_slice<'a>(xml: &'a str, name: &str) -> &'a str {
        let start = xml.find(&format!("name=\"{name}\"")).expect(name);
        let end = xml[start..].find("</protection_domain>").expect("end") + start;
        &xml[start..end]
    }

    #[test]
    fn x86_fetch_native_registers_ahci_and_e1000e() {
        assert_eq!(
            board_registration("x86_64_generic_fetch_native").expect("register"),
            Some(BoardId {
                kind: ImageKind::Fetch,
                mode: Some(Mode::Fetch),
                arch: Arch::X86_64,
                disk: DiskBackend::Ahci,
                net: NetBackend::E1000e,
            })
        );
        assert_eq!(
            x86_disk_backend("x86_64_generic_fetch_native").expect("disk"),
            DiskBackend::Ahci
        );
        assert_eq!(
            x86_net_backend("x86_64_generic_fetch_native").expect("net"),
            NetBackend::E1000e
        );
        let aarch64 =
            board_registration("qemu_virt_aarch64_fetch_native").expect_err("unsupported");
        assert!(
            matches!(
                aarch64,
                ImageError::Unsupported {
                    kind: ImageKind::Fetch,
                    arch: Arch::Aarch64
                }
            ),
            "{aarch64:?}"
        );
        let riscv = board_registration("qemu_virt_riscv64_fetch_native").expect_err("unsupported");
        assert!(
            matches!(
                riscv,
                ImageError::Unsupported {
                    kind: ImageKind::Fetch,
                    arch: Arch::Riscv64
                }
            ),
            "{riscv:?}"
        );
        assert_eq!(
            board_registration("fetch_native").expect("unregistered"),
            None
        );
    }

    #[test]
    fn x86_fetch_native_renders_ahci_and_e1000e() {
        let xml = render_board("x86_64_generic_fetch_native")
            .expect("render")
            .expect("native fetch image");
        let virtio = render_board("x86_64_generic_fetch_sddf")
            .expect("render")
            .expect("virtio fetch image");
        let msi = ahci_msi();
        assert_eq!(msi.handle, 99);
        assert_eq!(msi.vector, IoApicVector(51));
        assert_eq!(
            xml.matches("<protection_domain ").count(),
            virtio.matches("<protection_domain ").count()
        );
        assert_eq!(
            xml.matches("<channel>").count(),
            virtio.matches("<channel>").count()
        );

        assert!(xml.contains("sddf-ahci-driver.elf"));
        assert!(xml.contains("sddf-e1000e-driver.elf"));
        assert!(xml.contains("sddf-shell.elf"));
        assert!(!xml.contains("sddf-blk-driver.elf"));
        assert!(!xml.contains("sddf-net-driver.elf"));
        assert!(!xml.contains("pin=\"11\""));
        assert_eq!(xml.matches("name=\"pci_ecam\"").count(), 1);
        assert!(xml.contains("setvar_id=\"pci_net_config_ioport_id\""));
        assert!(!xml.contains("pci_config_ioport_id"));
        assert!(!xml.contains("virtio"));
        assert!(virtio.contains("sddf-net-driver.elf"));
        assert!(virtio.contains("pin=\"11\""));

        let blk = pd_slice(&xml, "blk_driver");
        assert!(blk.contains("<irq pcidev=\"00:1f.2\" handle=\"99\" vector=\"51\" id=\"0\" />"));
        assert!(blk.contains("mr=\"pci_ecam\""));
        assert!(blk.contains("setvar_vaddr=\"pci_ecam_vaddr\""));
        assert!(!blk.contains("pin="));
        assert!(!blk.contains("ioport"));
        assert!(
            blk.contains("priority=\"4\" budget=\"4000\" period=\"4000\" stack_size=\"0x10_000\"")
        );

        let net = pd_slice(&xml, "net_driver");
        assert!(net.contains(
            "<irq pin=\"10\" vector=\"50\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(net.contains("sddf-e1000e-driver.elf"));
        assert!(net.contains("pci_net_config_ioport_id"));
        assert!(!net.contains("pci_ecam"));
        assert!(
            net.contains("priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\"")
        );
    }

    #[test]
    fn x86_fetch_board_renders_pci_virtio_and_serial_port() {
        let xml = render_board("x86_64_generic_fetch_sddf")
            .expect("render")
            .expect("x86 fetch image");
        assert!(xml.contains(
            "<ioport id=\"0\" addr=\"0xcf8\" size=\"8\" setvar_id=\"pci_net_config_ioport_id\" setvar_addr=\"pci_net_config_ioport_addr\" />"
        ));
        assert!(xml.contains("phys_addr=\"0xfed0_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed1_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed2_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed3_0000\""));
        assert!(xml.contains("pci_ecam"));
        assert!(!xml.contains("pci_config_ioport_id"));
        assert!(xml.contains(
            "<irq pin=\"11\" vector=\"49\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(xml.contains(
            "<irq pin=\"10\" vector=\"50\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("serial_mmio"));
        assert!(xml.contains(
            "<protection_domain name=\"fatfs\" priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\">"
        ));
        assert!(xml.contains(
            "<protection_domain name=\"net_driver\" priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\">"
        ));
        assert!(xml.contains(
            "<protection_domain name=\"net_copy\" priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\">"
        ));
    }

    #[test]
    fn serial_xml_matches_template_bytes() {
        let rendered = render_serial_xml().expect("render serial xml");
        let expected = filled_serial_template();
        compare_bytes(&rendered, expected).unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn x86_web_reuses_the_fetch_description() {
        let fetch = render_board("x86_64_generic_fetch_sddf")
            .expect("fetch")
            .expect("x86 fetch image");
        let web = render_board("x86_64_generic_web_sddf")
            .expect("web")
            .expect("x86 web image");
        assert_eq!(fetch, web);
        assert!(web.contains("phys_addr=\"0xfed0_0000\""));
        assert!(web.contains("phys_addr=\"0xfed1_0000\""));
        assert!(web.contains("phys_addr=\"0xfed2_0000\""));
        assert!(web.contains("phys_addr=\"0xfed3_0000\""));
        assert!(web.contains("pci_ecam"));
        assert!(web.contains(
            "<ioport id=\"0\" addr=\"0xcf8\" size=\"8\" setvar_id=\"pci_net_config_ioport_id\" setvar_addr=\"pci_net_config_ioport_addr\" />"
        ));
        assert!(web.contains(
            "<irq pin=\"11\" vector=\"49\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(web.contains(
            "<irq pin=\"10\" vector=\"50\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(web.contains(
            "<protection_domain name=\"net_driver\" priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\">"
        ));
        assert!(!web.contains("0xa003000"));
        assert!(!web.contains("serial_mmio"));
        assert!(!web.contains("pci_config_ioport_id"));
        assert_eq!(
            board_registration("x86_64_generic_web_sddf").expect("register"),
            Some(BoardId {
                kind: ImageKind::Fetch,
                mode: Some(Mode::Web),
                arch: Arch::X86_64,
                disk: DiskBackend::Virtio,
                net: NetBackend::Virtio,
            })
        );
        let boards = fs::read_to_string(repo_root().join("support/boards.toml")).expect("boards");
        let section = board_section(&boards, "x86_64_generic_web_sddf");
        assert!(
            section.contains("hostfwd"),
            "web board row should set hostfwd: {section}"
        );
        let aarch64_fetch = render_board("qemu_virt_aarch64_fetch_sddf")
            .expect("fetch")
            .expect("aarch64 fetch image");
        let aarch64_web = render_board("qemu_virt_aarch64_web_sddf")
            .expect("web")
            .expect("aarch64 web image");
        assert_eq!(aarch64_fetch, aarch64_web);
        assert!(aarch64_web.contains("0xa003000"));
    }

    #[test]
    fn web_reuses_the_fetch_description() {
        let fetch = render_board("qemu_virt_aarch64_fetch_sddf")
            .expect("fetch")
            .expect("fetch image");
        let web = render_board("qemu_virt_aarch64_web_sddf")
            .expect("web")
            .expect("web image");
        assert_eq!(fetch, web);
        assert_eq!(
            board_registration("qemu_virt_aarch64_fetch_sddf").expect("register"),
            Some(BoardId {
                kind: ImageKind::Fetch,
                mode: Some(Mode::Fetch),
                arch: Arch::Aarch64,
                disk: DiskBackend::Virtio,
                net: NetBackend::Virtio,
            })
        );
        assert_eq!(
            board_registration("qemu_virt_aarch64_web_sddf").expect("register"),
            Some(BoardId {
                kind: ImageKind::Fetch,
                mode: Some(Mode::Web),
                arch: Arch::Aarch64,
                disk: DiskBackend::Virtio,
                net: NetBackend::Virtio,
            })
        );
        assert!(web.contains("sddf-shell.elf"));
        let boards = fs::read_to_string(repo_root().join("support/boards.toml")).expect("boards");
        let section = board_section(&boards, "qemu_virt_aarch64_web_sddf");
        assert!(
            section.contains("hostfwd"),
            "web board row should set hostfwd: {section}"
        );
        println!("web description matches fetch; board row sets hostfwd");
    }

    fn shell_page_order(bytes: &[u8]) -> Result<(), String> {
        let serial_len = size_of::<serial_client_config_t>();
        let fs_len = size_of::<fs_client_config_t>();
        let net_len = size_of::<net_client_config_t>();
        let order = "serial, then filesystem, then network";
        if bytes.len() != serial_len + fs_len + net_len
            || !serial_config_check_magic(&bytes[..serial_len])
            || !fs_config_check_magic(&bytes[serial_len..serial_len + fs_len])
            || !net_config_check_magic(&bytes[serial_len + fs_len..])
        {
            return Err(format!("page order must be {order}"));
        }
        Ok(())
    }

    #[test]
    fn fetch_shell_pages_are_serial_then_filesystem_then_network() {
        let xml = render_board("qemu_virt_aarch64_fetch_sddf")
            .expect("fetch")
            .expect("fetch image");
        assert_eq!(channel_end_id(&xml, "serial_virt_tx", "shell", "shell"), 0);
        assert_eq!(channel_end_id(&xml, "fatfs", "shell", "shell"), 2);
        assert_eq!(channel_end_id(&xml, "net_copy", "shell", "shell"), 3);
        let bytes = pd_binding_bytes(
            "sddf-shell",
            &["board-qemu_virt_aarch64_fetch_sddf", "net-client"],
        )
        .expect("shell binding");
        shell_page_order(&bytes).unwrap_or_else(|error| panic!("{error}"));
        println!("shell pages are serial, then filesystem, then network");
    }

    #[test]
    fn filesystem_shell_pages_start_with_serial_then_filesystem() {
        let xml = render_board("qemu_virt_aarch64_fs_sddf")
            .expect("filesystem")
            .expect("filesystem image");
        assert_eq!(channel_end_id(&xml, "serial_virt_tx", "shell", "shell"), 0);
        assert_eq!(channel_end_id(&xml, "fatfs", "shell", "shell"), 2);
        assert!(xml.contains("pp=\"true\""));
        let bytes = pd_binding_bytes("sddf-shell", &["board-qemu_virt_aarch64_fs_sddf"])
            .expect("filesystem shell");
        let serial_len = size_of::<serial_client_config_t>();
        let fs_len = size_of::<fs_client_config_t>();
        assert!(serial_config_check_magic(&bytes[..serial_len]));
        assert!(fs_config_check_magic(
            &bytes[serial_len..serial_len + fs_len]
        ));
        assert!(bytes.len() > serial_len + fs_len);
    }

    #[test]
    fn network_client_pages_are_serial_then_network() {
        let xml = render_board("qemu_virt_aarch64_net_sddf")
            .expect("network")
            .expect("network image");
        assert!(xml.contains("sddf-net-client.elf"));
        assert_eq!(
            channel_end_id(&xml, "serial_virt_tx", "net_client", "net_client"),
            0
        );
        assert_eq!(
            channel_end_id(&xml, "net_copy", "net_client", "net_client"),
            2
        );
        let bytes = pd_binding_bytes("sddf-net-client", &[]).expect("network client");
        let serial_len = size_of::<serial_client_config_t>();
        assert!(serial_config_check_magic(&bytes[..serial_len]));
        assert!(net_config_check_magic(&bytes[serial_len..]));
    }

    #[test]
    fn browser_shell_reuses_the_fetch_pages() {
        let xml = render_board("qemu_virt_aarch64_browser_sddf")
            .expect("browser")
            .expect("browser image");
        assert!(xml.contains("sddf-gpu-driver.elf"));
        assert!(xml.contains("sddf-shell.elf"));
        let browser = pd_binding_bytes("sddf-shell", &["net-client"]).expect("browser shell");
        let fetch = pd_binding_bytes(
            "sddf-shell",
            &["board-qemu_virt_aarch64_fetch_sddf", "net-client"],
        )
        .expect("fetch shell");
        assert_eq!(browser, fetch);
        shell_page_order(&browser).unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn swapped_shell_pages_fail_the_relation() {
        let bytes = bytes_for_pages(&[Page::FsShell, Page::SerialClient, Page::NetShell]);
        let error = shell_page_order(&bytes).expect_err("swapped pages");
        assert!(
            error.contains("serial, then filesystem, then network"),
            "{error}"
        );
        println!("{error}");
    }

    #[test]
    fn agent_config_has_filesystem_and_network_pages() {
        let xml = render_board("qemu_virt_aarch64_agent_sddf")
            .expect("agent")
            .expect("agent image");
        assert!(xml.contains("sddf-agent.elf"));
        assert!(xml.contains("sddf-grok.elf"));
        let bytes = pd_binding_bytes("sddf-agent", &[]).expect("agent binding");
        let fs_len = size_of::<fs_client_config_t>();
        let net_len = size_of::<net_client_config_t>();
        let serial_len = size_of::<serial_client_config_t>();
        assert_eq!(bytes.len(), fs_len + net_len);
        assert!(fs_config_check_magic(&bytes[..fs_len]));
        assert!(net_config_check_magic(&bytes[fs_len..]));
        assert!(!serial_config_check_magic(
            &bytes[..serial_len.min(bytes.len())]
        ));
        println!("agent config has filesystem and network pages and no serial page");
    }

    #[test]
    fn grok_config_is_the_serial_client_page() {
        let bytes = pd_binding_bytes("sddf-grok", &[]).expect("grok binding");
        assert_eq!(bytes.len(), size_of::<serial_client_config_t>());
        assert!(serial_config_check_magic(&bytes));
        println!("grok config is the serial client page only");
    }

    #[test]
    fn program_description_names_the_program_elf() {
        let xml = render_board("qemu_virt_aarch64_program_sddf")
            .expect("program")
            .expect("program image");
        assert!(xml.contains("sddf-program.elf"));
        assert!(xml.contains("0xa003000"));
        assert!(!xml.contains("sddf-shell.elf"));
        let bytes = pd_binding_bytes("sddf-program", &[]).expect("program binding");
        shell_page_order(&bytes).unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn riscv_program_board_renders_virtio_and_serial_mmio() {
        let xml = render_board("qemu_virt_riscv64_program_sddf")
            .expect("render")
            .expect("riscv program image");
        assert!(xml.contains("phys_addr=\"0x10_001_000\""));
        assert!(xml.contains("phys_addr=\"0x10_002_000\""));
        assert!(xml.contains("phys_addr=\"0x1_000_0000\""));
        assert!(xml.contains("sddf-program.elf"));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("sddf-shell.elf"));
        assert!(xml.contains(
            "<protection_domain name=\"net_driver\" priority=\"4\" stack_size=\"0x10_000\">"
        ));
        assert!(!xml.contains("budget=\"2000\""));
    }

    #[test]
    fn x86_program_board_renders_pci_virtio_and_serial_port() {
        let xml = render_board("x86_64_generic_program_sddf")
            .expect("render")
            .expect("x86 program image");
        assert!(xml.contains(
            "<ioport id=\"0\" addr=\"0xcf8\" size=\"8\" setvar_id=\"pci_net_config_ioport_id\" setvar_addr=\"pci_net_config_ioport_addr\" />"
        ));
        assert!(xml.contains("phys_addr=\"0xfed0_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed1_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed2_0000\""));
        assert!(xml.contains("phys_addr=\"0xfed3_0000\""));
        assert!(xml.contains("pci_ecam"));
        assert!(!xml.contains("pci_config_ioport_id"));
        assert!(xml.contains(
            "<irq pin=\"11\" vector=\"49\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(xml.contains(
            "<irq pin=\"10\" vector=\"50\" id=\"0\" trigger=\"level\" polarity=\"low\" />"
        ));
        assert!(xml.contains(
            "<protection_domain name=\"net_driver\" priority=\"2\" budget=\"2000\" period=\"2000\" stack_size=\"0x10_000\">"
        ));
        assert!(xml.contains("sddf-program.elf"));
        assert!(!xml.contains("0xa003000"));
        assert!(!xml.contains("serial_mmio"));
        assert!(!xml.contains("sddf-shell.elf"));
    }

    #[test]
    fn zero_board_features_bind_the_single_image_crates() {
        for crate_name in ["sddf-agent", "sddf-grok", "sddf-program"] {
            pd_binding_bytes(crate_name, &[])
                .unwrap_or_else(|error| panic!("{crate_name} {error}"));
        }
    }

    #[test]
    fn uart_vaddr_change_fails_byte_compare() {
        let rendered = render_with_region_vaddr("serial_mmio", 0x2_100_000).expect("render");
        let err = compare_bytes(&rendered, filled_serial_template())
            .expect_err("changed uart virtual address");
        assert!(
            err.contains("0x2_000_000"),
            "byte compare should name 0x2_000_000: {err}"
        );
        println!("byte compare failed: {err}");
    }

    #[test]
    fn duplicate_serial_driver_end_fails_before_render() {
        let err = render_with_extra_end("serial_driver", 1).expect_err("duplicate end");
        let text = err.to_string();
        assert!(
            matches!(
                err,
                ImageError::DuplicateEnd { ref pd, id: 1 } if pd == "serial_driver"
            ),
            "{text}"
        );
        assert!(!text.contains("<system>"), "{text}");
        println!("validation failed: {text}");
    }

    #[test]
    fn builder_assigns_serial_client_ends() {
        let xml = render_serial_xml().expect("render serial xml");
        let transmit = channel_end_id(&xml, "serial_virt_tx", "serial_client", "serial_client");
        let receive = channel_end_id(&xml, "serial_virt_rx", "serial_client", "serial_client");
        assert_eq!(transmit, 0);
        assert_eq!(receive, 1);
    }

    #[test]
    fn serial_xml_ids_match_config_page() {
        let xml = render_serial_xml().expect("render serial xml");
        let driver = driver_config();
        let virt_tx = virt_tx_config();
        let virt_rx = virt_rx_config();
        let client = client_config();
        let id = connection_id();
        let cases = [
            (
                "driver transmit",
                "serial_driver",
                "serial_virt_tx",
                "serial_driver",
                page_byte(&driver, offset_of!(serial_driver_config_t, tx) + id),
            ),
            (
                "driver receive",
                "serial_driver",
                "serial_virt_rx",
                "serial_driver",
                page_byte(&driver, offset_of!(serial_driver_config_t, rx) + id),
            ),
            (
                "virt tx driver",
                "serial_driver",
                "serial_virt_tx",
                "serial_virt_tx",
                page_byte(&virt_tx, offset_of!(serial_virt_tx_config_t, driver) + id),
            ),
            (
                "virt tx client",
                "serial_virt_tx",
                "serial_client",
                "serial_virt_tx",
                page_byte(
                    &virt_tx,
                    offset_of!(serial_virt_tx_config_t, clients)
                        + offset_of!(serial_virt_tx_client_config_t, conn)
                        + id,
                ),
            ),
            (
                "virt rx driver",
                "serial_driver",
                "serial_virt_rx",
                "serial_virt_rx",
                page_byte(&virt_rx, offset_of!(serial_virt_rx_config_t, driver) + id),
            ),
            (
                "virt rx client",
                "serial_virt_rx",
                "serial_client",
                "serial_virt_rx",
                page_byte(&virt_rx, offset_of!(serial_virt_rx_config_t, clients) + id),
            ),
            (
                "client transmit",
                "serial_virt_tx",
                "serial_client",
                "serial_client",
                page_byte(&client, offset_of!(serial_client_config_t, tx) + id),
            ),
            (
                "client receive",
                "serial_virt_rx",
                "serial_client",
                "serial_client",
                page_byte(&client, offset_of!(serial_client_config_t, rx) + id),
            ),
        ];
        for (label, pd_a, pd_b, want, page_id) in cases {
            let xml_id = channel_end_id(&xml, pd_a, pd_b, want);
            assert_eq!(xml_id, page_id, "{label}");
            if label == "driver transmit" {
                println!("driver transmit id {xml_id}");
            }
        }
    }
}
