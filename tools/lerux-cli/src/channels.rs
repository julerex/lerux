//! Structured Microkit channel manifests (Phase 41).
//!
//! Profiles and package fragments describe IPC topology as typed ends rather
//! than free-text strings. Hardware IRQs and MMIO stay in `boards.toml`; this
//! module only owns channel IDs and `pp` flags.
//!
//! PD names here are **SDF names** (underscores), matching
//! `<protection_domain name="…">`. Cargo crate names use hyphens; use
//! [`to_sdf_pd_name`] when comparing to `profile.pds`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// One end of a Microkit `<channel>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelEnd {
    /// SDF protection-domain name (`serial_driver`, not `serial-driver`).
    pub pd: String,
    /// Channel id local to that PD (`Channel::new(id)` in Rust).
    pub id: u8,
    /// Protected procedure call on this end (`pp="true"` in SDF).
    #[serde(default)]
    pub pp: bool,
}

/// One bidirectional Microkit channel (exactly two ends).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelSpec {
    /// Optional stable name for docs / diffs (e.g. `shell_serial`).
    #[serde(default)]
    pub name: Option<String>,
    /// Exactly two ends after validation.
    pub ends: Vec<ChannelEnd>,
}

impl ChannelSpec {
    /// Human-readable form: `serial_driver:1 <-> shell:0 pp`.
    pub fn display_line(&self) -> String {
        let body = self
            .ends
            .iter()
            .map(|e| {
                if e.pp {
                    format!("{}:{} pp", e.pd, e.id)
                } else {
                    format!("{}:{}", e.pd, e.id)
                }
            })
            .collect::<Vec<_>>()
            .join(" <-> ");
        match &self.name {
            Some(n) if !n.is_empty() => format!("{n}: {body}"),
            _ => body,
        }
    }

    /// Emit a Microkit `<channel>` XML fragment (four-space indent, trailing newline).
    pub fn to_xml(&self) -> String {
        let mut out = String::from("    <channel>\n");
        for end in &self.ends {
            if end.pp {
                out.push_str(&format!(
                    "        <end pd=\"{}\" id=\"{}\" pp=\"true\" />\n",
                    end.pd, end.id
                ));
            } else {
                out.push_str(&format!(
                    "        <end pd=\"{}\" id=\"{}\" />\n",
                    end.pd, end.id
                ));
            }
        }
        out.push_str("    </channel>\n");
        out
    }
}

/// Render all channels as SDF fragments separated by a blank line.
pub fn render_channels_xml(channels: &[ChannelSpec]) -> String {
    channels
        .iter()
        .map(ChannelSpec::to_xml)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip every `<channel>…</channel>` block (and trailing whitespace after each).
pub fn strip_channel_blocks(system_xml: &str) -> String {
    // Non-greedy across newlines; Microkit channel blocks do not nest.
    let re = regex::Regex::new(r"(?s)<channel>.*?</channel>\s*").expect("channel strip regex");
    re.replace_all(system_xml, "").into_owned()
}

/// Replace all channel blocks in a Microkit system description with `channels`.
///
/// Keeps memory regions and protection domains from the template; inserts
/// generated channels immediately before `</system>`.
pub fn replace_channels_in_system(system_xml: &str, channels: &[ChannelSpec]) -> Result<String> {
    if channels.is_empty() {
        bail!("replace_channels_in_system: channel list is empty");
    }
    ensure_channels_valid(channels, &[], "replace_channels_in_system")?;

    let without = strip_channel_blocks(system_xml);
    let Some(close_idx) = without.rfind("</system>") else {
        bail!("system XML missing </system>");
    };
    let before = without[..close_idx].trim_end();
    let channels_xml = render_channels_xml(channels);
    Ok(format!("{before}\n\n{channels_xml}</system>\n"))
}

/// Extract only the concatenated channel blocks from a system description
/// (normalized via strip + re-render is not applied; raw blocks joined).
pub fn extract_channel_blocks(system_xml: &str) -> String {
    let re = regex::Regex::new(r"(?s)<channel>.*?</channel>").expect("channel extract regex");
    let mut out = String::new();
    for (i, m) in re.find_iter(system_xml).enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // Re-indent consistently: match groups as they appear (templates use 4 spaces).
        let block = m.as_str().trim();
        // Templates indent the opening tag with 4 spaces; find_iter may not include them.
        if block.starts_with("<channel>") {
            out.push_str("    ");
        }
        out.push_str(block);
        out.push('\n');
    }
    out
}

