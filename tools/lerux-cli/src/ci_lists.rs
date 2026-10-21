#![cfg(test)]

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::board::load_boards;

const LOCAL_ONLY_BOARDS: &[&str] = &[
    "qemu_virt_aarch64_debug",
    "qemu_virt_riscv64_debug",
    "x86_64_generic_debug",
];

struct SmokeCensus {
    include: Vec<String>,
    readme_counts: Vec<usize>,
    ci_doc_counts: Vec<usize>,
    ci_true: BTreeSet<String>,
}

fn load_census(root: &Path) -> Result<SmokeCensus> {
    let workflow = fs::read_to_string(root.join(".github/workflows/rust.yml"))
        .context("read .github/workflows/rust.yml")?;
    let readme = fs::read_to_string(root.join("README.md")).context("read README.md")?;
    let ci_doc = fs::read_to_string(root.join("docs/ci.md")).context("read docs/ci.md")?;
    let boards = load_boards(root)?;
    Ok(SmokeCensus {
        include: smoke_include_boards(&workflow)?,
        readme_counts: readme_smoke_counts(&readme)?,
        ci_doc_counts: ci_doc_smoke_counts(&ci_doc)?,
        ci_true: boards
            .into_iter()
            .filter(|(_, board)| board.ci)
            .map(|(name, _)| name)
            .collect(),
    })
}

