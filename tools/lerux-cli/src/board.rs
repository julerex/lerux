use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use toml::Value as TomlValue;

/// Disk attachment for a QEMU board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskMode {
    #[default]
    None,
    /// virtio-blk, read-only image.
    Ro,
    /// virtio-blk, writable image.
    Rw,
}

/// Network attachment for a QEMU board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetMode {
    #[default]
    None,
    /// virtio-net on QEMU user networking.
    User,
    /// virtio-net with hostfwd tcp::18080-:8080.
    Hostfwd,
}

/// Per-board QEMU launch description. Boards without a `qemu` key are
/// hardware-only (image build + optional hw-serial smoke).
#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct QemuConfig {
    #[serde(default)]
    pub disk: DiskMode,
    #[serde(default)]
    pub net: NetMode,
    /// Requires the patched SP804 QEMU (`lerux install sp804-qemu`).
    #[serde(default)]
    pub sp804: bool,
    /// Start the host tcp-echo helper on :18080 before the smoke.
    #[serde(default)]
    pub tcp_echo: bool,
    /// Start the host one-shot HTTP origin on :8081 (fetch smoke).
    #[serde(default)]
    pub http_one: bool,
    /// Start the host one-shot HTTPS origin on :8443 (fetch-tls / request / browser smokes).
    #[serde(default)]
    pub https_one: bool,
    /// Start the host scripted completions stub on :8444 (Phase 77–79 agent).
    #[serde(default)]
    pub grok_one: bool,
    /// Attach QEMU `-device ramfb` (Phase 72 software framebuffer).
    #[serde(default)]
    pub ramfb: bool,
    /// Attach `-device virtio-gpu-device` after virtio-net and virtio-blk.
    #[serde(default)]
    pub gpu: bool,
    /// Open a QMP socket (`console.qmp` in the board build dir) so a smoke can
    /// inject PS/2 keys with `send-key`.
    #[serde(default)]
    pub qmp: bool,
    /// `qemu-xhci` at PCI 00:14.0 plus a USB HID boot keyboard.
    #[serde(default)]
    pub usb_kbd: bool,
}

/// One entry of `support/boards.toml` — the single source of truth for the
/// board matrix: build inputs, QEMU launch, smoke curls, and CI membership.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Board {
    pub arch: String,
    pub microkit_board: String,
    pub target: String,
    pub template: String,
    pub pds: Vec<String>,
    #[serde(default)]
    pub qemu: Option<QemuConfig>,
    /// Included in `lerux test-all`. The GitHub smoke matrix is the `include`
    /// list in `.github/workflows/rust.yml`, not this flag.
    #[serde(default)]
    pub ci: bool,
    /// Expected substring for a host curl of http://127.0.0.1:18080/ after boot.
    #[serde(default)]
    pub curl_expect: Option<String>,
    pub system_vars: BTreeMap<String, TomlValue>,
}

impl Board {
    pub fn qemu(&self) -> Option<&QemuConfig> {
        self.qemu.as_ref()
    }

    pub fn needs_disk(&self) -> bool {
        self.qemu.as_ref().is_some_and(|q| q.disk != DiskMode::None)
    }
}

pub type Boards = BTreeMap<String, Board>;

pub fn load_boards(root: &Path) -> Result<Boards> {
    let path = root.join("support/boards.toml");
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))
}

pub fn get_board<'a>(boards: &'a Boards, name: &str) -> Result<&'a Board> {
    boards
        .get(name)
        .with_context(|| format!("unknown board {name:?}"))
}

pub fn print_board_field(board: &Board, field: Option<&str>) -> Result<()> {
    let Some(field) = field else {
        println!("{}", serde_json::to_string(board)?);
        return Ok(());
    };
    match field {
        "arch" => println!("{}", board.arch),
        "microkit_board" => println!("{}", board.microkit_board),
        "target" | "target_triple" => println!("{}", board.target),
        "template" => println!("{}", board.template),
        "pds" => println!("{}", board.pds.join(" ")),
        "qemu" => match &board.qemu {
            Some(q) => println!("{}", serde_json::to_string(q)?),
            None => println!("(hardware)"),
        },
        "ci" => println!("{}", board.ci),
        "system_vars" => println!("{}", serde_json::to_string(&board.system_vars)?),
        _ => bail!("unknown field {field:?}"),
    }
    Ok(())
}

