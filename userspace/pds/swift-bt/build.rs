use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let support = manifest.join("../../../support");
    let firmware = load(
        &support.join("mt7921-bt.bin"),
        PathBuf::from("/usr/lib/firmware/mediatek/BT_RAM_CODE_MT7961_1_2_hdr.bin.zst"),
    );
    fs::write(out.join("mt7921-bt.bin"), firmware).expect("bluetooth blob");
    println!("cargo:rerun-if-changed=../../../support/mt7921-bt.bin");
    println!(
        "cargo:rerun-if-changed=/usr/lib/firmware/mediatek/BT_RAM_CODE_MT7961_1_2_hdr.bin.zst"
    );
}

fn load(support_file: &std::path::Path, fallback: PathBuf) -> Vec<u8> {
    if let Ok(bytes) = fs::read(support_file)
        && !bytes.is_empty()
    {
        return bytes;
    }
    decompress(&fallback).unwrap_or_default()
}

fn decompress(path: &std::path::Path) -> Option<Vec<u8>> {
    if !path.exists() {
        return None;
    }
    let output = Command::new("zstd")
        .args(["-d", "-c", "--"])
        .arg(path)
        .output()
        .ok()?;
    if output.status.success() && !output.stdout.is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}
