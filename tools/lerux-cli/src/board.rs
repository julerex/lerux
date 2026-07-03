use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use toml::Value as TomlValue;

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Board {
    pub arch: String,
    pub microkit_board: String,
    pub target: String,
    pub target_triple: String,
    pub template: String,
    pub pds: Vec<String>,
    #[serde(default)]
    pub qemu: Option<String>,
    pub system_vars: BTreeMap<String, TomlValue>,
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
        "target" => println!("{}", board.target),
        "target_triple" => println!("{}", board.target_triple),
        "template" => println!("{}", board.template),
        "pds" => println!("{}", board.pds.join(" ")),
        "qemu" => println!("{}", board.qemu.as_deref().unwrap_or("(hardware)")),
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
