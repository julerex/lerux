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

use anyhow::{anyhow, bail, Context, Result};

use crate::{
    board::{format_system_var, get_board, load_boards},
    channel_consts::write_channel_consts,
    channels::{extract_channel_blocks, replace_channels_in_system, to_sdf_pd_name, ChannelSpec},
    profile::{find_profile_for_board, load_profiles, Profile},
};

/// System-description name of one protection domain.
struct ComponentName(String);

/// One `<protection_domain>` element. Constructed only by [`ComponentSet::parse`].
pub(crate) struct Component {
    name: ComponentName,
    /// Bytes from `<protection_domain` through `</protection_domain>`.
    xml: String,
}

/// Protection domains of one template, in document order.
pub(crate) struct ComponentSet {
    order: Vec<Component>,
}

impl ComponentSet {
    /// Duplicate `name` returns `duplicate component "{name}"`.
    /// Elements do not nest.
    pub(crate) fn parse(template_body: &str) -> Result<Self> {
        let mut order: Vec<Component> = Vec::new();
        let mut rest = template_body;
        while let Some(start) = rest.find("<protection_domain") {
            let after = &rest[start..];
            let Some(close_rel) = after.find("</protection_domain>") else {
                bail!("unclosed protection domain");
            };
            let end = close_rel + "</protection_domain>".len();
            let xml = &after[..end];
            let name = component_name(xml)?;
            if order.iter().any(|component| component.name.0 == name) {
                bail!("duplicate component \"{name}\"");
            }
            order.push(Component {
                name: ComponentName(name),
                xml: xml.to_string(),
            });
            rest = &after[end..];
        }
        Ok(Self { order })
    }

    /// `to_sdf_pd_name` once. A missing name returns `unknown component "{sdf_name}"`.
    pub(crate) fn resolve(&self, pd: &str) -> Result<&Component> {
        let sdf_name = to_sdf_pd_name(pd);
        self.order
            .iter()
            .find(|component| component.name.0 == sdf_name)
            .ok_or_else(|| anyhow!("unknown component \"{sdf_name}\""))
    }

    pub(crate) fn sdf_names(&self) -> Vec<String> {
        self.order
            .iter()
            .map(|component| component.name.0.clone())
            .collect()
    }
}

fn component_name(xml: &str) -> Result<String> {
    let tag_end = xml
        .find('>')
        .ok_or_else(|| anyhow!("protection domain tag is missing a closing bracket"))?;
    let tag = &xml[..tag_end];
    let Some(start) = tag.find("name=\"") else {
        bail!("protection domain is missing a name");
    };
    let rest = &tag[start + "name=\"".len()..];
    let Some(end) = rest.find('"') else {
        bail!("protection domain name is missing a closing quote");
    };
    Ok(rest[..end].to_string())
}

/// Every profile protection domain and every channel end must resolve.
/// Returned references follow template order, not profile order.
pub(crate) fn select_components<'a>(
    set: &'a ComponentSet,
    pds: &[String],
    channels: &[ChannelSpec],
) -> Result<Vec<&'a Component>> {
    let mut selected_names = BTreeSet::new();
    for pd in pds {
        let component = set.resolve(pd)?;
        selected_names.insert(component.name.0.clone());
    }
    for channel in channels {
        for end in &channel.ends {
            set.resolve(&end.pd)?;
        }
    }
    Ok(set
        .order
        .iter()
        .filter(|component| selected_names.contains(&component.name.0))
        .collect())
}

