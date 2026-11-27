//! Host rolling image.
//!
//! `lerux package install` and `lerux package upgrade` rebuild the selected
//! board profile, write `support/package-pins.toml` and `loader.img.sha256`,
//! and print the pin digest. The guest has no package installer.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    image_digest::{self, loader_path},
    profile::{self, load_profiles},
};

/// Boards whose flash line is an ISO `dd`, `just deploy-pc`, or
/// `just deploy-rpi4`. `upgrade --all` rebuilds this set.
pub(crate) const ROLLING_BOARDS: &[&str] = &["swift_sfx14_41g", "pc_z97_d3h_i217", "rpi4b_4gb"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BoardPin {
    pub profile: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PinFile {
    #[serde(default)]
    boards: BTreeMap<String, BoardPin>,
}

pub(crate) fn execute_install(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
) -> Result<()> {
    let pin = install(root, board, build_dir, config, crate::build::image)?;
    print_pin(None, &pin, board, false);
    Ok(())
}

pub(crate) fn execute_upgrade(
    root: &Path,
    board: Option<&str>,
    all: bool,
    build_dir: &str,
    config: &str,
) -> Result<()> {
    if all && board.is_some() {
        bail!("pass either --board or --all");
    }
    let boards: Vec<String> = if all {
        ROLLING_BOARDS
            .iter()
            .copied()
            .map(ToString::to_string)
            .collect()
    } else if let Some(board) = board {
        vec![board.to_string()]
    } else {
        bail!("pass --board or --all");
    };
    for board in &boards {
        let previous = read_pin(root, board)?;
        let pin = upgrade(root, board, build_dir, config, crate::build::image)?;
        print_pin(previous.as_ref(), &pin, board, true);
    }
    Ok(())
}

pub(crate) fn install<F>(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    rebuild: F,
) -> Result<BoardPin>
where
    F: FnOnce(&Path, &str, &str, &str) -> Result<()>,
{
    // Install and upgrade are the same host rebuild. The guest cannot install a package.
    upgrade(root, board, build_dir, config, rebuild)
}

pub(crate) fn upgrade<F>(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    rebuild: F,
) -> Result<BoardPin>
where
    F: FnOnce(&Path, &str, &str, &str) -> Result<()>,
{
    roll(root, board, build_dir, config, rebuild)
}

fn roll<F>(root: &Path, board: &str, build_dir: &str, config: &str, rebuild: F) -> Result<BoardPin>
where
    F: FnOnce(&Path, &str, &str, &str) -> Result<()>,
{
    require_board_name(board)?;
    let profile = profile_for_board(root, board)?;
    rebuild(root, board, build_dir, config)?;
    let loader = loader_path(root, board, build_dir);
    if !loader.is_file() {
        bail!("profile rebuild did not write {}", loader.display());
    }
    let sha256 = image_digest::write_sidecar(&loader)?;
    let pin = BoardPin { profile, sha256 };
    let mut pins = read_pins(root)?;
    pins.boards.insert(board.to_string(), pin.clone());
    write_pins(root, &pins)?;
    Ok(pin)
}

fn print_pin(previous: Option<&BoardPin>, pin: &BoardPin, board: &str, show_previous: bool) {
    if show_previous && let Some(prev) = previous.filter(|prev| prev.sha256 != pin.sha256) {
        println!("was {}", prev.sha256);
    }
    println!("pin {} profile={} board={board}", pin.sha256, pin.profile);
}

pub(crate) fn read_pin(root: &Path, board: &str) -> Result<Option<BoardPin>> {
    Ok(read_pins(root)?.boards.get(board).cloned())
}

fn pins_path(root: &Path) -> PathBuf {
    root.join("support/package-pins.toml")
}

fn read_pins(root: &Path) -> Result<PinFile> {
    let path = pins_path(root);
    if !path.is_file() {
        return Ok(PinFile::default());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    if text
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
    {
        return Ok(PinFile::default());
    }
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn write_pins(root: &Path, pins: &PinFile) -> Result<()> {
    let path = pins_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let body = toml::to_string(pins).context("format package pins")?;
    let text = format!(
        "# Host image pins. The guest does not install packages.\n\
         # Rewritten by `lerux package install` and `lerux package upgrade`.\n\
         \n{body}"
    );
    fs::write(&path, text).with_context(|| format!("write {}", path.display()))
}

fn profile_for_board(root: &Path, board: &str) -> Result<String> {
    let profiles = load_profiles(root)?;
    if let Some((name, _)) = profile::find_profile_for_board(&profiles, board) {
        return Ok(name.to_string());
    }
    Ok(board.to_string())
}

fn require_board_name(board: &str) -> Result<()> {
    let ok = !board.is_empty()
        && board
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok {
        Ok(())
    } else {
        bail!("--board must be a single name of letters, digits, '_' or '-', got {board:?}")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn write_profile(root: &Path, name: &str, board: &str) {
        let dir = root.join("support/profiles");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{name}.toml")),
            format!(
                "template = \"serial-hello.system.template\"\n\
                 pds = [\"hello\"]\n\
                 default_board = \"{board}\"\n"
            ),
        )
        .unwrap();
    }

    fn write_loader(bytes: &'static [u8]) -> impl FnOnce(&Path, &str, &str, &str) -> Result<()> {
        move |root, board, build_dir, _config| {
            let dir = root.join(build_dir).join(board);
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("loader.img"), bytes)?;
            Ok(())
        }
    }

    #[test]
    fn upgrade_writes_pin_and_loader_sha256() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_profile(root, "swift", "swift_sfx14_41g");

        let first = upgrade(
            root,
            "swift_sfx14_41g",
            "build",
            "debug",
            write_loader(b"profile-v1"),
        )
        .unwrap();
        assert_eq!(first.profile, "swift");
        assert_eq!(first.sha256.len(), 64);

        let loader = loader_path(root, "swift_sfx14_41g", "build");
        assert_eq!(image_digest::verify_sidecar(&loader).unwrap(), first.sha256);
        let pins = fs::read_to_string(pins_path(root)).unwrap();
        assert!(pins.contains(&first.sha256), "{pins}");
        assert!(pins.contains("swift_sfx14_41g"), "{pins}");

        let installed = install(
            root,
            "rpi4b_4gb",
            "build",
            "debug",
            write_loader(b"pi-image"),
        )
        .unwrap();
        assert_eq!(installed.profile, "rpi4b_4gb");

        let second = upgrade(
            root,
            "swift_sfx14_41g",
            "build",
            "debug",
            write_loader(b"profile-v2"),
        )
        .unwrap();
        assert_ne!(first.sha256, second.sha256);
        assert_eq!(read_pin(root, "swift_sfx14_41g").unwrap().unwrap(), second);
        assert_eq!(read_pin(root, "rpi4b_4gb").unwrap().unwrap(), installed);
        assert_eq!(
            image_digest::verify_sidecar(&loader).unwrap(),
            second.sha256
        );
    }

    #[test]
    fn upgrade_refuses_a_path_board_and_skips_the_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let called = Cell::new(false);
        let err = upgrade(root, "../nope", "build", "debug", |_, _, _, _| {
            called.set(true);
            Ok(())
        })
        .unwrap_err()
        .to_string();
        assert!(!called.get());
        assert!(err.contains("--board"), "{err}");
        assert!(!pins_path(root).exists());
    }

    #[test]
    fn upgrade_refuses_a_rebuild_that_writes_no_image() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let err = upgrade(root, "pc_z97_d3h_i217", "build", "debug", |_, _, _, _| {
            Ok(())
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("loader.img"), "{err}");
        assert!(!pins_path(root).exists());
    }

    #[test]
    fn comment_only_pin_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("support")).unwrap();
        fs::write(
            pins_path(tmp.path()),
            "# Host image pins. The guest does not install packages.\n",
        )
        .unwrap();
        assert!(read_pins(tmp.path()).unwrap().boards.is_empty());
    }

    #[test]
    fn rolling_boards_have_a_flash_line() {
        for board in ROLLING_BOARDS {
            assert!(
                image_digest::flash_instruction(board, "build").is_some(),
                "{board}"
            );
        }
    }
}
