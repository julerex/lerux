use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let support = manifest.join("../../../support");
    let wifi = load(&support.join("swift-wifi.txt"), None);
    let ram = load(
        &support.join("mt7921.bin"),
        Some(PathBuf::from(
            "/usr/lib/firmware/mediatek/WIFI_RAM_CODE_MT7961_1.bin.zst",
        )),
    );
    let patch = decompress(&PathBuf::from(
        "/usr/lib/firmware/mediatek/WIFI_MT7961_patch_mcu_1_2_hdr.bin.zst",
    ))
    .unwrap_or_default();
    let patch_1a = decompress(&PathBuf::from(
        "/usr/lib/firmware/mediatek/WIFI_MT7961_patch_mcu_1a_2_hdr.bin.zst",
    ))
    .unwrap_or_default();
    let ram_1a = decompress(&PathBuf::from(
        "/usr/lib/firmware/mediatek/WIFI_RAM_CODE_MT7961_1a.bin.zst",
    ))
    .unwrap_or_default();
    fs::write(out.join("wifi.txt"), wifi).expect("wifi blob");
    fs::write(out.join("mt7921.bin"), ram).expect("wifi blob");
    fs::write(out.join("patch.bin"), patch).expect("wifi blob");
    fs::write(out.join("patch_1a.bin"), patch_1a).expect("wifi blob");
    fs::write(out.join("ram_1a.bin"), ram_1a).expect("wifi blob");
    println!("cargo:rerun-if-changed=../../../support/swift-wifi.txt");
    println!("cargo:rerun-if-changed=../../../support/mt7921.bin");
    println!("cargo:rerun-if-changed=/usr/lib/firmware/mediatek/WIFI_RAM_CODE_MT7961_1.bin.zst");
    println!(
        "cargo:rerun-if-changed=/usr/lib/firmware/mediatek/WIFI_MT7961_patch_mcu_1_2_hdr.bin.zst"
    );
    println!(
        "cargo:rerun-if-changed=/usr/lib/firmware/mediatek/WIFI_MT7961_patch_mcu_1a_2_hdr.bin.zst"
    );
    println!("cargo:rerun-if-changed=/usr/lib/firmware/mediatek/WIFI_RAM_CODE_MT7961_1a.bin.zst");
}

fn load(support_file: &std::path::Path, fallback: Option<PathBuf>) -> Vec<u8> {
    if let Ok(bytes) = fs::read(support_file)
        && !bytes.is_empty()
    {
        return bytes;
    }
    let Some(fallback) = fallback else {
        return Vec::new();
    };
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
