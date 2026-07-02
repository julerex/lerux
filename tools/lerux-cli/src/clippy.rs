use std::{path::Path, process::Command};

use anyhow::{bail, Context, Result};

use crate::{
    build_sdk::sdk_path,
    libclang::apply_libclang_env,
    process::{ensure_dir, path_str},
};

struct ClippyCrate<'a> {
    name: &'a str,
    feature: Option<&'a str>,
}

struct ClippyArch<'a> {
    id: &'a str,
    microkit_board: &'a str,
    target_triple: &'a str,
    crates: &'a [ClippyCrate<'a>],
}

const AARCH64_CRATES: &[ClippyCrate<'_>] = &[
    ClippyCrate {
        name: "hello",
        feature: Some("board-qemu_virt_aarch64_composed"),
    },
    ClippyCrate {
        name: "serial-driver",
        feature: Some("board-qemu_virt_aarch64_http_composed"),
    },
    ClippyCrate {
        name: "boot-init",
        feature: Some("board-qemu_virt_aarch64_http_composed"),
    },
    ClippyCrate {
        name: "http-server",
        feature: Some("board-qemu_virt_aarch64_http_composed"),
    },
    ClippyCrate {
        name: "virtio-blk-driver",
        feature: Some("board-qemu_virt_aarch64_composed"),
    },
    ClippyCrate {
        name: "virtio-net-driver",
        feature: Some("board-qemu_virt_aarch64_composed"),
    },
    ClippyCrate {
        name: "pl031-driver",
        feature: None,
    },
    ClippyCrate {
        name: "sp804-driver",
        feature: None,
    },
    ClippyCrate {
        name: "echo-server",
        feature: Some("board-qemu_virt_aarch64_echo"),
    },
    ClippyCrate {
        name: "echo-client",
        feature: None,
    },
    ClippyCrate {
        name: "blk-server",
        feature: Some("board-qemu_virt_aarch64_blk"),
    },
    ClippyCrate {
        name: "blk-client",
        feature: None,
    },
    ClippyCrate {
        name: "net-server",
        feature: Some("board-qemu_virt_aarch64_net"),
    },
    ClippyCrate {
        name: "net-server",
        feature: Some("board-qemu_virt_aarch64_fetch"),
    },
    ClippyCrate {
        name: "fetch-client",
        feature: None,
    },
    ClippyCrate {
        name: "net-client",
        feature: None,
    },
    ClippyCrate {
        name: "net-server",
        feature: Some("board-qemu_virt_aarch64_net_composed"),
    },
    ClippyCrate {
        name: "net-client",
        feature: Some("board-qemu_virt_aarch64_net_composed"),
    },
    ClippyCrate {
        name: "serial-driver",
        feature: Some("board-qemu_virt_aarch64_ipc_composed"),
    },
    ClippyCrate {
        name: "boot-init",
        feature: Some("board-qemu_virt_aarch64_ipc_composed"),
    },
    ClippyCrate {
        name: "blk-client",
        feature: Some("board-qemu_virt_aarch64_ipc_composed"),
    },
    ClippyCrate {
        name: "net-client",
        feature: Some("board-qemu_virt_aarch64_ipc_composed"),
    },
];

const RISCV64_CRATES: &[ClippyCrate<'_>] = &[
    ClippyCrate {
        name: "hello",
        feature: Some("board-qemu_virt_riscv64_virtio"),
    },
    ClippyCrate {
        name: "serial-driver",
        feature: Some("board-qemu_virt_riscv64_http"),
    },
    ClippyCrate {
        name: "http-server",
        feature: Some("board-qemu_virt_riscv64_http"),
    },
    ClippyCrate {
        name: "virtio-blk-driver",
        feature: Some("board-qemu_virt_riscv64_virtio"),
    },
    ClippyCrate {
        name: "virtio-net-driver",
        feature: Some("board-qemu_virt_riscv64_virtio"),
    },
    ClippyCrate {
        name: "virtio-net-driver",
        feature: Some("board-qemu_virt_riscv64_net"),
    },
    ClippyCrate {
        name: "blk-server",
        feature: Some("board-qemu_virt_riscv64_blk"),
    },
    ClippyCrate {
        name: "blk-client",
        feature: None,
    },
    ClippyCrate {
        name: "net-server",
        feature: Some("board-qemu_virt_riscv64_net"),
    },
    ClippyCrate {
        name: "net-client",
        feature: None,
    },
];

