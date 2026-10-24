//! The SDK job builds `lerux-cli` inside `docker/Dockerfile`.
//!
//! Cargo reads every workspace member before that build. A member whose
//! directory is absent from the image fails the job before the SDK exists.

#![cfg(test)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

fn workspace_members(text: &str) -> Result<Vec<String>> {
    let table: toml::Table = text.parse().context("parse Cargo.toml")?;
    let members = table
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(|members| members.as_array())
        .context("Cargo.toml workspace.members")?;
    let mut paths = Vec::new();
    for member in members {
        let Some(path) = member.as_str() else {
            bail!("workspace member is not a string");
        };
        if path.is_empty() {
            bail!("workspace member path is empty");
        }
        paths.push(path.to_string());
    }
    if paths.is_empty() {
        bail!("workspace has no members");
    }
    Ok(paths)
}

fn copy_sources(dockerfile: &str) -> Result<Vec<String>> {
    let mut sources = Vec::new();
    for (index, line) in dockerfile.lines().enumerate() {
        let line = strip_inline_comment(line).trim();
        let Some(rest) = line.strip_prefix("COPY") else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            bail!(
                "docker/Dockerfile line {} has a COPY with no argument",
                index + 1
            );
        }
        let rest = rest.trim();
        if rest.starts_with('[') {
            bail!("docker/Dockerfile line {} uses a JSON COPY", index + 1);
        }
        let mut tokens: Vec<&str> = rest
            .split_whitespace()
            .filter(|token| !token.starts_with("--"))
            .collect();
        if tokens.len() < 2 {
            bail!(
                "docker/Dockerfile line {} has a COPY without a destination",
                index + 1
            );
        }
        tokens.pop();
        sources.extend(tokens.into_iter().map(str::to_string));
    }
    if sources.is_empty() {
        bail!("docker/Dockerfile has no COPY");
    }
    Ok(sources)
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once(" #")
        .map(|(code, _comment)| code)
        .unwrap_or(line)
}

fn copy_covers(source: &str, member: &str) -> bool {
    source == member
        || member
            .strip_prefix(source)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn dev_image_covers_workspace(members: &[String], copies: &[String]) -> Result<()> {
    let missing: Vec<&str> = members
        .iter()
        .filter(|member| !copies.iter().any(|source| copy_covers(source, member)))
        .map(String::as_str)
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        bail!(
            "docker/Dockerfile does not copy workspace members: {}",
            missing.join(", ")
        )
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_image_lists(root: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let cargo = fs::read_to_string(root.join("Cargo.toml")).context("read Cargo.toml")?;
    let dockerfile =
        fs::read_to_string(root.join("docker/Dockerfile")).context("read docker/Dockerfile")?;
    Ok((workspace_members(&cargo)?, copy_sources(&dockerfile)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_image_copies_every_workspace_member() {
        let (members, copies) = load_image_lists(&root()).expect("load workspace and Dockerfile");
        dev_image_covers_workspace(&members, &copies).expect("dev image copies every member");
    }

    #[test]
    fn missing_ladybird_copy_names_the_member() {
        let members = vec!["lerux-ladybird/crates/lerux-ladybird-url".to_string()];
        let copies = vec!["userspace/crates".to_string()];
        let err = dev_image_covers_workspace(&members, &copies).expect_err("missing copy");
        assert_eq!(
            err.to_string(),
            "docker/Dockerfile does not copy workspace members: lerux-ladybird/crates/lerux-ladybird-url"
        );
    }

    #[test]
    fn parent_copy_covers_a_member_and_a_shorter_prefix_does_not() {
        let members = vec!["userspace/pds/hello".to_string()];
        dev_image_covers_workspace(&members, &["userspace/pds".to_string()])
            .expect("parent directory covers the member");
        let err = dev_image_covers_workspace(&members, &["userspace/pd".to_string()])
            .expect_err("shorter prefix");
        assert!(
            err.to_string().contains("userspace/pds/hello"),
            "error did not name the member: {err}"
        );
    }
}
