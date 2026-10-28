//! Host tools `lerux iso` needs, and the command that installs each one.
//!
//! The check runs before the image build and the ISO pack, so a missing
//! compiler or SDK fails in one report instead of halfway through cargo.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};

use crate::{
    board::{get_board, load_boards},
    build_sdk::sdk_path,
    install::system_libclang_present,
    process::command_on_path,
    qemu::sel4_elf,
};

const OVMF_CANDIDATES: &[&str] = &[
    "/usr/share/ovmf/OVMF.fd",
    "/usr/share/OVMF/OVMF.fd",
    "/usr/share/qemu/OVMF.fd",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Distro {
    Arch,
    Debian,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SdkFact {
    Missing,
    NoTool {
        path: String,
    },
    NoKernel {
        path: String,
        kernel: String,
        board: String,
    },
    Ready,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OvmfFact {
    Found(PathBuf),
    BadPath(String),
    Missing,
}

#[derive(Clone, Debug)]
pub(crate) struct HostFacts {
    pub cargo: Option<PathBuf>,
    pub rustup_cargo: Option<PathBuf>,
    pub rust_src: bool,
    pub sdk: SdkFact,
    pub clang: bool,
    pub readelf: bool,
    pub cc: bool,
    pub need_cc: bool,
    pub xorriso: bool,
    pub unpack_debs: bool,
    pub libclang: bool,
    pub fetch_libclang: bool,
    pub qemu_x86: bool,
    pub ovmf: OvmfFact,
    pub distro: Distro,
}

struct Gap {
    title: &'static str,
    detail: String,
}

/// Fail with one report when `just iso` is missing a host tool.
pub(crate) fn require_iso_host(
    root: &Path,
    board: &str,
    config: &str,
    need_image: bool,
    boot_test: bool,
) -> Result<()> {
    let facts = collect_host_facts(root, board, config, need_image)?;
    let gaps = host_gaps(&facts, need_image, boot_test);
    if gaps.is_empty() {
        return Ok(());
    }
    bail!("{}", format_gaps(&gaps));
}

pub(crate) fn find_ovmf() -> OvmfFact {
    if let Ok(path) = std::env::var("OVMF_PATH")
        && !path.is_empty()
    {
        let file = PathBuf::from(&path);
        if file.is_file() {
            return OvmfFact::Found(file);
        }
        return OvmfFact::BadPath(path);
    }
    for candidate in OVMF_CANDIDATES {
        let file = PathBuf::from(candidate);
        if file.is_file() {
            return OvmfFact::Found(file);
        }
    }
    OvmfFact::Missing
}

pub(crate) fn current_distro() -> Distro {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    distro_from_os_release(&text)
}

pub(crate) fn package_hint(distro: Distro, arch_pkgs: &str, deb_pkgs: &str) -> String {
    match distro {
        Distro::Arch => format!("sudo pacman -S {arch_pkgs}"),
        Distro::Debian => format!("sudo apt install {deb_pkgs}"),
        Distro::Other => {
            format!("Arch: sudo pacman -S {arch_pkgs}\nDebian/Ubuntu: sudo apt install {deb_pkgs}")
        }
    }
}

fn collect_host_facts(
    root: &Path,
    board: &str,
    config: &str,
    need_image: bool,
) -> Result<HostFacts> {
    let sdk = if need_image {
        sdk_fact(root, board, config)?
    } else {
        SdkFact::Ready
    };
    Ok(HostFacts {
        cargo: which("cargo"),
        rustup_cargo: rustup_cargo(),
        rust_src: rust_src_installed(),
        sdk,
        clang: command_on_path("clang"),
        readelf: command_on_path("readelf"),
        cc: command_on_path("cc"),
        need_cc: !root.join("deps/limine/limine-binary/limine").is_file(),
        xorriso: command_on_path("xorriso") || unpacked_xorriso_ready(root),
        unpack_debs: command_on_path("dpkg-deb")
            || (command_on_path("ar") && command_on_path("tar")),
        libclang: system_libclang_present(),
        fetch_libclang: command_on_path("dpkg-deb"),
        qemu_x86: command_on_path("qemu-system-x86_64"),
        ovmf: find_ovmf(),
        distro: current_distro(),
    })
}

fn sdk_fact(root: &Path, board: &str, config: &str) -> Result<SdkFact> {
    let Ok(path) = sdk_path(root) else {
        return Ok(SdkFact::Missing);
    };
    if !Path::new(&path).join("bin/microkit").is_file() {
        return Ok(SdkFact::NoTool { path });
    }
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, board)?;
    let kernel = sel4_elf(&path, &board_cfg.microkit_board, config);
    if !kernel.is_file() {
        return Ok(SdkFact::NoKernel {
            path,
            kernel: kernel.display().to_string(),
            board: board_cfg.microkit_board.clone(),
        });
    }
    Ok(SdkFact::Ready)
}

fn which(name: &str) -> Option<PathBuf> {
    which::which(name).ok()
}

fn rustup_cargo() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home).join(".cargo/bin/cargo");
    path.is_file().then_some(path)
}

