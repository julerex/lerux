//! Host registry for device-framework images.
//!
//! Seven images render from this table. Web reuses the fetch image.
//! The board build still reads the system templates.

mod images;
mod model;

pub use images::{board_registration, render_board, render_kind, ImageKind, Mode};
pub use model::ImageError;

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

    use lerux_sddf::{
        serial_client_config_t, serial_connection_resource_t, serial_driver_config_t,
        serial_image::{
            client_config, driver_config, serial_config_to_bytes, virt_rx_config, virt_tx_config,
        },
        serial_virt_rx_config_t, serial_virt_tx_client_config_t, serial_virt_tx_config_t,
    };

    use super::render_serial_xml;
    use crate::{
        board_registration,
        images::{render_kind, render_with_extra_end, render_with_region_vaddr, ImageKind},
        render_board, ImageError, Mode,
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

    fn board_system_var(boards: &str, board: &str, key: &str) -> String {
        let section = board_section(boards, board);
        let line = section
            .lines()
            .find(|line| line.contains("system_vars"))
            .unwrap_or_else(|| panic!("{board} system_vars"));
        let needle = format!("{key} = ");
        let pos = line
            .find(&needle)
            .unwrap_or_else(|| panic!("{board} {key}"));
        let value = line[pos + needle.len()..].trim_start();
        if let Some(rest) = value.strip_prefix('"') {
            let end = rest.find('"').unwrap_or_else(|| panic!("{key} quote"));
            rest[..end].to_string()
        } else {
            value
                .split([',', '}'])
                .next()
                .unwrap_or_else(|| panic!("{key} value"))
                .trim()
                .to_string()
        }
    }

    fn filled_template(template_name: &str, board: &str) -> String {
        let root = repo_root();
        let template_path = root.join("userspace/systems/templates").join(template_name);
        let template = fs::read_to_string(&template_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", template_path.display()));
        let boards = fs::read_to_string(root.join("support/boards.toml")).expect("boards.toml");
        let mut rendered = template;
        for key in [
            "serial_mmio_phys_addr",
            "serial_irq",
            "virtio_mmio_phys_addr",
            "virtio_blk_irq",
            "virtio_net_irq",
            "virtio_gpu_irq",
        ] {
            if !rendered.contains(&format!("{{{key}}}")) {
                continue;
            }
            let value = board_system_var(&boards, board, key);
            rendered = rendered.replace(&format!("{{{key}}}"), &value);
        }
        rendered
    }

    fn filled_serial_template() -> String {
        filled_template(
            "serial-sddf.system.template",
            "qemu_virt_aarch64_serial_sddf",
        )
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
    fn serial_xml_matches_template_bytes() {
        let rendered = render_serial_xml().expect("render serial xml");
        let expected = filled_serial_template();
        compare_bytes(&rendered, &expected).unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn each_image_matches_its_template_bytes() {
        let cases = [
            (
                ImageKind::Serial,
                "serial-sddf.system.template",
                "qemu_virt_aarch64_serial_sddf",
            ),
            (
                ImageKind::Fs,
                "fs-sddf.system.template",
                "qemu_virt_aarch64_fs_sddf",
            ),
            (
                ImageKind::Net,
                "net-sddf.system.template",
                "qemu_virt_aarch64_net_sddf",
            ),
            (
                ImageKind::Fetch,
                "fetch-sddf.system.template",
                "qemu_virt_aarch64_fetch_sddf",
            ),
            (
                ImageKind::Browser,
                "browser-sddf.system.template",
                "qemu_virt_aarch64_browser_sddf",
            ),
            (
                ImageKind::Agent,
                "agent-sddf.system.template",
                "qemu_virt_aarch64_agent_sddf",
            ),
            (
                ImageKind::Program,
                "program-sddf.system.template",
                "qemu_virt_aarch64_program_sddf",
            ),
        ];
        for (kind, template, board) in cases {
            let rendered = render_kind(kind).unwrap_or_else(|error| panic!("{template}: {error}"));
            let expected = filled_template(template, board);
            compare_bytes(&rendered, &expected)
                .unwrap_or_else(|error| panic!("{template}\n{error}"));
        }
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
            board_registration("qemu_virt_aarch64_fetch_sddf"),
            Some((ImageKind::Fetch, Some(Mode::Fetch)))
        );
        assert_eq!(
            board_registration("qemu_virt_aarch64_web_sddf"),
            Some((ImageKind::Fetch, Some(Mode::Web)))
        );
        assert!(web.contains("sddf-shell.elf"));
    }

    #[test]
    fn uart_vaddr_change_fails_byte_compare() {
        let rendered = render_with_region_vaddr("serial_mmio", 0x2_100_000).expect("render");
        let err = compare_bytes(&rendered, &filled_serial_template())
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
