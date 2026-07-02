mod deb;
mod tarball;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::process::{self, binary_parent, command_on_path, download, ensure_dir, run_checked};

pub use deb::install_deb_tool;
pub use tarball::extract_tar_xz;

pub fn toolchains_dir(root: &Path) -> PathBuf {
    root.join("deps/toolchains")
}

pub fn install_arm_toolchain(root: &Path) -> Result<PathBuf> {
    if let Ok(path) = which::which("aarch64-none-elf-gcc") {
        eprintln!(
            "==> aarch64-none-elf-gcc already on PATH: {}",
            path.display()
        );
        return binary_parent(&path);
    }

    let toolchains = toolchains_dir(root);
    if let Some(install_dir) = find_dir(&toolchains, "arm-gnu-toolchain-*-aarch64-none-elf") {
        let bin = install_dir.join("bin/aarch64-none-elf-gcc");
        if bin.is_file() {
            eprintln!(
                "==> ARM toolchain already installed at {}",
                install_dir.display()
            );
            return Ok(install_dir.join("bin"));
        }
    }

    let url = "https://developer.arm.com/-/media/Files/downloads/gnu/12.2.rel1/binrel/arm-gnu-toolchain-12.2.rel1-x86_64-aarch64-none-elf.tar.xz";
    let tmp = tempfile::NamedTempFile::new().context("temp file")?;
    download(url, tmp.path())?;
    extract_tar_xz(tmp.path(), &toolchains)?;

    let install_dir = find_dir(&toolchains, "arm-gnu-toolchain-*-aarch64-none-elf")
        .context("ARM toolchain install failed")?;
    let gcc = install_dir.join("bin/aarch64-none-elf-gcc");
    if !gcc.is_file() {
        bail!("ARM toolchain install failed");
    }
    Ok(install_dir.join("bin"))
}

pub fn install_riscv_toolchain(root: &Path) -> Result<PathBuf> {
    if command_on_path("riscv64-unknown-elf-gcc") {
        let version = riscv_gcc_major_version()?;
        if version >= 13 {
            let path = which::which("riscv64-unknown-elf-gcc")?;
            eprintln!(
                "==> riscv64-unknown-elf-gcc already on PATH: {}",
                path.display()
            );
            return binary_parent(&path);
        }
    }

    let toolchains = toolchains_dir(root);
    let wrapper_dir = toolchains.join("riscv64-unknown-elf/bin");
    let wrapper_gcc = wrapper_dir.join("riscv64-unknown-elf-gcc");
    if wrapper_gcc.is_file() {
        eprintln!(
            "==> RISC-V toolchain already installed under {}",
            toolchains.display()
        );
        return Ok(wrapper_dir);
    }

    let xpack_url = "https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v13.2.0-2/xpack-riscv-none-elf-gcc-13.2.0-2-linux-x64.tar.gz";
    let tmp = tempfile::NamedTempFile::new().context("temp file")?;
    download(xpack_url, tmp.path())?;
    deb::extract_tar_gz(tmp.path(), &toolchains)?;

    let xpack_dir = find_dir(&toolchains, "xpack-riscv-none-elf-gcc-*")
        .context("xPack RISC-V toolchain install failed")?;
    let xpack_gcc = xpack_dir.join("bin/riscv-none-elf-gcc");
    if !xpack_gcc.is_file() {
        bail!("xPack RISC-V toolchain install failed");
    }

    ensure_dir(&wrapper_dir)?;
    let xpack_leaf = xpack_dir
        .file_name()
        .and_then(|n| n.to_str())
        .context("xPack directory name")?;
    let tools = [
        ("gcc", "-march=rv64imafdc -mabi=lp64d"),
        ("g++", "-march=rv64imafdc -mabi=lp64d"),
        ("cpp", "-march=rv64imafdc -mabi=lp64d"),
        ("as", "-march=rv64imafdc -mabi=lp64d"),
        ("ld", "-m elf64lriscv"),
        ("ar", ""),
        ("nm", ""),
        ("objcopy", ""),
        ("objdump", ""),
        ("ranlib", ""),
        ("strip", ""),
    ];
    for (tool, extra) in tools {
        let wrapper = wrapper_dir.join(format!("riscv64-unknown-elf-{tool}"));
        let script = format!(
            concat!(
                "#!/usr/bin/env bash\n",
                "set -euo pipefail\n",
                "toolchains_root=\"$(cd \"$(dirname \"${{BASH_SOURCE[0]}}\")/../..\" && pwd)\"\n",
                "exec \"${{toolchains_root}}/{xpack_leaf}/bin/riscv-none-elf-{tool}\" {extra} \"$@\"\n"
            ),
            xpack_leaf = xpack_leaf,
            tool = tool,
            extra = extra
        );
        std::fs::write(&wrapper, script)?;
        run_checked("chmod", &["+x", &wrapper.to_string_lossy()])?;
    }

    Ok(wrapper_dir)
}