fn rust_src_installed() -> bool {
    let Ok(output) = Command::new("rustc").arg("--print").arg("sysroot").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(sysroot) = String::from_utf8(output.stdout) else {
        return false;
    };
    Path::new(sysroot.trim())
        .join("lib/rustlib/src/rust/library/Cargo.lock")
        .is_file()
}

fn unpacked_xorriso_ready(root: &Path) -> bool {
    let prefix = root.join("deps/toolchains/xorriso");
    let lib = prefix.join("usr/lib/x86_64-linux-gnu");
    prefix.join("usr/bin/xorriso").is_file()
        && lib.join("libisoburn.so.1").is_file()
        && lib.join("libburn.so.4").is_file()
        && lib.join("libisofs.so.6").is_file()
        && lib.join("libjte.so.2").is_file()
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn host_gaps(facts: &HostFacts, need_image: bool, boot_test: bool) -> Vec<Gap> {
    let mut gaps = Vec::new();
    if need_image {
        match &facts.sdk {
            SdkFact::Missing => gaps.push(Gap {
                title: "Microkit SDK",
                detail: "\
Run: just fetch-sdk
That downloads the prebuilt SDK 2.3.0 used to build this ISO.
just build-sdk builds the SDK from source instead.
Or set MICROKIT_SDK to an existing SDK directory."
                    .to_string(),
            }),
            SdkFact::NoTool { path } => gaps.push(Gap {
                title: "Microkit SDK",
                detail: format!(
                    "{path} has no bin/microkit.\nRun: just fetch-sdk\nOr set MICROKIT_SDK to a complete SDK directory."
                ),
            }),
            SdkFact::NoKernel { path, kernel, board } => gaps.push(Gap {
                title: "seL4 kernel",
                detail: format!(
                    "The SDK at {path} has no {kernel}.\n\
just fetch-sdk provides the prebuilt SDK, including the x86_64_generic kernel this desk ISO uses.\n\
For Microkit board {board}, run: MICROKIT_BOARDS={board} just build-sdk"
                ),
            }),
            SdkFact::Ready => {}
        }
        if facts.cargo.is_none() {
            gaps.push(Gap {
                title: "cargo",
                detail: "\
cargo was not found on PATH.
Install rustup from https://rustup.rs and open a new shell.
Re-run just iso from this repository. rust-toolchain.toml selects nightly-2026-04-04, which includes rust-src."
                    .to_string(),
            });
        } else if !facts.rust_src {
            gaps.push(rust_src_gap(facts));
        }
        if !facts.libclang && !facts.fetch_libclang {
            gaps.push(Gap {
                title: "libclang",
                detail: format!(
                    "libclang was not found. Compiling the protection domains needs it for bindgen.\n{}",
                    package_hint(facts.distro, "clang", "libclang-dev")
                ),
            });
        }
    }
    if !facts.clang {
        gaps.push(Gap {
            title: "clang",
            detail: format!(
                "The UEFI trampoline support/mb2-stub is built with clang.\n{}",
                package_hint(facts.distro, "clang", "clang")
            ),
        });
    }
    if !facts.readelf {
        gaps.push(Gap {
            title: "readelf",
            detail: format!(
                "support/mb2-stub/build.sh runs readelf after it links the trampoline.\n{}",
                package_hint(facts.distro, "binutils", "binutils")
            ),
        });
    }
    if facts.need_cc && !facts.cc {
        gaps.push(Gap {
            title: "cc",
            detail: format!(
                "The Limine host tool is compiled with cc.\n{}",
                package_hint(facts.distro, "gcc", "gcc")
            ),
        });
    }
    if !facts.xorriso && !facts.unpack_debs {
        gaps.push(Gap {
            title: "xorriso",
            detail: format!(
                "xorriso is not on PATH, and this host cannot unpack the fallback Ubuntu packages (dpkg-deb, or ar and tar).\n{}",
                package_hint(facts.distro, "xorriso", "xorriso")
            ),
        });
    }
    if boot_test {
        if !facts.qemu_x86 {
            gaps.push(Gap {
                title: "qemu-system-x86_64",
                detail: format!(
                    "The ISO boot test runs QEMU.\n{}",
                    package_hint(facts.distro, "qemu-system-x86", "qemu-system-x86")
                ),
            });
        }
        match &facts.ovmf {
            OvmfFact::Found(_) => {}
            OvmfFact::BadPath(path) => gaps.push(Gap {
                title: "OVMF",
                detail: format!(
                    "OVMF_PATH={path} is not a file.\nSet it to a combined OVMF.fd, or unset it and install the ovmf package."
                ),
            }),
            OvmfFact::Missing => gaps.push(Gap {
                title: "OVMF",
                detail: format!(
                    "A combined OVMF firmware file was not found.\nTried OVMF_PATH and {}.\nDebian/Ubuntu: sudo apt install ovmf\nArch's edk2-ovmf package ships split CODE and VARS images. This boot test uses one combined OVMF.fd. Set OVMF_PATH to that file.",
                    OVMF_CANDIDATES.join(", ")
                ),
            }),
        }
    }
    gaps
}

fn rust_src_gap(facts: &HostFacts) -> Gap {
    let cargo = facts
        .cargo
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "cargo".to_string());
    let using_rustup = facts.rustup_cargo.as_ref().is_some_and(|rustup| {
        facts
            .cargo
            .as_ref()
            .is_some_and(|cargo| same_file(cargo, rustup))
    });
    let detail = if using_rustup {
        "\
The active Rust toolchain has no standard-library source.
The protection-domain build rebuilds core for a custom target from that source.
Run: rustup component add rust-src
Re-run just iso from this repository. rust-toolchain.toml selects nightly-2026-04-04, which includes rust-src."
            .to_string()
    } else if let Some(rustup) = &facts.rustup_cargo {
        let mut detail = format!(
            "\
cargo is {cargo}, and its sysroot has no standard-library source.
The protection-domain build rebuilds core for a custom target from that source.
rustup's cargo is {}. Put it first on PATH:
  export PATH=\"$HOME/.cargo/bin:$PATH\"
Re-run just iso from this repository. rust-toolchain.toml selects nightly-2026-04-04, which includes rust-src.",
            rustup.display()
        );
        if facts.distro == Distro::Arch {
            detail.push_str(
                "\nOn Arch, sudo pacman -S rust-src adds that source to the packaged compiler.",
            );
        }
        detail
    } else {
        let mut detail = format!(
            "\
cargo is {cargo}, and its sysroot has no standard-library source.
The protection-domain build rebuilds core for a custom target from that source.
Install rustup from https://rustup.rs and open a new shell.
Re-run just iso from this repository. rust-toolchain.toml selects nightly-2026-04-04, which includes rust-src."
        );
        if facts.distro == Distro::Arch {
            detail.push_str(
                "\nOn Arch, sudo pacman -S rust-src adds that source to the packaged compiler.",
            );
        }
        detail
    };
    Gap {
        title: "rust-src",
        detail,
    }
}

