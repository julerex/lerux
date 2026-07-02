use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::board::{format_system_var, get_board, load_boards};

pub fn generate_system(root: &Path, board_name: &str, output: &Path) -> Result<()> {
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

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(output, rendered).with_context(|| format!("write {}", output.display()))?;
    Ok(())
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
