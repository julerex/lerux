use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let support = manifest.join("../../../support");
    copy(&support.join("mt7921-bt.bin"), &out.join("mt7921-bt.bin"));
    println!("cargo:rerun-if-changed=../../../support/mt7921-bt.bin");
}

fn copy(src: &std::path::Path, dest: &std::path::Path) {
    let bytes = fs::read(src).unwrap_or_default();
    fs::write(dest, bytes).expect("bluetooth blob");
}
