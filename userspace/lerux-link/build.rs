use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let link_dir = manifest_dir.parent().unwrap().join("lerux-entry/link");
    let mut arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    if arch == "x86" {
        arch = "i586".to_owned();
    }
    println!("cargo::rustc-link-arg=-z");
    println!("cargo::rustc-link-arg=max-page-size=4096");
    println!("cargo::rustc-link-arg=-T");
    println!("cargo::rustc-link-arg={}", link_dir.join(format!("{arch}.ld")).display());
}