impl fmt::Display for ChannelSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_line())
    }
}

/// Normalize Cargo crate / profile PD names to SDF `protection_domain` names.
pub fn to_sdf_pd_name(name: &str) -> String {
    name.replace('-', "_")
}

/// Result of validating a channel list.
#[derive(Debug, Clone, Default)]
pub struct ChannelValidation {
    /// `(pd, id)` pairs that appeared more than once.
    pub duplicate_ends: Vec<(String, u8)>,
    /// SDF PD names referenced by channels but not in the allowed set.
    pub unknown_pds: BTreeSet<String>,
    /// Channels that did not have exactly two ends.
    pub bad_arity: Vec<String>,
}

impl ChannelValidation {
    pub fn ok(&self) -> bool {
        self.duplicate_ends.is_empty() && self.unknown_pds.is_empty() && self.bad_arity.is_empty()
    }

    pub fn into_result(self, context: &str) -> Result<()> {
        if self.ok() {
            return Ok(());
        }
        let mut parts = Vec::new();
        for ch in &self.bad_arity {
            parts.push(format!("channel must have exactly 2 ends: {ch}"));
        }
        for (pd, id) in &self.duplicate_ends {
            parts.push(format!("duplicate channel end {pd}:{id}"));
        }
        for pd in &self.unknown_pds {
            parts.push(format!("channel references unknown PD {pd:?}"));
        }
        bail!("{context}: {}", parts.join("; "))
    }
}

/// Validate channel topology.
///
/// * Each channel must have exactly two ends.
/// * `(pd, id)` pairs must be unique across all ends (IRQ id 0 on drivers is
///   still free for device channels, which are not listed here).
/// * If `allowed_pds` is non-empty, every end PD must match an allowed name
///   (hyphen/underscore insensitive).
pub fn validate_channels(channels: &[ChannelSpec], allowed_pds: &[String]) -> ChannelValidation {
    let mut report = ChannelValidation::default();
    let allowed: BTreeSet<String> = allowed_pds.iter().map(|p| to_sdf_pd_name(p)).collect();

    let mut seen: BTreeMap<(String, u8), usize> = BTreeMap::new();

    for (i, ch) in channels.iter().enumerate() {
        let label = ch
            .name
            .clone()
            .unwrap_or_else(|| format!("#{} ({})", i, ch.display_line()));

        if ch.ends.len() != 2 {
            report.bad_arity.push(label);
            continue;
        }

        for end in &ch.ends {
            let pd = to_sdf_pd_name(&end.pd);
            if !allowed.is_empty() && !allowed.contains(&pd) {
                report.unknown_pds.insert(pd.clone());
            }
            let key = (pd, end.id);
            *seen.entry(key).or_insert(0) += 1;
        }
    }

    for ((pd, id), count) in seen {
        if count > 1 {
            report.duplicate_ends.push((pd, id));
        }
    }
    report.duplicate_ends.sort();
    report
}

/// Validate and return `Err` with context if invalid.
pub fn ensure_channels_valid(
    channels: &[ChannelSpec],
    allowed_pds: &[String],
    context: &str,
) -> Result<()> {
    validate_channels(channels, allowed_pds).into_result(context)
}

/// Parse a legacy free-text line like `serial_driver:1 <-> shell:0 (pp)`.
///
/// Best-effort: `(pp)` is applied to the **second** end only (historical
/// convention in docs). Prefer structured `[[channel]]` tables.
#[allow(dead_code)] // migration helper + unit tests
pub fn parse_legacy_channel_line(line: &str) -> Result<ChannelSpec> {
    let line = line.trim();
    let (body, trailing_pp) = if let Some(stripped) = line.strip_suffix("(pp)") {
        (stripped.trim(), true)
    } else {
        (line, false)
    };

    let parts: Vec<&str> = body.split("<->").map(str::trim).collect();
    if parts.len() != 2 {
        bail!("legacy channel line must have two ends separated by <->: {line:?}");
    }

    let mut ends = Vec::with_capacity(2);
    for (i, part) in parts.iter().enumerate() {
        let (pd, id) =
            parse_end_token(part).with_context(|| format!("parse end {part:?} in {line:?}"))?;
        let pp = trailing_pp && i == 1;
        ends.push(ChannelEnd { pd, id, pp });
    }
    Ok(ChannelSpec { name: None, ends })
}

