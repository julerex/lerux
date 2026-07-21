//! Microkit system composition (Phase 41).
//!
//! Full SDF for a board is composed as:
//!
//! 1. **Layout body** — board `.system.template` with `{system_vars}` substituted
//!    (memory regions, protection domains, maps, IRQs). Profile-backed workstation
//!    templates omit `<channel>` blocks so channels are not dual-maintained.
//! 2. **Channels** — when a profile has `default_board = <board>` and a non-empty
//!    `[[channel]]` list, all template channels are replaced with generated XML.
//! 3. **Channel const reference** — optional `channel_consts.rs` beside the
//!    output for drift checks (`lerux profile check-channels` / emit-channels).

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    board::{format_system_var, get_board, load_boards},
    channel_consts::write_channel_consts,
    channels::{extract_channel_blocks, replace_channels_in_system},
    profile::{find_profile_for_board, load_profiles, Profile},
};

pub fn generate_system(root: &Path, board_name: &str, output: &Path) -> Result<()> {
    let rendered = render_system(root, board_name)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(output, rendered).with_context(|| format!("write {}", output.display()))?;

    // Side artifact: channel const reference when a profile owns this board.
    if let Ok(profiles) = load_profiles(root)
        && let Some((profile_name, profile)) = find_profile_for_board(&profiles, board_name)
        && !profile.channel.is_empty()
        && let Some(board_dir) = output.parent()
    {
        // Prefer writing next to system.system under build/<board>/.
        let build_dir = board_dir.parent().unwrap_or(board_dir);
        let build_dir_str = build_dir
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| build_dir.to_string_lossy().into_owned());
        let _ = write_channel_consts(root, board_name, &build_dir_str, profile_name, profile);
    }
    Ok(())
}

/// Compose the full Microkit system description for a board (no write).
pub fn render_system(root: &Path, board_name: &str) -> Result<String> {
    let body = render_system_body(root, board_name)?;
    if let Ok(profiles) = load_profiles(root)
        && let Some((profile_name, profile)) = find_profile_for_board(&profiles, board_name)
        && !profile.channel.is_empty()
    {
        return replace_channels_in_system(&body, &profile.channel).with_context(|| {
            format!("compose channels from profile {profile_name} into board {board_name}")
        });
    }
    Ok(body)
}

/// Compose SDF for a named profile (uses `default_board` or override).
pub fn render_profile_system(
    root: &Path,
    profile_name: &str,
    profile: &Profile,
    board_override: Option<&str>,
) -> Result<String> {
    let board = board_override
        .map(str::to_string)
        .or_else(|| profile.default_board.clone())
        .with_context(|| format!("profile {profile_name} has no default_board"))?;
    let body = render_system_body(root, &board)?;
    if profile.channel.is_empty() {
        return Ok(body);
    }
    replace_channels_in_system(&body, &profile.channel)
        .with_context(|| format!("compose channels for profile {profile_name}"))
}

/// Layout body: template + `system_vars` (may still contain hand channels on
/// non-profile boards).
pub fn render_system_body(root: &Path, board_name: &str) -> Result<String> {
    let boards = load_boards(root)?;
    let board = get_board(&boards, board_name)?;
    let template_path = root
        .join("userspace/systems/templates")
        .join(&board.template);
    let template = std::fs::read_to_string(&template_path)
        .with_context(|| format!("read {}", template_path.display()))?;

    let mut rendered = template;
    for (key, value) in &board.system_vars {
        let placeholder = format!("{{{key}}}");
        rendered = rendered.replace(&placeholder, &format_system_var(value));
    }
    Ok(rendered)
}

pub fn system_file(root: &Path, board: &str, build_dir: &str) -> PathBuf {
    root.join(build_dir).join(board).join("system.system")
}

pub fn board_build_dir(root: &Path, board: &str, build_dir: &str) -> PathBuf {
    root.join(build_dir).join(board)
}

/// Shared Cargo `--target-dir` root; each `--target` triple gets its own subdir.
pub fn shared_target_dir(root: &Path, build_dir: &str) -> PathBuf {
    root.join(build_dir).join("target")
}