fn riscv_gcc_major_version() -> Result<u32> {
    let output = run_checked("riscv64-unknown-elf-gcc", &["-dumpversion"])?;
    let version = String::from_utf8_lossy(&output.stdout);
    version
        .trim()
        .split('.')
        .next()
        .unwrap_or("0")
        .parse()
        .context("parse riscv gcc version")
}

pub fn install_qemu_aarch64(root: &Path) -> Result<PathBuf> {
    install_qemu_deb(
        root,
        "qemu-system-aarch64",
        "qemu",
        "qemu-system-arm",
        "http://archive.ubuntu.com/ubuntu/pool/main/q/qemu/qemu-system-arm_6.2%2bdfsg-2ubuntu6.31_amd64.deb",
    )
}

pub fn install_qemu_riscv64(root: &Path) -> Result<PathBuf> {
    install_qemu_deb(
        root,
        "qemu-system-riscv64",
        "qemu-misc",
        "qemu-system-misc",
        "http://archive.ubuntu.com/ubuntu/pool/main/q/qemu/qemu-system-misc_6.2%2bdfsg-2ubuntu6.31_amd64.deb",
    )
}

fn install_qemu_deb(
    root: &Path,
    binary: &str,
    install_name: &str,
    apt_package: &str,
    fallback_url: &str,
) -> Result<PathBuf> {
    if let Ok(path) = which::which(binary) {
        eprintln!("==> {binary} already on PATH: {}", path.display());
        return binary_parent(&path);
    }

    let toolchains = toolchains_dir(root);
    let qemu_root = toolchains.join(install_name);
    let qemu_bin = qemu_root.join("usr/bin").join(binary);
    if qemu_bin.is_file() {
        eprintln!("==> QEMU already installed at {}", qemu_root.display());
        return Ok(qemu_root.join("usr/bin"));
    }

    let deb_url = process::apt_deb_url(apt_package, fallback_url);
    install_deb_tool(root, install_name, &deb_url, binary)
}

pub fn install_dtc(root: &Path) -> Result<PathBuf> {
    let fallback = "http://archive.ubuntu.com/ubuntu/pool/main/d/device-tree-compiler/device-tree-compiler_1.6.1-1_amd64.deb";
    let deb_url = process::apt_deb_url("device-tree-compiler", fallback);
    install_deb_tool(root, "dtc", &deb_url, "dtc")
}

pub fn install_xmllint(root: &Path) -> Result<PathBuf> {
    let fallback = "http://archive.ubuntu.com/ubuntu/pool/main/libx/libxml2/libxml2-utils_2.9.13+dfsg-1ubuntu2.11_amd64.deb";
    let deb_url = process::apt_deb_url("libxml2-utils", fallback);
    install_deb_tool(root, "xmllint", &deb_url, "xmllint")
}

pub fn install_libclang(root: &Path) -> Result<()> {
    if system_libclang_present() {
        eprintln!("==> libclang found on system");
        return Ok(());
    }

    let clang_root = toolchains_dir(root).join("libclang");
    let lib = clang_root.join("usr/lib/x86_64-linux-gnu/libclang-14.so.14.0.0");
    if lib.is_file() {
        eprintln!("==> libclang already installed at {}", clang_root.display());
        return Ok(());
    }

    eprintln!("==> Downloading libclang/llvm packages into deps/toolchains/");
    if clang_root.exists() {
        std::fs::remove_dir_all(&clang_root)?;
    }
    ensure_dir(&clang_root)?;

    let packages = [
        (
            "libllvm14",
            "http://archive.ubuntu.com/ubuntu/pool/main/l/llvm-toolchain-14/libllvm14_14.0.0-1ubuntu1.1_amd64.deb",
        ),
        (
            "libclang1-14",
            "http://archive.ubuntu.com/ubuntu/pool/universe/l/llvm-toolchain-14/libclang1-14_14.0.0-1ubuntu1.1_amd64.deb",
        ),
        (
            "libclang-14-dev",
            "http://archive.ubuntu.com/ubuntu/pool/universe/l/llvm-toolchain-14/libclang-14-dev_14.0.0-1ubuntu1.1_amd64.deb",
        ),
    ];

    for (pkg, fallback) in packages {
        let url = process::apt_deb_url(pkg, fallback);
        let tmp = tempfile::NamedTempFile::new().context("temp file")?;
        download(&url, tmp.path())?;
        run_checked(
            "dpkg-deb",
            &[
                "-x",
                &tmp.path().to_string_lossy(),
                &clang_root.to_string_lossy(),
            ],
        )?;
    }

    if !lib.is_file() {
        bail!("libclang install failed");
    }
    Ok(())
}