pub fn format_system_var(value: &TomlValue) -> String {
    match value {
        TomlValue::String(s) => s.clone(),
        TomlValue::Integer(i) => i.to_string(),
        TomlValue::Float(f) => f.to_string(),
        TomlValue::Boolean(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// True when `crate_name` declares a `board-<board>` cargo feature, meaning
/// builds and lints must pass `--features board-<board>` for it.
pub fn crate_has_board_feature(root: &Path, crate_name: &str, board: &str) -> bool {
    let manifest = root
        .join("userspace/pds")
        .join(crate_name)
        .join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return false;
    };
    let Ok(value) = text.parse::<toml::Table>() else {
        return false;
    };
    value
        .get("features")
        .and_then(|f| f.as_table())
        .is_some_and(|features| features.contains_key(&format!("board-{board}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramfb_defaults_off() {
        let q: QemuConfig = toml::from_str("").unwrap();
        assert!(!q.ramfb);
    }

    #[test]
    fn ramfb_parses() {
        let q: QemuConfig = toml::from_str("ramfb = true").unwrap();
        assert!(q.ramfb);
    }

    #[test]
    fn html_board_has_html_demo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let boards = load_boards(&root).unwrap();
        let board = boards.get("qemu_virt_aarch64_html").unwrap();
        assert!(board.ci);
        assert!(board.pds.iter().any(|p| p == "html-demo"));
        assert!(!board.pds.iter().any(|p| p == "request-server"));
    }

    #[test]
    fn qmp_flag_parses() {
        let q: QemuConfig = toml::from_str("qmp = true").unwrap();
        assert!(q.qmp);
    }

    #[test]
    fn swift_image_maps_the_panel_and_skips_serial() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let xml = crate::system::render_system(&root, "swift_sfx14_41g").expect("swift sdf");
        assert!(xml.contains("phys_addr=\"0x1fe0000\""));
        assert!(xml.contains("phys_addr=\"0xfc90000000\""));
        assert!(xml.contains("size=\"0x7e9000\""));
        assert!(xml.contains("page_size=\"0x1000\""));
        assert!(xml.contains("phys_addr=\"0x1fb0000\""));
        assert!(xml.contains("phys_addr=\"0xd1500000\""));
        assert!(xml.contains("phys_addr=\"0xd1100000\""));
        assert!(xml.contains("phys_addr=\"0xd1200000\""));
        assert!(xml.contains("phys_addr=\"0x82000000\""));
        assert!(!xml.contains("phys_addr=\"0xfd000000\""));
        assert!(xml.contains("phys_addr=\"0x1f00000\""));
        assert!(!xml.contains("phys_addr=\"0xfc000000\""));
        assert!(!xml.contains("phys_addr=\"0xfa000000\""));
        assert!(!xml.contains("phys_addr=\"0xf9000000\""));
        assert!(!xml.contains("phys_addr=\"0xfb000000\""));
        assert_eq!(xml.matches("addr=\"0xcf8\"").count(), 1);
        assert!(xml.contains("addr=\"0x60\""));
        assert!(xml.contains("addr=\"0x64\""));
        assert!(xml.contains("pin=\"1\""));
        assert!(xml.contains("vector=\"49\""));
        assert!(xml.contains("console-driver.elf"));
        assert!(xml.contains("console-shell.elf"));
        assert!(xml.contains("swift-pci-probe.elf"));
        assert!(xml.contains("swift-mt7921.elf"));
        assert!(xml.contains("swift-nvme.elf"));
        assert!(xml.contains("swift-bt.elf"));
        assert!(xml.contains("pcidev=\"03:00.0\""));
        assert!(xml.contains("pcidev=\"02:00.0\""));
        assert!(xml.contains("pcidev=\"04:00.3\""));
        assert!(xml.contains("pcidev=\"04:00.4\""));
        assert!(!xml.contains("serial-driver"));
        let boards = load_boards(&root).unwrap();
        let board = boards.get("swift_sfx14_41g").unwrap();
        assert!(!board.ci);
        assert!(board.qemu.is_none());
    }

    #[test]
    fn rtl8852be_probe_board_is_hardware_only() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let boards = load_boards(&root).unwrap();
        let board = boards.get("pc_z97_d3h_rtl8852be").unwrap();
        assert_eq!(board.arch, "x86_64");
        assert!(board.qemu.is_none());
        assert!(!board.ci);
        assert!(board.pds.iter().any(|p| p == "rtl8852be-probe"));
    }
}
