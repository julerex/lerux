#![allow(clippy::unwrap_used)] // the build script can panic

use std::{env, path::Path, process::Command};

// The build-dep "toml" (and its closure) has been inlined under
// lerux-kernel/src/lerux-toml* (per the zero-cargo-deps request and
// "place under lerux-kernel/src/lerux-*" rule). For the build script
// itself we use a tiny manual parser — the config file is extremely
// simple (one small table of arch -> feature = "always|never|auto").
// This keeps the build script (and the kernel) with literally zero
// cargo dependencies while leaving the full vendored sources in place
// under the lerux-* dirs for reference.

/// Minimal parser for the exact shape used by parse_kconfig.
/// Returns the inner features map for the requested arch (or None).
fn parse_simple_arch_features(s: &str, arch: &str) -> Option<std::collections::BTreeMap<String, String>> {
    let features_section = format!("arch.{arch}.features");
    let mut in_features = false;
    let mut out = std::collections::BTreeMap::new();
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let sec = line[1..line.len() - 1].trim();
            // config.toml.example uses `[arch.x86_64.features]` (dotted table header).
            in_features = sec == features_section;
            continue;
        }
        if in_features {
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim().trim_matches(|c| c == '"' || c == '\'').to_owned();
                let v = v.trim().trim_matches(|c| c == '"' || c == '\'').to_owned();
                if !k.is_empty() {
                    out.insert(k, v);
                }
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_simple_arch_features;

    #[test]
    fn parse_config_toml_example_features() {
        let config = std::fs::read_to_string("config.toml.example").unwrap();
        let features = parse_simple_arch_features(&config, "x86_64").expect("x86_64 features");
        assert_eq!(features.get("smap").map(String::as_str), Some("auto"));
        assert_eq!(features.get("fsgsbase").map(String::as_str), Some("auto"));
        assert_eq!(features.get("xsave").map(String::as_str), Some("auto"));
        assert_eq!(features.get("xsaveopt").map(String::as_str), Some("auto"));
    }

    #[test]
    fn parse_missing_arch_returns_none() {
        let config = std::fs::read_to_string("config.toml.example").unwrap();
        assert!(parse_simple_arch_features(&config, "aarch64").is_none());
    }
}

fn parse_kconfig(arch: &str) -> Option<()> {
    println!("cargo:rerun-if-changed=config.toml");

    assert!(Path::new("config.toml.example").try_exists().unwrap());
    if !Path::new("config.toml").try_exists().unwrap() {
        std::fs::copy("config.toml.example", "config.toml").unwrap();
    }
    let config_str = std::fs::read_to_string("config.toml").unwrap();

    // Use the tiny inlined parser (the full toml/serde family is present
    // under lerux-kernel/src/lerux-toml* as plain sources per the request,
    // but we don't want to compile or depend on that world from the build
    // script or the no_std kernel).
    let altfeatures = parse_simple_arch_features(&config_str, arch)?;

    #[expect(clippy::format_collect)] // TODO: remove once version is bumped
    let features_list = altfeatures
        .keys()
        .map(|feat| format!(", {feat:?}"))
        .collect::<String>();
    println!("cargo::rustc-check-cfg=cfg(cpu_feature_always, values(\"\"{features_list}))");
    println!("cargo::rustc-check-cfg=cfg(cpu_feature_auto, values(\"\"{features_list}))");
    println!("cargo::rustc-check-cfg=cfg(cpu_feature_never, values(\"\"{features_list}))");

    let self_modifying = env::var("CARGO_FEATURE_SELF_MODIFYING").is_ok();

    for (name, value) in &altfeatures {
        let mut choice = value.as_str();
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
            // lerux-kernel/src/arch/x86_shared/trampoline.rs and assembles the PVH stub
            // via core::arch::global_asm! in lerux-kernel/src/arch/x86_shared/pvh_boot.rs
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