const X86_64_CRATES: &[ClippyCrate<'_>] = &[
    ClippyCrate {
        name: "hello",
        feature: Some("board-x86_64_generic_virtio"),
    },
    ClippyCrate {
        name: "serial-driver",
        feature: Some("board-x86_64_generic_virtio"),
    },
    ClippyCrate {
        name: "virtio-pci-driver",
        feature: Some("board-x86_64_generic_virtio"),
    },
    ClippyCrate {
        name: "virtio-pci-driver",
        feature: Some("board-x86_64_generic_blk"),
    },
    ClippyCrate {
        name: "virtio-pci-driver",
        feature: Some("board-x86_64_generic_net"),
    },
    ClippyCrate {
        name: "virtio-blk-driver",
        feature: Some("board-x86_64_generic_virtio"),
    },
    ClippyCrate {
        name: "virtio-net-driver",
        feature: Some("board-x86_64_generic_virtio"),
    },
    ClippyCrate {
        name: "http-server",
        feature: Some("board-x86_64_generic_http"),
    },
    ClippyCrate {
        name: "blk-server",
        feature: Some("board-x86_64_generic_blk"),
    },
    ClippyCrate {
        name: "blk-client",
        feature: None,
    },
    ClippyCrate {
        name: "net-server",
        feature: Some("board-x86_64_generic_net"),
    },
    ClippyCrate {
        name: "net-client",
        feature: None,
    },
];

const ARCH_PROFILES: &[ClippyArch<'_>] = &[
    ClippyArch {
        id: "aarch64",
        microkit_board: "qemu_virt_aarch64",
        target_triple: "aarch64-sel4-microkit",
        crates: AARCH64_CRATES,
    },
    ClippyArch {
        id: "riscv64",
        microkit_board: "qemu_virt_riscv64",
        target_triple: "riscv64-sel4-microkit",
        crates: RISCV64_CRATES,
    },
    ClippyArch {
        id: "x86_64",
        microkit_board: "x86_64_generic",
        target_triple: "x86_64-sel4-microkit",
        crates: X86_64_CRATES,
    },
];

pub fn clippy_workspace(root: &Path, build_dir: &str, config: &str) -> Result<()> {
    let sdk = sdk_path(root)?;
    apply_libclang_env(root);

    for profile in ARCH_PROFILES {
        eprintln!("==> clippy ({})", profile.id);
        clippy_arch(root, build_dir, config, &sdk, profile)?;
    }

    Ok(())
}

fn clippy_arch(
    root: &Path,
    build_dir: &str,
    config: &str,
    sdk: &str,
    profile: &ClippyArch<'_>,
) -> Result<()> {
    let target_spec = root
        .join("support/targets")
        .join(format!("{}.json", profile.target_triple));
    let target_dir = root
        .join(build_dir)
        .join("clippy")
        .join(profile.id)
        .join("target");
    ensure_dir(&target_dir)?;

    let include = format!("{sdk}/board/{}/{config}/include", profile.microkit_board);

    for crate_spec in profile.crates {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root);
        cmd.arg("clippy")
            .arg("-p")
            .arg(crate_spec.name)
            .arg("--target-dir")
            .arg(path_str(&target_dir))
            .arg("--target")
            .arg(path_str(&target_spec))
            .args([
                "-Z",
                "json-target-spec",
                "-Z",
                "build-std=core,alloc,compiler_builtins",
                "-Z",
                "build-std-features=compiler-builtins-mem",
            ]);

        if let Some(feature) = crate_spec.feature {
            cmd.arg("--features").arg(feature);
        }

        cmd.args(["--", "-D", "warnings"]);

        cmd.env("SEL4_INCLUDE_DIRS", &include);
        cmd.env("RUST_TARGET_PATH", root.join("support/targets"));
        cmd.env("RUSTC_BOOTSTRAP", "1");

        let status = cmd
            .status()
            .with_context(|| format!("cargo clippy -p {}", crate_spec.name))?;
        if !status.success() {
            bail!(
                "cargo clippy -p {} ({}) failed",
                crate_spec.name,
                profile.id
            );
        }
    }

    Ok(())
}