pub fn system_libclang_present() -> bool {
    std::process::Command::new("find")
        .args(["/usr/lib", "-name", "libclang.so"])
        .output()
        .ok()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

pub fn install_sp804_qemu(root: &Path) -> Result<PathBuf> {
    let toolchains = toolchains_dir(root);
    let install_prefix = toolchains.join("qemu-sp804");
    let qemu_bin = install_prefix.join("bin/qemu-system-aarch64");

    if qemu_bin.is_file() {
        eprintln!(
            "==> SP804 QEMU already installed at {}",
            install_prefix.display()
        );
        return Ok(install_prefix.join("bin"));
    }

    for tool in ["curl", "patch", "make"] {
        if !command_on_path(tool) {
            bail!("{tool} required to build SP804 QEMU");
        }
    }

    let pkg_config = run_checked("pkg-config", &["--exists", "glib-2.0"]);
    if pkg_config.is_err() {
        bail!("libglib2.0-dev required to build SP804 QEMU (apt install libglib2.0-dev libpixman-1-dev)");
    }

    let qemu_version = "6.2.0";
    let src_dir = toolchains.join(format!("qemu-{qemu_version}-src"));
    let tarball = toolchains.join(format!("qemu-{qemu_version}.tar.xz"));
    let patch = root.join("support/qemu/arm-virt-sp804.patch");

    ensure_dir(&toolchains)?;
    if !tarball.is_file() {
        eprintln!("==> Downloading QEMU {qemu_version}");
        download(
            &format!("https://download.qemu.org/qemu-{qemu_version}.tar.xz"),
            &tarball,
        )?;
    }

    if !src_dir.is_dir() {
        eprintln!("==> Extracting QEMU {qemu_version}");
        extract_tar_xz(&tarball, &toolchains)?;
        let extracted = toolchains.join(format!("qemu-{qemu_version}"));
        std::fs::rename(extracted, &src_dir)?;
    }

    let virt_c = src_dir.join("hw/arm/virt.c");
    let virt_contents = std::fs::read_to_string(&virt_c).unwrap_or_default();
    if !virt_contents.contains("VIRT_TIMER1") {
        eprintln!("==> Applying arm-virt-sp804 patch");
        run_checked(
            "patch",
            &[
                "-d",
                &src_dir.to_string_lossy(),
                "-p1",
                "-i",
                &patch.to_string_lossy(),
            ],
        )?;
    }

    let config_status = src_dir.join("build/config.status");
    if !config_status.is_file() {
        eprintln!("==> Configuring SP804 QEMU (aarch64-softmmu only)");
        let build_dir = src_dir.join("build");
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)?;
        }
        let status = std::process::Command::new("./configure")
            .current_dir(&src_dir)
            .args([
                &format!("--prefix={}", install_prefix.display()),
                "--target-list=aarch64-softmmu",
                "--disable-werror",
                "--disable-docs",
                "--disable-gtk",
                "--disable-sdl",
                "--disable-vnc",
                "--disable-curses",
                "--audio-drvlist=",
                "--disable-capstone",
                "--disable-libusb",
                "--disable-usb-redir",
                "--disable-vhost-user",
                "--disable-vhost-vdpa",
            ])
            .status()
            .context("configure SP804 QEMU")?;
        if !status.success() {
            bail!("SP804 QEMU configure failed");
        }
    } else {
        eprintln!("==> Reusing existing SP804 QEMU build tree");
    }

    eprintln!("==> Building SP804 QEMU");
    let nproc = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".to_string());
    run_checked(
        "make",
        &["-C", &src_dir.join("build").to_string_lossy(), "-j", &nproc],
    )?;
    run_checked(
        "make",
        &["-C", &src_dir.join("build").to_string_lossy(), "install"],
    )?;

    if !qemu_bin.is_file() {
        bail!("SP804 QEMU build failed");
    }
    eprintln!("==> SP804 QEMU installed at {}", install_prefix.display());
    Ok(install_prefix.join("bin"))
}

fn find_dir(parent: &Path, pattern: &str) -> Option<PathBuf> {
    let prefix = pattern.split('*').next()?;
    std::fs::read_dir(parent)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .find(|p| {
            p.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum InstallTool {
    ArmToolchain,
    RiscvToolchain,
    Qemu,
    QemuRiscv,
    Sp804Qemu,
    Dtc,
    Xmllint,
    Libclang,
}

pub fn install_tool(root: &Path, tool: InstallTool) -> Result<PathBuf> {
    match tool {
        InstallTool::ArmToolchain => install_arm_toolchain(root),
        InstallTool::RiscvToolchain => install_riscv_toolchain(root),
        InstallTool::Qemu => install_qemu_aarch64(root),
        InstallTool::QemuRiscv => install_qemu_riscv64(root),
        InstallTool::Sp804Qemu => install_sp804_qemu(root),
        InstallTool::Dtc => install_dtc(root),
        InstallTool::Xmllint => install_xmllint(root),
        InstallTool::Libclang => {
            install_libclang(root)?;
            Ok(toolchains_dir(root).join("libclang/usr/bin"))
        }
    }
}
