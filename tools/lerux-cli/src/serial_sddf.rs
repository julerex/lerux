//! Writes the one-client serial configuration pages next to a board build.
//!
//! The protection domains embed the same bytes from their build scripts. These
//! files are the copy the host checks against the rendered system description.

use std::{fs, mem::size_of, path::Path};

use anyhow::{Context, Result};
use lerux_sddf::serial_image::{
    client_config, driver_config, serial_config_to_bytes, virt_rx_config, virt_tx_config,
};

#[cfg(test)]
pub(crate) fn hex_group(value: u64) -> String {
    let hex = format!("{value:x}");
    let mut grouped = String::new();
    for (index, ch) in hex.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('_');
        }
        grouped.push(ch);
    }
    format!("0x{}", grouped.chars().rev().collect::<String>())
}

pub fn write_configs(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    write_one(dir, "serial_driver_config.bin", &driver_config())?;
    write_one(dir, "serial_virt_tx_config.bin", &virt_tx_config())?;
    write_one(dir, "serial_virt_rx_config.bin", &virt_rx_config())?;
    write_one(dir, "serial_client_config.bin", &client_config())?;
    Ok(())
}

fn write_one<T>(dir: &Path, name: &str, value: &T) -> Result<()> {
    let mut bytes = vec![0u8; size_of::<T>()];
    serial_config_to_bytes(value, &mut bytes);
    let path = dir.join(name);
    fs::write(&path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lerux_sddf::{
        serial_config_check_magic,
        serial_image::{
            client_config, driver_config, serial_config_to_bytes, virt_rx_config, virt_tx_config,
            CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL, DRIVER_IRQ_CHANNEL, DRIVER_RX_CHANNEL,
            DRIVER_TX_CHANNEL, RX_CLIENT_DATA_VADDR, RX_CLIENT_QUEUE_VADDR, RX_DRIVER_DATA_VADDR,
            RX_DRIVER_QUEUE_VADDR, TX_CLIENT_DATA_VADDR, TX_CLIENT_QUEUE_VADDR,
            TX_DRIVER_DATA_VADDR, TX_DRIVER_QUEUE_VADDR, UART_VADDR, VIRT_RX_CLIENT_CHANNEL,
            VIRT_RX_DRIVER_CHANNEL, VIRT_TX_CLIENT_CHANNEL, VIRT_TX_DRIVER_CHANNEL,
        },
    };

    use super::*;
    use crate::system::render_system;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn write_configs_starts_with_the_serial_magic() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_configs(dir.path()).expect("write configs");
        check_file(dir.path(), "serial_driver_config.bin", &driver_config());
        check_file(dir.path(), "serial_virt_tx_config.bin", &virt_tx_config());
        check_file(dir.path(), "serial_virt_rx_config.bin", &virt_rx_config());
        check_file(dir.path(), "serial_client_config.bin", &client_config());
    }

    fn check_file<T>(dir: &Path, name: &str, value: &T) {
        let bytes = fs::read(dir.join(name)).expect(name);
        assert!(serial_config_check_magic(&bytes), "{name}");
        let mut expected = vec![0u8; size_of::<T>()];
        serial_config_to_bytes(value, &mut expected);
        assert_eq!(bytes, expected, "{name}");
    }

    #[test]
    fn rendered_system_matches_the_config_constants() {
        assert_eq!(hex_group(UART_VADDR), "0x2_000_000");
        assert_eq!(hex_group(TX_DRIVER_QUEUE_VADDR), "0x3_000_000");
        assert_eq!(hex_group(0x10_000), "0x10_000");

        let xml = render_system(&repo_root(), "qemu_virt_aarch64_serial_sddf")
            .expect("render serial image");

        assert_eq!(xml.matches("<protection_domain ").count(), 4);
        assert_eq!(xml.matches("<channel>").count(), 4);
        assert_eq!(xml.matches("program_image").count(), 4);
        assert_eq!(xml.matches("stack_size=\"0x10_000\"").count(), 4);
        assert_eq!(xml.matches("cached=\"true\"").count(), 16);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("echo_server"));
        assert!(!xml.contains("echo_client"));
        assert!(!xml.contains("hello.elf"));
        assert!(xml.contains("phys_addr=\"0x9_000_000\""));
        assert!(xml.contains(&format!("<irq irq=\"33\" id=\"{DRIVER_IRQ_CHANNEL}\" />")));

        for (name, priority) in [
            ("serial_driver", 4),
            ("serial_virt_tx", 3),
            ("serial_virt_rx", 3),
            ("serial_client", 1),
        ] {
            assert!(
                xml.contains(&format!(
                    "<protection_domain name=\"{name}\" priority=\"{priority}\" stack_size=\"0x10_000\">"
                )),
                "{name}"
            );
        }

        for image in [
            "sddf-serial-driver.elf",
            "sddf-serial-virt-tx.elf",
            "sddf-serial-virt-rx.elf",
            "sddf-serial-client.elf",
        ] {
            assert!(xml.contains(&format!("<program_image path=\"{image}\" />")));
        }

        for (pd, id) in [
            ("serial_driver", DRIVER_TX_CHANNEL),
            ("serial_virt_tx", VIRT_TX_DRIVER_CHANNEL),
            ("serial_driver", DRIVER_RX_CHANNEL),
            ("serial_virt_rx", VIRT_RX_DRIVER_CHANNEL),
            ("serial_virt_tx", VIRT_TX_CLIENT_CHANNEL),
            ("serial_client", CLIENT_TX_CHANNEL),
            ("serial_virt_rx", VIRT_RX_CLIENT_CHANNEL),
            ("serial_client", CLIENT_RX_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }

        let maps = [
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

        let driver_at = xml.find("name=\"serial_driver\"").expect("driver");
        let driver_end = xml[driver_at..]
            .find("</protection_domain>")
            .expect("driver end")
            + driver_at;
        assert!(xml[driver_at..driver_end].contains("mr=\"serial_mmio\""));
        assert!(xml[driver_at..driver_end].contains("setvar_vaddr=\"serial_register_block\""));
        assert!(!xml[driver_end..].contains("mr=\"serial_mmio\""));
        assert!(!xml[driver_at..driver_end].contains("cached=\"true\" mr=\"serial_mmio\""));
    }
}
