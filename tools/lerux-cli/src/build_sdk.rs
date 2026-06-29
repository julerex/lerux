use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::{
    install::{
        install_dtc, install_libclang, install_qemu_aarch64, install_qemu_riscv64,
        install_riscv_toolchain, install_xmllint,
    },
    libclang::apply_libclang_env,
    process::{command_on_path, run_checked, write_file},
};

pub fn build_sdk(root: &Path) -> Result<()> {
    let workspace = root.join("deps/workspace");
    let microkit = workspace.join("microkit");
    let sel4 = workspace.join("seL4");

    if !microkit.is_dir() || !sel4.is_dir() {
        bail!("run `lerux fetch` first");
    }

    let boards = std::env::var("MICROKIT_BOARDS").unwrap_or_else(|_| "qemu_virt_aarch64".into());
    let configs = std::env::var("MICROKIT_CONFIGS").unwrap_or_else(|_| "debug".into());

    let needs_arm = boards_needs(&boards, &["qemu_virt_aarch64", "aarch64"]);
    let needs_riscv = boards_needs(&boards, &["qemu_virt_riscv64", "riscv"]);
    let needs_x86 = boards_needs(&boards, &["x86_64_generic", "x86"]);

    if needs_arm {
        ensure_toolchain("aarch64-none-elf-gcc", || {
            eprintln!("==> Installing ARM GNU toolchain into deps/toolchains/");
            let bin = crate::install::install_arm_toolchain(root)?;
            prepend_path(&bin);
            Ok(())
        })?;
    }

    if needs_riscv {
        if !riscv_gcc_ok() {
            eprintln!("==> Installing RISC-V GNU toolchain into deps/toolchains/");
            let bin = install_riscv_toolchain(root)?;
            prepend_path(&bin);
        }
        if !riscv_gcc_ok() {
            bail!("riscv64-unknown-elf-gcc 13+ not found after install attempt");
        }
    }

    if needs_x86 && !command_on_path("x86_64-linux-gnu-gcc") {
        bail!("x86_64-linux-gnu-gcc required for x86_64 boards");
    }

    if !command_on_path("qemu-system-aarch64") {
        eprintln!("==> Installing QEMU (aarch64) into deps/toolchains/");
        let bin = install_qemu_aarch64(root)?;
        prepend_path(&bin);
    }

    if needs_riscv && !command_on_path("qemu-system-riscv64") {
        eprintln!("==> Installing QEMU (riscv64) into deps/toolchains/");
        let bin = install_qemu_riscv64(root)?;
        prepend_path(&bin);
    }

    if !command_on_path("dtc") {
        eprintln!("==> Installing device-tree-compiler into deps/toolchains/");
        let bin = install_dtc(root)?;
        prepend_path(&bin);
    }

    if !command_on_path("xmllint") {
        eprintln!("==> Installing xmllint into deps/toolchains/");
        let bin = install_xmllint(root)?;
        prepend_path(&bin);
    }

    let mut required = vec!["dtc", "xmllint", "cmake", "ninja", "python3"];
    if needs_arm {
        required.push("qemu-system-aarch64");
    }
    if needs_riscv {
        required.push("qemu-system-riscv64");
    }
    if needs_x86 {
        required.push("qemu-system-x86_64");
    }
    for tool in required {
        if !command_on_path(tool) {
            bail!("{tool} not found in PATH");
        }
    }

    if command_on_path("rustup") {
        let _ = run_checked("rustup", &["target", "add", "x86_64-unknown-linux-musl"]);
        let _ = run_checked(
            "rustup",
            &[
                "toolchain",
                "install",
                "nightly-2026-03-18",
                "-c",
                "rust-src",
            ],
        );
        let _ = run_checked(
            "rustup",
            &[
                "target",
                "add",
                "x86_64-unknown-linux-musl",
                "--toolchain",
                "nightly-2026-03-18",
            ],
        );
        if needs_arm {
            let _ = run_checked("rustup", &["target", "add", "aarch64-unknown-none"]);
            let _ = run_checked(
                "rustup",
                &[
                    "target",
                    "add",
                    "aarch64-unknown-none",
                    "--toolchain",
                    "nightly-2026-03-18",
                ],
            );
        }
        if needs_riscv {
            let _ = run_checked("rustup", &["target", "add", "riscv64gc-unknown-none-elf"]);
            let _ = run_checked(
                "rustup",
                &[
                    "target",
                    "add",
                    "riscv64gc-unknown-none-elf",
                    "--toolchain",
                    "nightly-2026-03-18",
                ],
            );
        }
        if needs_x86 {
            let _ = run_checked("rustup", &["target", "add", "x86_64-unknown-none"]);
            let _ = run_checked(
                "rustup",
                &[
                    "target",
                    "add",
                    "x86_64-unknown-none",
                    "--toolchain",
                    "nightly-2026-03-18",
                ],
            );
        }
    }

    if !crate::install::system_libclang_present() {
        install_libclang(root)?;
    }
    apply_libclang_env(root);

    // SAFETY: host build tooling mutates the current process environment only.
    unsafe {
        std::env::remove_var("CARGO_TARGET_DIR");
    }

    let pyenv = microkit.join("pyenv");
    if !pyenv.is_dir() {
        run_checked("python3", &["-m", "venv", &pyenv.to_string_lossy()])?;
        run_checked(
            pyenv.join("bin/pip").as_os_str(),
            &["install", "--upgrade", "pip", "setuptools", "wheel"],
        )?;
        let pip = pyenv.join("bin/pip");
        let status = std::process::Command::new(&pip)
            .current_dir(&microkit)
            .args(["install", "-r", "requirements.txt"])
            .status()
            .context("pip install microkit requirements")?;
        if !status.success() {
            bail!("pip install microkit requirements failed");
        }
    }

    let status = std::process::Command::new(pyenv.join("bin/python"))
        .current_dir(&microkit)
        .args([
            "build_sdk.py",
            &format!("--sel4={}", sel4.display()),
            "--skip-docs",
            "--skip-tar",
            "--boards",
            &boards,
            "--configs",
            &configs,
        ])
        .status()
        .context("microkit build_sdk.py")?;
    if !status.success() {
        bail!("microkit build_sdk.py failed");
    }

    let release = microkit.join("release");
    let sdk = find_sdk(&release).context("SDK build produced no microkit-sdk-* directory")?;
    write_file(
        &root.join("deps/.sdk-path"),
        &format!("{}\n", sdk.display()),
    )?;
    eprintln!("==> Microkit SDK: {}", sdk.display());
    Ok(())
}

