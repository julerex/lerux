//! Host check for the agent image system description.
//!
//! The agent header is defined by Milestone 13. LionsOS 0.4.0 has no agent
//! configuration struct. `grok` embeds the serial client page. The agent embeds
//! the filesystem shell page and then the network shell page.

use std::{fs, path::Path};

use anyhow::{Context, Result};

pub fn write_configs(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    bind(dir, "sddf-grok", "grok_config.bin")?;
    bind(dir, "sddf-agent", "agent_config.bin")?;
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
        agent_tool_t, blk_image, fs_client_config_t, fs_image, net_client_config_t,
        net_config_check_magic, net_image, serial_client_config_t, serial_config_check_magic,
        serial_image, AGENT_BROWSE_CHANNEL, AGENT_FS_CHANNEL, AGENT_NET_RX_CHANNEL,
        AGENT_NET_TX_CHANNEL, AGENT_TOOL_CHANNEL, AGENT_TOOL_SIZE, AGENT_TOOL_VADDR,
        LIONS_FS_MAGIC, SDDF_NET_MAGIC, SDDF_SERIAL_MAGIC, SHELL_BROWSE_CHANNEL,
        SHELL_TOOL_CHANNEL,
    };

    use super::*;
    use crate::{serial_sddf::hex_group, system::render_system};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn write_configs_splits_serial_from_the_agent_pages() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_configs(dir.path()).expect("write configs");
        let grok = fs::read(dir.path().join("grok_config.bin")).expect("read grok config");
        let agent = fs::read(dir.path().join("agent_config.bin")).expect("read agent config");
        assert_eq!(grok.len(), size_of::<serial_client_config_t>());
        assert!(serial_config_check_magic(&grok));

        let fs_len = size_of::<fs_client_config_t>();
        let net_len = size_of::<net_client_config_t>();
        assert_eq!(agent.len(), fs_len + net_len);
        assert!(net_config_check_magic(&agent[fs_len..]));
        assert!(size_of::<agent_tool_t>() <= AGENT_TOOL_SIZE as usize);

        // SAFETY: `write_configs` filled each page with the image constructors.
        let serial =
            unsafe { serial_image::serial_config_from_bytes::<serial_client_config_t>(&grok) };
        let fs = unsafe { fs_image::fs_config_from_bytes::<fs_client_config_t>(&agent[..fs_len]) };
        let net =
            unsafe { net_image::net_config_from_bytes::<net_client_config_t>(&agent[fs_len..]) };
        assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
        assert_eq!(serial.tx.id, serial_image::CLIENT_TX_CHANNEL);
        assert_eq!(serial.rx.id, serial_image::CLIENT_RX_CHANNEL);
        assert_eq!(fs.magic, LIONS_FS_MAGIC);
        assert_eq!(fs.server.id, AGENT_FS_CHANNEL);
        assert_eq!(net.magic, SDDF_NET_MAGIC);
        assert_eq!(net.rx.id, AGENT_NET_RX_CHANNEL);
        assert_eq!(net.tx.id, AGENT_NET_TX_CHANNEL);
    }

    #[test]
    fn rendered_agent_sddf_system_gives_the_queues_to_the_agent() {
        let root = repo_root();
        let grok = std::fs::read_to_string(root.join("userspace/pds/sddf-grok/Cargo.toml"))
            .expect("grok manifest");
        let agent = std::fs::read_to_string(root.join("userspace/pds/sddf-agent/Cargo.toml"))
            .expect("agent manifest");
        for manifest in [&grok, &agent] {
            assert!(
                !manifest.contains("lerux-interface-types"),
                "manifest stays off the postcard interface crate"
            );
            assert!(
                !manifest.contains("lerux-logging"),
                "manifest stays off the logging crate"
            );
        }

        let xml = render_system(&root, "qemu_virt_aarch64_agent_sddf").expect("render agent image");
        assert_eq!(xml.matches("<protection_domain ").count(), 12);
        assert_eq!(xml.matches("<channel>").count(), 14);
        assert_eq!(xml.matches("program_image").count(), 12);
        assert_eq!(xml.matches("<irq ").count(), 3);
        assert!(!xml.contains("pp="));
        assert!(!xml.contains("sddf-shell.elf"));
        assert!(xml.contains("phys_addr=\"0xa003000\""));
        assert!(xml.contains("phys_addr=\"0x9_000_000\""));
        assert!(xml.contains(&format!(
            "<memory_region name=\"agent_tool\" size=\"{}\" />",
            hex_group(AGENT_TOOL_SIZE)
        )));
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
            ("agent", 2),
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
            "sddf-grok.elf",
            "sddf-agent.elf",
        ] {
            assert!(xml.contains(&format!("<program_image path=\"{image}\" />")));
        }

        for (pd, id) in [
            ("blk_driver", blk_image::BLK_DRIVER_VIRT_CHANNEL),
            ("blk_virt", blk_image::BLK_VIRT_DRIVER_CHANNEL),
            ("blk_virt", blk_image::BLK_VIRT_CLIENT_CHANNEL),
            ("fatfs", blk_image::BLK_CLIENT_VIRT_CHANNEL),
            ("fatfs", fs_image::FS_SERVER_CLIENT_CHANNEL),
            ("agent", AGENT_FS_CHANNEL),
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
            ("agent", AGENT_NET_RX_CHANNEL),
            ("net_virt_tx", net_image::VIRT_TX_CLIENT_CHANNEL),
            ("agent", AGENT_NET_TX_CHANNEL),
            ("shell", SHELL_TOOL_CHANNEL),
            ("agent", AGENT_TOOL_CHANNEL),
            ("shell", SHELL_BROWSE_CHANNEL),
            ("agent", AGENT_BROWSE_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }

        let shell = domain(&xml, "shell");
        assert!(shell.contains("sddf-grok.elf"));
        assert!(!shell.contains("fs_share"));
        assert!(!shell.contains("net_rx_client"));
        assert!(!shell.contains("virtio"));
        for (region, addr) in [
            (
                "serial_tx_client_queue",
                serial_image::TX_CLIENT_QUEUE_VADDR,
            ),
            ("serial_tx_client_data", serial_image::TX_CLIENT_DATA_VADDR),
            (
                "serial_rx_client_queue",
                serial_image::RX_CLIENT_QUEUE_VADDR,
            ),
            ("serial_rx_client_data", serial_image::RX_CLIENT_DATA_VADDR),
            ("agent_tool", AGENT_TOOL_VADDR),
        ] {
            assert!(
                shell.contains(&format!("mr=\"{region}\" vaddr=\"{}\"", hex_group(addr))),
                "{region}"
            );
        }

        let agent_pd = domain(&xml, "agent");
        assert!(agent_pd.contains("sddf-agent.elf"));
        assert!(!agent_pd.contains("serial_tx"));
        assert!(!agent_pd.contains("virtio"));
        for (region, addr) in [
            ("fs_command_queue", fs_image::FS_COMMAND_QUEUE_VADDR),
            ("fs_completion_queue", fs_image::FS_COMPLETION_QUEUE_VADDR),
            ("fs_share", fs_image::FS_SHARE_VADDR),
            ("net_rx_client_free", net_image::NET_RX_CLIENT_FREE_VADDR),
            (
                "net_rx_client_active",
                net_image::NET_RX_CLIENT_ACTIVE_VADDR,
            ),
            ("net_tx_client_free", net_image::NET_TX_CLIENT_FREE_VADDR),
            (
                "net_tx_client_active",
                net_image::NET_TX_CLIENT_ACTIVE_VADDR,
            ),
            ("net_rx_client_data", net_image::NET_RX_CLIENT_DATA_VADDR),
            ("net_tx_data", net_image::NET_TX_DATA_VADDR),
            ("agent_tool", AGENT_TOOL_VADDR),
        ] {
            assert!(
                agent_pd.contains(&format!("mr=\"{region}\" vaddr=\"{}\"", hex_group(addr))),
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