/// Unified line-oriented diff for two SDF strings (for `profile diff`).
pub fn sdf_diff_summary(a_label: &str, a: &str, b_label: &str, b: &str) -> String {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let mut out = String::new();
    out.push_str(&format!(
        "SDF {a_label} ({} lines, {} channels) vs {b_label} ({} lines, {} channels)\n",
        a_lines.len(),
        a.matches("<channel>").count(),
        b_lines.len(),
        b.matches("<channel>").count(),
    ));

    let a_ch = extract_channel_blocks(a);
    let b_ch = extract_channel_blocks(b);
    if a_ch == b_ch {
        out.push_str("Channel blocks: identical\n");
    } else {
        out.push_str("Channel blocks: differ\n");
        let a_set: BTreeSet<&str> = a_ch.lines().filter(|l| l.contains("pd=")).collect();
        let b_set: BTreeSet<&str> = b_ch.lines().filter(|l| l.contains("pd=")).collect();
        for line in a_set.difference(&b_set) {
            out.push_str(&format!("  ch - {line}\n"));
        }
        for line in b_set.difference(&a_set) {
            out.push_str(&format!("  ch + {line}\n"));
        }
    }

    let max = a_lines.len().max(b_lines.len());
    let mut shown = 0usize;
    let mut total_diff = 0usize;
    for i in 0..max {
        let al = a_lines.get(i).copied().unwrap_or("");
        let bl = b_lines.get(i).copied().unwrap_or("");
        if al != bl {
            if total_diff == 0 {
                out.push_str("First SDF line diffs:\n");
            }
            if shown < 12 {
                out.push_str(&format!("  @@ line {} @@\n", i + 1));
                out.push_str(&format!("  - {al}\n"));
                out.push_str(&format!("  + {bl}\n"));
                shown += 1;
            }
            total_diff += 1;
        }
    }
    if total_diff > 12 {
        out.push_str(&format!("  … {} more differing lines\n", total_diff - 12));
    } else if total_diff == 0 {
        out.push_str("Full SDF: identical\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::load_profiles;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn workstation_composed_has_29_channels() {
        let root = repo_root();
        let generated = render_system(&root, "qemu_virt_aarch64_workstation").expect("gen");
        // workstation-base app/service edges + drivers + backup_shell/backup_fs (Phase 58).
        assert_eq!(generated.matches("<channel>").count(), 29);
        assert!(generated.contains("protection_domain name=\"shell\""));
        assert!(generated.contains("serial_mmio"));
        // Placeholder substitution for aarch64 virt UART.
        assert!(
            generated.contains("0x9_000_000") || generated.contains("0x9000000"),
            "serial phys addr substituted"
        );
    }

    #[test]
    fn workstation_profile_render_matches_board_render() {
        let root = repo_root();
        let profiles = load_profiles(&root).expect("profiles");
        let profile = profiles.get("workstation").expect("workstation");
        let from_profile =
            render_profile_system(&root, "workstation", profile, None).expect("profile sdf");
        let from_board = render_system(&root, "qemu_virt_aarch64_workstation").expect("board sdf");
        assert_eq!(from_profile, from_board);
    }

    #[test]
    fn workstation_template_is_channel_free_layout() {
        let root = repo_root();
        let body = render_system_body(&root, "qemu_virt_aarch64_workstation").unwrap();
        assert_eq!(
            body.matches("<channel>").count(),
            0,
            "workstation template must not dual-maintain channels"
        );
        let composed = render_system(&root, "qemu_virt_aarch64_workstation").unwrap();
        assert_eq!(composed.matches("<channel>").count(), 29);
        assert!(composed.contains("serial_virt"));
    }

    #[test]
    fn board_without_profile_keeps_template_channels() {
        let root = repo_root();
        let board = "qemu_virt_aarch64_virtio";
        let body = render_system_body(&root, board).expect("body");
        let generated = render_system(&root, board).expect("gen");
        assert_eq!(body, generated);
        assert!(generated.matches("<channel>").count() >= 1);
    }

    #[test]
    fn sdf_diff_identical() {
        let s = "<system>\n</system>\n";
        let summary = sdf_diff_summary("a", s, "b", s);
        assert!(summary.contains("identical"));
    }
}
