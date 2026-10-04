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
        assert_eq!(hex_group(BLK_STORAGE_INFO_VADDR), "0x3_000_000");
        assert_eq!(hex_group(BLK_VIRTIO_MMIO_VADDR), "0x6_000_000_000");
        assert_eq!(hex_group(BLK_VIRTIO_DMA_VADDR), "0x8_000_000_000");
        assert_eq!(hex_group(FS_REGION_SIZE), "0x8_000");
        assert_eq!(hex_group(BLK_DATA_SIZE), "0x10_000");

        let xml = render_system(&repo_root(), "qemu_virt_aarch64_fs_sddf")
            .expect("render filesystem image");

        assert_eq!(xml.matches("<protection_domain ").count(), 4);
        assert_eq!(xml.matches("<channel>").count(), 3);
        assert_eq!(xml.matches("program_image").count(), 4);
        assert_eq!(xml.matches("stack_size=\"0x10_000\"").count(), 4);
        assert_eq!(xml.matches("cached=\"true\"").count(), 20);
        assert_eq!(xml.matches("cached=\"false\"").count(), 1);
        assert_eq!(xml.matches("<irq ").count(), 1);
        assert!(!xml.contains("pp="));
        assert!(xml.contains("phys_addr=\"0xa003000\""));
        assert!(xml.contains(&format!(
            "<irq irq=\"78\" id=\"{BLK_DRIVER_IRQ_CHANNEL}\" />"
        )));

        for (name, priority) in [
            ("blk_driver", 4),
            ("blk_virt", 3),
            ("fatfs", 2),
            ("fs_client", 1),
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
            "sddf-fs-client.elf",
        ] {
            assert!(xml.contains(&format!("<program_image path=\"{image}\" />")));
        }

        for (pd, id) in [
            ("blk_driver", BLK_DRIVER_VIRT_CHANNEL),
            ("blk_virt", BLK_VIRT_DRIVER_CHANNEL),
            ("blk_virt", BLK_VIRT_CLIENT_CHANNEL),
            ("fatfs", BLK_CLIENT_VIRT_CHANNEL),
            ("fatfs", FS_SERVER_CLIENT_CHANNEL),
            ("fs_client", FS_CLIENT_SERVER_CHANNEL),
        ] {
            assert!(
                xml.contains(&format!("<end pd=\"{pd}\" id=\"{id}\" />")),
                "{pd} {id}"
            );
        }

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
        assert!(!xml[driver_end..].contains("<irq "));
    }
}