fn parse_end_token(token: &str) -> Result<(String, u8)> {
    let token = token.trim();
    if let Some((pd, id_s)) = token.rsplit_once(':') {
        let id: u8 = id_s
            .trim()
            .parse()
            .with_context(|| format!("channel id in {token:?}"))?;
        Ok((to_sdf_pd_name(pd.trim()), id))
    } else {
        // Incomplete legacy form without ids — reject for structured path.
        bail!("legacy end missing :id (use structured [[channel]]): {token:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end(pd: &str, id: u8, pp: bool) -> ChannelEnd {
        ChannelEnd {
            pd: pd.into(),
            id,
            pp,
        }
    }

    fn ch(a: ChannelEnd, b: ChannelEnd) -> ChannelSpec {
        ChannelSpec {
            name: None,
            ends: vec![a, b],
        }
    }

    #[test]
    fn accepts_unique_ends() {
        let channels = vec![ch(end("serial_driver", 1, false), end("shell", 0, true))];
        let pds = vec!["serial-driver".into(), "shell".into()];
        assert!(validate_channels(&channels, &pds).ok());
    }

    #[test]
    fn rejects_duplicate_end() {
        let channels = vec![
            ch(end("serial_driver", 1, false), end("shell", 0, true)),
            ch(end("serial_driver", 1, false), end("supervisor", 0, true)),
        ];
        let pds = vec!["serial-driver".into(), "shell".into(), "supervisor".into()];
        let v = validate_channels(&channels, &pds);
        assert!(!v.ok());
        assert_eq!(v.duplicate_ends, vec![("serial_driver".into(), 1)]);
    }

    #[test]
    fn rejects_unknown_pd() {
        let channels = vec![ch(end("serial_driver", 1, false), end("ghost", 0, true))];
        let pds = vec!["serial-driver".into()];
        let v = validate_channels(&channels, &pds);
        assert!(v.unknown_pds.contains("ghost"));
    }

    #[test]
    fn rejects_wrong_arity() {
        let channels = vec![ChannelSpec {
            name: Some("broken".into()),
            ends: vec![end("shell", 0, false)],
        }];
        let v = validate_channels(&channels, &[]);
        assert_eq!(v.bad_arity, vec!["broken".to_string()]);
    }

    #[test]
    fn xml_roundtrip_shape() {
        let c = ch(end("serial_driver", 1, false), end("shell", 0, true));
        let xml = c.to_xml();
        assert!(xml.contains("pd=\"serial_driver\" id=\"1\""));
        assert!(xml.contains("pd=\"shell\" id=\"0\" pp=\"true\""));
    }

    #[test]
    fn replace_channels_preserves_pds() {
        let template = r#"<?xml version="1.0" encoding="UTF-8"?>
<system>
    <protection_domain name="serial_driver" priority="2" stack_size="0x10_000">
        <program_image path="serial-driver.elf" />
    </protection_domain>

    <protection_domain name="hello" priority="1" stack_size="0x10_000">
        <program_image path="hello.elf" />
    </protection_domain>

    <channel>
        <end pd="serial_driver" id="9" />
        <end pd="hello" id="9" pp="true" />
    </channel>
</system>
"#;
        let channels = vec![ch(end("serial_driver", 1, false), end("hello", 0, true))];
        let out = replace_channels_in_system(template, &channels).unwrap();
        assert!(out.contains("protection_domain name=\"hello\""));
        assert!(out.contains("id=\"1\""));
        assert!(out.contains("id=\"0\" pp=\"true\""));
        assert!(!out.contains("id=\"9\""));
        assert_eq!(out.matches("<channel>").count(), 1);
        assert!(out.trim_end().ends_with("</system>"));
    }

    #[test]
    fn parse_legacy_with_pp() {
        let c = parse_legacy_channel_line("serial_driver:1 <-> shell:0 (pp)").unwrap();
        assert_eq!(c.ends[0].pd, "serial_driver");
        assert_eq!(c.ends[0].id, 1);
        assert!(!c.ends[0].pp);
        assert_eq!(c.ends[1].pd, "shell");
        assert_eq!(c.ends[1].id, 0);
        assert!(c.ends[1].pp);
    }

    #[test]
    fn hyphen_in_pd_normalized() {
        let c = parse_legacy_channel_line("shell:7 <-> chat-client:0 (pp)").unwrap();
        assert_eq!(c.ends[1].pd, "chat_client");
    }
}