fn format_gaps(gaps: &[Gap]) -> String {
    let mut report = String::from("just iso cannot start. Required tools are missing.\n");
    for gap in gaps {
        report.push('\n');
        report.push_str(gap.title);
        report.push('\n');
        for line in gap.detail.lines() {
            report.push_str("  ");
            report.push_str(line);
            report.push('\n');
        }
    }
    report
}

pub(crate) fn distro_from_os_release(text: &str) -> Distro {
    let mut id = String::new();
    let mut like = String::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = unquote(value);
        } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
            like = unquote(value);
        }
    }
    let tokens = format!("{id} {like}");
    let arch = tokens.split_whitespace().any(|token| token == "arch");
    let debian = tokens
        .split_whitespace()
        .any(|token| token == "debian" || token == "ubuntu");
    if arch {
        Distro::Arch
    } else if debian {
        Distro::Debian
    } else {
        Distro::Other
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> HostFacts {
        HostFacts {
            cargo: Some(PathBuf::from("/usr/bin/cargo")),
            rustup_cargo: None,
            rust_src: true,
            sdk: SdkFact::Ready,
            clang: true,
            readelf: true,
            cc: true,
            need_cc: true,
            xorriso: true,
            unpack_debs: true,
            libclang: true,
            fetch_libclang: false,
            qemu_x86: true,
            ovmf: OvmfFact::Found(PathBuf::from("/usr/share/OVMF/OVMF.fd")),
            distro: Distro::Arch,
        }
    }

    #[test]
    fn os_release_maps_omarchy_to_arch_and_ubuntu_to_debian() {
        assert_eq!(
            distro_from_os_release("ID=omarchy\nID_LIKE=arch\n"),
            Distro::Arch
        );
        assert_eq!(
            distro_from_os_release("ID=ubuntu\nID_LIKE=debian\n"),
            Distro::Debian
        );
        assert_eq!(distro_from_os_release(""), Distro::Other);
    }

    #[test]
    fn ready_host_has_no_gaps() {
        assert!(host_gaps(&ready(), true, true).is_empty());
    }

    #[test]
    fn missing_sdk_suggests_fetch_sdk_only_when_the_image_must_be_built() {
        let mut facts = ready();
        facts.sdk = SdkFact::Missing;
        let report = format_gaps(&host_gaps(&facts, true, false));
        assert!(report.contains("just fetch-sdk"), "{report}");
        assert!(report.contains("just build-sdk"), "{report}");
        assert!(host_gaps(&facts, false, false).is_empty());
    }

    #[test]
    fn missing_kernel_names_the_microkit_board() {
        let mut facts = ready();
        facts.sdk = SdkFact::NoKernel {
            path: "/sdk".to_string(),
            kernel: "/sdk/board/qemu_virt_aarch64/debug/elf/sel4.elf".to_string(),
            board: "qemu_virt_aarch64".to_string(),
        };
        let report = format_gaps(&host_gaps(&facts, true, false));
        assert!(
            report.contains("MICROKIT_BOARDS=qemu_virt_aarch64 just build-sdk"),
            "{report}"
        );
    }

    #[test]
    fn system_cargo_with_rustup_installed_suggests_path_order() {
        let mut facts = ready();
        facts.rust_src = false;
        facts.rustup_cargo = Some(PathBuf::from("/home/julian/.cargo/bin/cargo"));
        let report = format_gaps(&host_gaps(&facts, true, false));
        assert!(report.contains("export PATH="), "{report}");
        assert!(report.contains("rust-toolchain.toml"), "{report}");
        assert!(report.contains("sudo pacman -S rust-src"), "{report}");
    }

    #[test]
    fn arch_without_rustup_mentions_the_rust_src_package() {
        let mut facts = ready();
        facts.rust_src = false;
        let report = format_gaps(&host_gaps(&facts, true, false));
        assert!(report.contains("https://rustup.rs"), "{report}");
        assert!(report.contains("sudo pacman -S rust-src"), "{report}");
    }

    #[test]
    fn debian_clang_hint_uses_apt() {
        let mut facts = ready();
        facts.clang = false;
        facts.distro = Distro::Debian;
        let report = format_gaps(&host_gaps(&facts, false, false));
        assert!(report.contains("sudo apt install clang"), "{report}");
    }

    #[test]
    fn boot_test_reports_qemu_and_ovmf_together() {
        let mut facts = ready();
        facts.qemu_x86 = false;
        facts.ovmf = OvmfFact::Missing;
        let report = format_gaps(&host_gaps(&facts, false, true));
        assert!(report.contains("qemu-system-x86"), "{report}");
        assert!(report.contains("OVMF_PATH"), "{report}");
        assert!(report.contains("edk2-ovmf"), "{report}");
    }

    #[test]
    fn cached_limine_tool_does_not_require_cc() {
        let mut facts = ready();
        facts.cc = false;
        facts.need_cc = false;
        assert!(host_gaps(&facts, false, false).is_empty());
    }
}