fn check_census(census: &SmokeCensus) -> Result<()> {
    if census.include.is_empty() {
        bail!("workflow smoke include list is empty");
    }
    let include_len = census.include.len();
    let mut errors = Vec::new();
    push_count_errors(&mut errors, "README.md", &census.readme_counts, include_len);
    push_count_errors(
        &mut errors,
        "docs/ci.md",
        &census.ci_doc_counts,
        include_len,
    );
    let include_set: BTreeSet<&str> = census.include.iter().map(String::as_str).collect();
    for board in &census.include {
        if !census.ci_true.contains(board) {
            errors.push(format!(
                "{board} is in the workflow include list but ci is not true"
            ));
        }
    }
    for board in &census.ci_true {
        if include_set.contains(board.as_str()) || LOCAL_ONLY_BOARDS.contains(&board.as_str()) {
            continue;
        }
        errors.push(format!(
            "{board} has ci = true and is absent from the workflow include list"
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        bail!("{}", errors.join("\n"))
    }
}

fn push_count_errors(errors: &mut Vec<String>, path: &str, counts: &[usize], include_len: usize) {
    if counts.is_empty() {
        errors.push(format!("{path} has no smoke count"));
        return;
    }
    for count in counts {
        if *count != include_len {
            errors.push(format!(
                "{path} count {count} does not match workflow include length {include_len}"
            ));
        }
    }
}

fn smoke_include_boards(workflow: &str) -> Result<Vec<String>> {
    let mut in_smoke = false;
    let mut in_include = false;
    let mut boards = Vec::new();
    for line in workflow.lines() {
        if !in_smoke {
            if line == "  smoke:" {
                in_smoke = true;
            }
            continue;
        }
        if is_job_header(line) {
            break;
        }
        if !in_include {
            if line.trim() == "include:" {
                in_include = true;
            }
            continue;
        }
        if line.trim() == "steps:" {
            break;
        }
        let Some(rest) = line.trim().strip_prefix("board:") else {
            continue;
        };
        let name = rest.trim();
        if name.is_empty() || name.contains([' ', '"', '\'']) {
            bail!("smoke include has an unreadable board line: {line}");
        }
        boards.push(name.to_string());
    }
    if !in_smoke {
        bail!("workflow has no smoke job");
    }
    if boards.is_empty() {
        bail!("smoke job include list is empty");
    }
    Ok(boards)
}

fn is_job_header(line: &str) -> bool {
    let Some(name) = line.strip_prefix("  ") else {
        return false;
    };
    if name.starts_with(' ') {
        return false;
    }
    let Some(name) = name.strip_suffix(':') else {
        return false;
    };
    !name.is_empty() && !name.contains([' ', ':'])
}

fn readme_smoke_counts(text: &str) -> Result<Vec<usize>> {
    counts_before(text, " smoke**", "README.md")
}

fn ci_doc_smoke_counts(text: &str) -> Result<Vec<usize>> {
    let mut counts = counts_before(text, " parallel matrix jobs", "docs/ci.md")?;
    counts.extend(counts_after(text, "smoke matrix x", "docs/ci.md")?);
    Ok(counts)
}

fn counts_before(text: &str, suffix: &str, path: &str) -> Result<Vec<usize>> {
    let mut counts = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find(suffix) {
        let digits = digits_before(&rest[..index]);
        if digits.is_empty() {
            bail!("{path} is missing a smoke count");
        }
        counts.push(parse_count(path, digits)?);
        rest = &rest[index + suffix.len()..];
    }
    if counts.is_empty() {
        bail!("{path} has no smoke count");
    }
    Ok(counts)
}

fn counts_after(text: &str, prefix: &str, path: &str) -> Result<Vec<usize>> {
    let mut counts = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find(prefix) {
        let after = &rest[index + prefix.len()..];
        let digits = digits_after(after);
        if digits.is_empty() {
            bail!("{path} is missing a smoke count");
        }
        counts.push(parse_count(path, digits)?);
        rest = &after[digits.len()..];
    }
    if counts.is_empty() {
        bail!("{path} has no smoke count");
    }
    Ok(counts)
}

fn digits_before(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    &text[start..]
}

fn digits_after(text: &str) -> &str {
    let end = text
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(text.len());
    &text[..end]
}

fn parse_count(path: &str, digits: &str) -> Result<usize> {
    digits
        .parse()
        .with_context(|| format!("{path} smoke count {digits}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn include_list_matches_docs() {
        let census = load_census(&root()).expect("load smoke lists");
        let include_len = census.include.len();
        assert!(
            census
                .ci_doc_counts
                .iter()
                .all(|count| *count == include_len),
            "docs/ci.md counts {:?} vs include {include_len}",
            census.ci_doc_counts
        );
        assert!(
            census
                .readme_counts
                .iter()
                .all(|count| *count == include_len),
            "README.md counts {:?} vs include {include_len}",
            census.readme_counts
        );
        check_census(&census).expect("smoke lists match");
    }

    #[test]
    fn readme_count_52_fails_and_names_readme() {
        let repo = root();
        let mut census = load_census(&repo).expect("load smoke lists");
        let readme = fs::read_to_string(repo.join("README.md")).expect("read README.md");
        let include_len = census.include.len();
        let wrong = include_len.saturating_sub(1);
        let readme = readme.replacen(
            &format!("**{include_len} smoke**"),
            &format!("**{wrong} smoke**"),
            1,
        );
        assert!(
            readme.contains(&format!("**{wrong} smoke**")),
            "README.md fixture did not adjust the smoke count"
        );
        census.readme_counts = readme_smoke_counts(&readme).expect("parse README.md fixture");
        assert_eq!(census.readme_counts, vec![wrong]);
        let err = check_census(&census).expect_err("README.md count mismatch");
        assert_eq!(
            err.to_string(),
            format!("README.md count {wrong} does not match workflow include length {include_len}")
        );
    }

    #[test]
    fn workflow_board_missing_ci_names_the_board() {
        let mut census = load_census(&root()).expect("load smoke lists");
        let board = "qemu_virt_aarch64";
        assert!(
            census.include.iter().any(|name| name == board),
            "{board} is not in the workflow include list"
        );
        assert!(census.ci_true.remove(board));
        let err = check_census(&census).expect_err("missing ci");
        assert_eq!(
            err.to_string(),
            "qemu_virt_aarch64 is in the workflow include list but ci is not true"
        );
    }

    #[test]
    fn local_only_boards_stay_legal() {
        let mut census = load_census(&root()).expect("load smoke lists");
        let mut ci_true: BTreeSet<String> = census.include.iter().cloned().collect();
        for name in LOCAL_ONLY_BOARDS {
            assert!(
                !ci_true.contains(*name),
                "{name} is in the workflow include list"
            );
            ci_true.insert((*name).to_string());
        }
        census.ci_true = ci_true;
        check_census(&census).expect("local-only boards stay legal");
    }
}
