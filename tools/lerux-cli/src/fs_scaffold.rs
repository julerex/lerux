//! Filesystem board scaffold.
//!
//! A copied filesystem board aliases the shell feature of the board it copies.
//! It does not duplicate the timer and filesystem config pages. This command
//! diffs that alias against the shell manifest, the config writer, the just
//! recipe, and the smoke table.

use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

pub(crate) fn report(root: &Path, from: &str, board: &str) -> Result<String> {
    let shell = fs::read_to_string(root.join("userspace/pds/sddf-shell/Cargo.toml"))
        .context("read sddf-shell Cargo.toml")?;
    let writer = fs::read_to_string(root.join("tools/lerux-cli/src/fs_sddf.rs"))
        .context("read fs_sddf.rs")?;
    let justfile = fs::read_to_string(root.join("justfile")).context("read justfile")?;
    let smoke = fs::read_to_string(root.join("support/smoke-expects.toml"))
        .context("read smoke-expects.toml")?;
    match scaffold_diff(
        from,
        board,
        ScaffoldFiles {
            shell: &shell,
            writer: &writer,
            justfile: &justfile,
            smoke: &smoke,
        },
    ) {
        Ok(text) => Ok(text),
        Err(text) => bail!("{text}"),
    }
}

pub(crate) struct ScaffoldFiles<'a> {
    pub shell: &'a str,
    pub writer: &'a str,
    pub justfile: &'a str,
    pub smoke: &'a str,
}

pub(crate) fn scaffold_diff(
    from: &str,
    board: &str,
    files: ScaffoldFiles<'_>,
) -> Result<String, String> {
    let feature = format!("board-{from}");
    let alias = format!("board-{board} = [\"{feature}\"]");
    let recipe = format!("BOARD={board} just test");
    let smoke_key = format!("[boards.{board}]");
    let mut missing = Vec::new();
    if !files.shell.lines().any(|line| line.trim() == alias) {
        missing.push(format!("userspace/pds/sddf-shell/Cargo.toml\n+{alias}"));
    }
    if !writer_aliases(&files.writer, board, &feature) {
        missing.push(format!(
            "tools/lerux-cli/src/fs_sddf.rs\n+\"{board}\" => \"{feature}\","
        ));
    }
    if !files.justfile.contains(&recipe) {
        missing.push(format!("justfile\n+{recipe}"));
    }
    if !files.smoke.contains(&smoke_key) {
        missing.push(format!("support/smoke-expects.toml\n+{smoke_key}"));
    }
    if missing.is_empty() {
        return Ok(format!(
            "filesystem scaffold {board} aliases {from}\n  shell feature: {alias}\n  config writer: {board} uses {feature}\n  recipe: {recipe}\n  smoke: {smoke_key}\n"
        ));
    }
    Err(format!(
        "filesystem scaffold {board} does not alias {from}\n{}",
        missing.join("\n")
    ))
}

fn writer_aliases(writer: &str, board: &str, feature: &str) -> bool {
    let needle = format!("\"{board}\"");
    let result = format!("\"{feature}\"");
    writer.lines().any(|line| {
        let Some((pattern, value)) = line.split_once("=>") else {
            return false;
        };
        pattern.contains(&needle) && value.contains(&result)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn ahci_aliases_the_x86_filesystem_shell_feature() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = report(&root, "x86_64_generic_fs_sddf", "x86_64_generic_fs_ahci")
            .expect("ahci scaffold");
        assert!(text.contains("aliases x86_64_generic_fs_sddf"), "{text}");
    }

    #[test]
    fn a_copied_feature_without_the_alias_diffs_the_expected_line() {
        let err = scaffold_diff(
            "x86_64_generic_fs_sddf",
            "x86_64_generic_fs_ahci",
            ScaffoldFiles {
                shell: "board-x86_64_generic_fs_ahci = []\n",
                writer: "\"x86_64_generic_fs_sddf\" => \"board-x86_64_generic_fs_sddf\",\n",
                justfile: "",
                smoke: "",
            },
        )
        .expect_err("missing alias");
        assert!(err.contains("+board-x86_64_generic_fs_ahci = [\"board-x86_64_generic_fs_sddf\"]"));
        assert!(err.contains("fs_sddf.rs"));
        assert!(err.contains("justfile"));
        assert!(err.contains("smoke-expects.toml"));
    }
}
