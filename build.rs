#![allow(clippy::unwrap_used)] // the build script can panic

use std::{env, path::Path, process::Command};
use toml::Table;

fn parse_kconfig(arch: &str) -> Option<()> {
    println!("cargo:rerun-if-changed=config.toml");

    assert!(Path::new("config.toml.example").try_exists().unwrap());
    if !Path::new("config.toml").try_exists().unwrap() {
        std::fs::copy("config.toml.example", "config.toml").unwrap();
    }
    let config_str = std::fs::read_to_string("config.toml").unwrap();
    let root: Table = toml::from_str(&config_str).unwrap();

    let altfeatures = root
        .get("arch")?
        .as_table()
        .unwrap()
        .get(arch)?
        .as_table()
        .unwrap()
        .get("features")?
        .as_table()
        .unwrap();

    #[expect(clippy::format_collect)] // TODO: remove once version is bumped
    let features_list = altfeatures
        .keys()
        .map(|feat| format!(", {feat:?}"))
        .collect::<String>();
    println!("cargo::rustc-check-cfg=cfg(cpu_feature_always, values(\"\"{features_list}))");
    println!("cargo::rustc-check-cfg=cfg(cpu_feature_auto, values(\"\"{features_list}))");
    println!("cargo::rustc-check-cfg=cfg(cpu_feature_never, values(\"\"{features_list}))");

    let self_modifying = env::var("CARGO_FEATURE_SELF_MODIFYING").is_ok();

    for (name, value) in altfeatures {
        let mut choice = value.as_str().unwrap();
        assert!(matches!(choice, "always" | "never" | "auto"));

        if !self_modifying && choice == "auto" {
            choice = "never";
        }

        println!("cargo:rustc-cfg=cpu_feature_{choice}=\"{name}\"");
    }

    Some(())
}

fn main() {
    println!("cargo::rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo::rustc-check-cfg=cfg(dtb)");

    let arch_str = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    match &*arch_str {
        "aarch64" => {
            println!("cargo::rustc-cfg=dtb");
        }
        "x86" | "x86_64" => {
            // lerux divergence from upstream redox-os/kernel build.rs:
            //
            // Upstream invoked nasm (SMP trampolines from src/asm/*/trampoline.asm)
            // and cc/clang (pvh_boot.S). lerux embeds trampoline bytes in
            // kernel/src/arch/x86_shared/trampoline.rs and assembles the PVH stub
            // via core::arch::global_asm! in kernel/src/arch/x86_shared/pvh_boot.rs
            // (direct-boot feature only). No external assembler or C compiler here.
            //
            // See VENDORED.md at the repo root.
        }
        "riscv64" => {
            println!("cargo::rustc-cfg=dtb");
        }
        _ => (),
    }

    let _ = parse_kconfig(&arch_str);

    if env::var("CARGO_FEATURE_DIRECT_BOOT").is_ok() {
        embed_initfs();
    }
}

/// Copy (or build) `build/initfs.bin` into OUT_DIR for `include_bytes!` in direct-boot.
fn embed_initfs() {
    let manifest_dir_str = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = Path::new(&manifest_dir_str);
    let out_dir = env::var("OUT_DIR").unwrap();
    let initfs_bin = manifest_dir.join("build/initfs.bin");
    let staging = manifest_dir.join("userspace/initfs-staging");
    let bootstrap = staging.join("bootstrap.elf");

    println!("cargo:rerun-if-changed=build/initfs.bin");
    println!("cargo:rerun-if-changed=userspace/initfs-staging");

    if !initfs_bin.exists() {
        assert!(
            bootstrap.exists(),
            "build/initfs.bin missing and {bootstrap:?} not found; run `just build-initfs` first"
        );
        std::fs::create_dir_all(manifest_dir.join("build")).unwrap();
        let status = Command::new("cargo")
            .args([
                "run",
                "--release",
                "--manifest-path",
                "userspace/initfs-tools/Cargo.toml",
                "--bin",
                "redox-initfs-ar",
                "--",
                staging.to_str().unwrap(),
                bootstrap.to_str().unwrap(),
                "-o",
                "build/initfs.bin",
            ])
            .current_dir(&manifest_dir)
            .status()
            .expect("failed to spawn initfs archiver");
        assert!(status.success(), "initfs archiver failed");
    }

    let dest = Path::new(&out_dir).join("initfs.bin");
    std::fs::copy(&initfs_bin, &dest).expect("failed to copy initfs.bin to OUT_DIR");

    let embed_rs = format!(
        "pub static INITFS_BLOB: &[u8] = include_bytes!(r\"{}\");\n",
        dest.display()
    );
    std::fs::write(Path::new(&out_dir).join("initfs_embed.rs"), embed_rs)
        .expect("failed to write initfs_embed.rs");
}