/// Drops a protection domain only when a package fragment names it and the
/// profile does not select it. Fixed template protection domains stay, so
/// echo and browser keep the protection domains their templates already
/// define. A dropped element is removed by its exact text. When nothing is
/// dropped, those bytes stay identical and channels are spliced as before.
pub(crate) fn compose_system(
    body: &str,
    set: &ComponentSet,
    selected: &[&Component],
    optional: &BTreeSet<String>,
    channels: &[ChannelSpec],
) -> Result<String> {
    let selected_names: BTreeSet<&str> = selected
        .iter()
        .map(|component| component.name.0.as_str())
        .collect();
    let mut composed = body.to_string();
    for component in &set.order {
        let name = &component.name.0;
        let keep = !optional.contains(name) || selected_names.contains(name.as_str());
        if keep {
            continue;
        }
        let Some(at) = composed.find(&component.xml) else {
            bail!("missing component \"{name}\"");
        };
        composed.replace_range(at..at + component.xml.len(), "");
    }
    if channels.is_empty() {
        return Ok(composed);
    }
    replace_channels_in_system(&composed, channels, &set.sdf_names())
}

fn compose_profile_body(_root: &Path, body: &str, profile: &Profile) -> Result<String> {
    let optional = BTreeSet::new();
    let set = ComponentSet::parse(body)?;
    let selected = select_components(&set, &profile.pds, &profile.channel)?;
    compose_system(body, &set, &selected, &optional, &profile.channel)
}

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
///
/// A board with no profile returns the template body and does not load packages.
pub fn render_system(root: &Path, board_name: &str) -> Result<String> {
    let body = render_system_body(root, board_name)?;
    let Ok(profiles) = load_profiles(root) else {
        return Ok(body);
    };
    let Some((_, profile)) = find_profile_for_board(&profiles, board_name) else {
        return Ok(body);
    };
    compose_profile_body(root, &body, profile)
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
    compose_profile_body(root, &body, profile)
}

/// Layout body for one board.
///
/// A device-framework board reads its description from `lerux-image`.
/// `support/boards.toml` `template` is not a file on that path, and
/// `system_vars` are not substituted. Every other board reads its template
/// and substitutes `system_vars`.
pub fn render_system_body(root: &Path, board_name: &str) -> Result<String> {
    let boards = load_boards(root)?;
    let board = get_board(&boards, board_name)?;
    if let Some(rendered) = lerux_image::render_board(board_name)? {
        return Ok(rendered);
    }
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

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
    fn qemu_net_boards_have_no_distinct_client_dma_mr() {
        let root = repo_root();
        for board in [
            "qemu_virt_riscv64_http",
            "x86_64_generic_http",
            "x86_64_generic_virtio",
        ] {
            let sdf = render_system(&root, board).unwrap_or_else(|e| panic!("{board}: {e}"));
            assert!(
                !sdf.contains("name=\"virtio_net_client_dma\""),
                "{board} still declares a distinct virtio_net_client_dma MR"
            );
        }
    }

    #[test]
    fn generate_system_rejects_unknown_component() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let template_dir = root.join("userspace/systems/templates");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(
            template_dir.join("only-shell.system.template"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<system>
    <protection_domain name="shell" priority="1">
        <program_image path="shell.elf" />
    </protection_domain>
</system>
"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("support/profiles")).unwrap();
        std::fs::write(
            root.join("support/boards.toml"),
            r#"
[test_board]
arch = "aarch64"
microkit_board = "qemu_virt_aarch64"
target = "aarch64-sel4-microkit"
template = "only-shell.system.template"
pds = ["shell"]
system_vars = {}
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("support/profiles/only.toml"),
            r#"
template = "only-shell.system.template"
pds = ["nosuch"]
default_board = "test_board"
description = "unknown"
"#,
        )
        .unwrap();
        let output = root.join("out.system");
        std::fs::write(&output, b"SENTINEL").unwrap();
        let err = generate_system(root, "test_board", &output).unwrap_err();
        assert_eq!(err.to_string(), "unknown component \"nosuch\"");
        assert_eq!(std::fs::read(&output).unwrap(), b"SENTINEL");
    }

    #[test]
    fn sdf_diff_identical() {
        let s = "<system>\n</system>\n";
        let summary = sdf_diff_summary("a", s, "b", s);
        assert!(summary.contains("identical"));
    }
}