fn boards_needs(boards: &str, patterns: &[&str]) -> bool {
    boards
        .split(',')
        .any(|b| patterns.iter().any(|p| b.contains(p)))
}

fn ensure_toolchain(tool: &str, install: impl FnOnce() -> Result<()>) -> Result<()> {
    if command_on_path(tool) {
        return Ok(());
    }
    install()?;
    if !command_on_path(tool) {
        eprintln!("Run: lerux fetch-sdk  (prebuilt SDK fallback)");
        bail!("{tool} not found after install attempt");
    }
    Ok(())
}

fn prepend_path(bin_dir: &Path) {
    let path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: host build tooling mutates the current process environment only.
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), path));
    }
}

fn riscv_gcc_ok() -> bool {
    if !command_on_path("riscv64-unknown-elf-gcc") {
        return false;
    }
    let output = run_checked("riscv64-unknown-elf-gcc", &["-dumpversion"]).ok();
    output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|v| v.trim().split('.').next().map(|s| s.to_string()))
        .and_then(|s| s.parse::<u32>().ok())
        .map(|major| major >= 13)
        .unwrap_or(false)
}

fn find_sdk(release: &Path) -> Result<std::path::PathBuf> {
    let mut candidates: Vec<_> = std::fs::read_dir(release)
        .with_context(|| format!("read {}", release.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("microkit-sdk-"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .ok_or_else(|| anyhow::anyhow!("no sdk dir"))
}

pub fn sdk_path(root: &Path) -> Result<String> {
    if let Ok(path) = std::env::var("MICROKIT_SDK")
        && !path.is_empty()
    {
        return Ok(path);
    }
    let file = root.join("deps/.sdk-path");
    if file.is_file() {
        return Ok(std::fs::read_to_string(&file)?.trim().to_string());
    }
    bail!("run `lerux build-sdk` or set MICROKIT_SDK")
}
