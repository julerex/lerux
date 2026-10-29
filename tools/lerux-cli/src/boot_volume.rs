//! UEFI boot image for the mb2-stub contract.
//!
//! Firmware loads `EFI/BOOT/BOOTX64.EFI`. That file is GRUB, built by
//! `grub-mkimage` with prefix `/boot/grub`. The menu and the three payloads
//! live on the same FAT. `EspPath` is the only spelling of those paths.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};

/// Relative path on the EFI system partition.
///
/// `fat` is the spelling `fat_esp::write_image` stores. `grub` adds the
/// leading slash for `grub.cfg`. Callers do not keep a second copy of the
/// string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EspPath {
    bytes: &'static str,
}

impl EspPath {
    pub(crate) const fn fat(self) -> &'static str {
        self.bytes
    }

    pub(crate) fn grub(self) -> String {
        format!("/{}", self.bytes)
    }

    const fn new(bytes: &'static str) -> Self {
        let raw = bytes.as_bytes();
        assert!(!raw.is_empty(), "esp path is empty");
        assert!(
            raw[0] != b'/',
            "esp path is relative; grub() adds the slash"
        );
        assert!(!contains_dotdot(bytes), "esp path rejects ..");
        Self { bytes }
    }
}

const fn contains_dotdot(bytes: &str) -> bool {
    let raw = bytes.as_bytes();
    let mut i = 0;
    while i + 1 < raw.len() {
        if raw[i] == b'.' && raw[i + 1] == b'.' {
            return true;
        }
        i += 1;
    }
    false
}

const EFI_PATH: EspPath = EspPath::new("EFI/BOOT/BOOTX64.EFI");
const PREFIX: EspPath = EspPath::new("boot/grub");
const CFG_PATH: EspPath = EspPath::new("boot/grub/grub.cfg");
const STUB_PATH: EspPath = EspPath::new("boot/mb2-stub.elf");
const KERNEL_PATH: EspPath = EspPath::new("boot/sel4.elf");
const LOADER_PATH: EspPath = EspPath::new("boot/loader.img");

/// Root names from the old x86 deploy. `write_to` removes these files so a
/// mounted volume keeps the ESP layout. Sidecars are not in this list.
const RETIRED_ROOT_NAMES: &[&str] = &[
    "lerux-grub.cfg",
    "lerux-limine.conf",
    "mb2-stub.elf",
    "sel4.elf",
    "loader.img",
];

/// Modules named on the `grub-mkimage` command line, in that order.
///
/// `iso9660` is absent: the core image reads the FAT it was loaded from.
/// `part_msdos` and `part_gpt` are both named because xorriso publishes one
/// ESP through a protective MBR and an EFI boot image. `search` is absent:
/// the menu uses absolute paths on that volume. Multiboot 1 `multiboot` is
/// absent: the stub header is Multiboot 2.
const GRUB_MODULES: &[&str] = &[
    "fat",
    "part_gpt",
    "part_msdos",
    "normal",
    "configfile",
    "multiboot2",
    "video",
    "efi_gop",
];

/// Early script embedded in the EFI image. It loads `$prefix/grub.cfg`.
/// The only `multiboot2` line lives in `grub_cfg`.
const EMBEDDED_NORMAL: &str = "normal\n";

/// xorriso arguments for one ESP image. No BIOS El Torito entry, no
/// `--grub2-mbr`, no HFS+/APM.
pub(crate) const XORRISO_BOOT: &[&str] = &[
    "-as",
    "mkisofs",
    "-R",
    "-r",
    "-J",
    "--efi-boot",
    "boot/esp.img",
    "-efi-boot-part",
    "--efi-boot-image",
    "--protective-msdos-label",
];

/// Host `grub-mkimage` and the platform directory it reads.
pub(crate) struct GrubHost {
    mkimage: PathBuf,
    platform_dir: PathBuf,
}

impl GrubHost {
    /// Debian package `grub-efi-amd64-bin` (depends on `grub-common`).
    /// Arch package `grub`. Both install this directory and `grub-mkimage`.
    const PLATFORM_DIR: &'static str = "/usr/lib/grub/x86_64-efi";

    pub(crate) fn discover() -> Result<Self> {
        let mkimage = which::which("grub-mkimage").map_err(|_| {
            anyhow::anyhow!(
                "grub-mkimage is not on PATH. Debian/CI: apt install grub-efi-amd64-bin. Arch: pacman -S grub"
            )
        })?;
        let platform_dir = PathBuf::from(Self::PLATFORM_DIR);
        for module in GRUB_MODULES {
            let path = platform_dir.join(format!("{module}.mod"));
            if !path.is_file() {
                bail!(
                    "missing {} (Debian/CI: grub-efi-amd64-bin, Arch: grub)",
                    path.display()
                );
            }
        }
        Ok(Self {
            mkimage,
            platform_dir,
        })
    }

    /// mkimage plus every entry in the platform directory, so a package
    /// update invalidates a cached ISO without a second module list.
    pub(crate) fn freshness_inputs(&self) -> Result<Vec<PathBuf>> {
        let mut inputs = vec![self.mkimage.clone()];
        let entries = fs::read_dir(&self.platform_dir)
            .with_context(|| format!("read {}", self.platform_dir.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("read {}", self.platform_dir.display()))?;
            inputs.push(entry.path());
        }
        Ok(inputs)
    }
}

/// Firmware entry. Production code builds it with [`Self::build`].
pub(crate) struct GrubEfi {
    bytes: Vec<u8>,
}

impl GrubEfi {
    /// `grub-mkimage -O x86_64-efi -p /boot/grub` with `normal\n` embedded
    /// and [`GRUB_MODULES`] after it.
    ///
    /// `-d` is the directory [`GrubHost::discover`] already checked, so the
    /// image and the check cannot see two different module trees.
    ///
    /// Unit tests inject bytes through [`Self::from_bytes_for_test`]. This
    /// constructor stays uncalled in `cfg(test)` builds because `grub-mkimage`
    /// is a host package, not a test fixture.
    #[cfg_attr(
        test,
        expect(
            dead_code,
            reason = "unit tests inject EFI bytes; cargo test does not run grub-mkimage"
        )
    )]
    pub(crate) fn build(host: &GrubHost) -> Result<Self> {
        let dir = tempfile::tempdir().context("grub-mkimage work directory")?;
        let embedded = dir.path().join("embedded.cfg");
        let out = dir.path().join("BOOTX64.EFI");
        fs::write(&embedded, EMBEDDED_NORMAL)
            .with_context(|| format!("write {}", embedded.display()))?;
        let mut cmd = Command::new(&host.mkimage);
        cmd.arg("-O")
            .arg("x86_64-efi")
            .arg("-d")
            .arg(&host.platform_dir)
            .arg("-p")
            .arg(PREFIX.grub())
            .arg("-c")
            .arg(&embedded)
            .arg("-o")
            .arg(&out);
        for module in GRUB_MODULES {
            cmd.arg(*module);
        }
        let output = cmd.output().context("failed to run grub-mkimage")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("grub-mkimage exited with {}\n{stderr}", output.status);
        }
        let bytes = fs::read(&out).with_context(|| format!("read {}", out.display()))?;
        if bytes.is_empty() {
            bail!("grub-mkimage wrote an empty BOOTX64.EFI");
        }
        Ok(Self { bytes })
    }

    #[cfg(test)]
    pub(crate) fn from_bytes_for_test(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Payloads in stub order.
///
/// `stub` is the Multiboot 2 kernel. `kernel` is module tag 0 (`sel4.elf`).
/// `loader` is module tag 1 (`loader.img`). The stub ignores module strings
/// and treats tag order as identity, so this struct has no `Vec` to reorder.
pub(crate) struct StubPayloads {
    stub: Vec<u8>,
    kernel: Vec<u8>,
    loader: Vec<u8>,
}

impl StubPayloads {
    /// Sources whose mtimes invalidate a cached ISO. The stub binary is
    /// rebuilt on each pack; the freshness check must not depend on that
    /// rebuild having already happened.
    pub(crate) const SOURCES: &[&str] = &[
        "support/mb2-stub/entry.S",
        "support/mb2-stub/stub.c",
        "support/mb2-stub/stub.ld",
        "support/mb2-stub/multiboot2.h",
        "support/mb2-stub/build.sh",
    ];

    pub(crate) fn new(stub: Vec<u8>, kernel: Vec<u8>, loader: Vec<u8>) -> Result<Self> {
        if stub.is_empty() || kernel.is_empty() || loader.is_empty() {
            bail!("mb2-stub, sel4.elf, and loader.img must be non-empty");
        }
        Ok(Self {
            stub,
            kernel,
            loader,
        })
    }
}

/// The boot image written to an ESP directory or a FAT16 blob.
pub(crate) struct StubBootImage {
    efi: GrubEfi,
    payloads: StubPayloads,
    cfg: String,
}

impl StubBootImage {
    pub(crate) fn assemble(efi: GrubEfi, payloads: StubPayloads) -> Self {
        Self {
            efi,
            payloads,
            cfg: grub_cfg(),
        }
    }

    /// Overwrite the five ESP paths under `root` and remove [`RETIRED_ROOT_NAMES`].
    /// Does not write SHA-256 sidecars and does not delete them.
    pub(crate) fn write_to(&self, root: &Path) -> Result<()> {
        for (rel, bytes) in self.file_table() {
            let path = root.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create {}", parent.display()))?;
            }
            fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
        }
        for name in RETIRED_ROOT_NAMES {
            let path = root.join(name);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err).with_context(|| format!("remove {}", path.display()));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn fat_image(&self) -> Result<Vec<u8>> {
        crate::fat_esp::write_image(&self.file_table())
    }

    fn file_table(&self) -> [(&str, &[u8]); 5] {
        [
            (EFI_PATH.fat(), self.efi.bytes()),
            (CFG_PATH.fat(), self.cfg.as_bytes()),
            (STUB_PATH.fat(), self.payloads.stub.as_slice()),
            (KERNEL_PATH.fat(), self.payloads.kernel.as_slice()),
            (LOADER_PATH.fat(), self.payloads.loader.as_slice()),
        ]
    }
}

/// Menu GRUB's `normal` loads from `$prefix/grub.cfg`.
///
/// `gfxpayload=keep` asks GRUB to forward the firmware GOP mode. Timeout is
/// hidden and zero so QEMU smokes do not sit on a menu. The board name is
/// not in the menu: one entry, same bytes on every board.
fn grub_cfg() -> String {
    format!(
        "\
set timeout=0
set timeout_style=hidden
set gfxpayload=keep
menuentry \"lerux\" {{
    multiboot2 {stub}
    module2 {kernel}
    module2 {loader}
}}
",
        stub = STUB_PATH.grub(),
        kernel = KERNEL_PATH.grub(),
        loader = LOADER_PATH.grub(),
    )
}

/// Policy identity stored beside a packed ISO.
pub(crate) fn recipe_stamp() -> String {
    format!(
        "prefix {}\nembedded {EMBEDDED_NORMAL}modules {}\nxorriso {}\ncfg\n{}",
        PREFIX.grub(),
        GRUB_MODULES.join(" "),
        XORRISO_BOOT.join(" "),
        grub_cfg(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_image() -> StubBootImage {
        StubBootImage::assemble(
            GrubEfi::from_bytes_for_test(b"efi".to_vec()),
            StubPayloads::new(b"stub".to_vec(), b"kernel".to_vec(), b"loader".to_vec()).unwrap(),
        )
    }

    #[test]
    fn prefix_and_cfg_path_agree() {
        assert_eq!(format!("{}/grub.cfg", PREFIX.grub()), CFG_PATH.grub());
        for path in [
            EFI_PATH,
            PREFIX,
            CFG_PATH,
            STUB_PATH,
            KERNEL_PATH,
            LOADER_PATH,
        ] {
            assert!(!path.fat().starts_with('/'), "{}", path.fat());
        }
    }

    #[test]
    fn menu_is_stub_then_kernel_then_loader() {
        let cfg = grub_cfg();
        let stub_at = cfg
            .find(&format!("multiboot2 {}", STUB_PATH.grub()))
            .expect("stub");
        let kernel_at = cfg
            .find(&format!("module2 {}", KERNEL_PATH.grub()))
            .expect("kernel");
        let loader_at = cfg
            .find(&format!("module2 {}", LOADER_PATH.grub()))
            .expect("loader");
        assert!(stub_at < kernel_at && kernel_at < loader_at, "{cfg}");
        assert!(
            !cfg.contains(&format!("multiboot2 {}", KERNEL_PATH.grub())),
            "{cfg}"
        );
        assert!(!cfg.contains("multiboot2 /sel4.elf"), "{cfg}");
        assert!(!cfg.contains("multiboot2 /boot/sel4.elf"), "{cfg}");
        assert!(!cfg.contains("insmod"), "{cfg}");
        assert!(!cfg.contains("search"), "{cfg}");
        assert!(!cfg.contains("multiboot "), "{cfg}");
        assert!(cfg.contains("set timeout=0\n"), "{cfg}");
        assert!(cfg.contains("set timeout_style=hidden\n"), "{cfg}");
        assert!(cfg.contains("set gfxpayload=keep\n"), "{cfg}");
        assert!(cfg.contains("menuentry \"lerux\" {\n"), "{cfg}");
        assert_eq!(EMBEDDED_NORMAL, "normal\n");
        assert!(!EMBEDDED_NORMAL.contains("multiboot"));
    }

    #[test]
    fn recipe_covers_cfg_and_modules() {
        let stamp = recipe_stamp();
        assert!(stamp.contains(&grub_cfg()), "{stamp}");
        assert!(stamp.contains("multiboot2 /boot/mb2-stub.elf"), "{stamp}");
        assert!(!stamp.contains("multiboot2 /boot/sel4.elf"), "{stamp}");
        assert!(!stamp.contains("multiboot2 /sel4.elf"), "{stamp}");
        assert!(
            stamp.contains("fat part_gpt part_msdos normal configfile multiboot2 video efi_gop"),
            "{stamp}"
        );
        assert!(stamp.contains("--efi-boot boot/esp.img"), "{stamp}");
        assert!(!stamp.contains("iso9660"), "{stamp}");
        assert!(!stamp.contains("limine"), "{stamp}");
        assert!(!stamp.contains("--grub2-mbr"), "{stamp}");
        assert!(!stamp.contains(" -b "), "{stamp}");
    }

    #[test]
    fn file_table_matches_menu_paths() {
        let image = sample_image();
        let table = image.file_table();
        assert_eq!(table[0].0, EFI_PATH.fat());
        assert_eq!(table[0].1, b"efi");
        assert_eq!(table[1].0, CFG_PATH.fat());
        assert_eq!(table[1].1, grub_cfg().as_bytes());
        assert_eq!(table[2].0, STUB_PATH.fat());
        assert_eq!(table[2].1, b"stub");
        assert_eq!(table[3].0, KERNEL_PATH.fat());
        assert_eq!(table[3].1, b"kernel");
        assert_eq!(table[4].0, LOADER_PATH.fat());
        assert_eq!(table[4].1, b"loader");
    }

    #[test]
    fn empty_payload_is_rejected() {
        assert!(StubPayloads::new(vec![], b"k".to_vec(), b"l".to_vec()).is_err());
        assert!(StubPayloads::new(b"s".to_vec(), vec![], b"l".to_vec()).is_err());
        assert!(StubPayloads::new(b"s".to_vec(), b"k".to_vec(), vec![]).is_err());
    }

    #[test]
    fn write_to_replaces_retired_root_names() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("lerux-limine.conf"), b"old-limine").unwrap();
        fs::write(root.join("lerux-grub.cfg"), b"multiboot2 /sel4.elf\n").unwrap();
        fs::write(root.join("mb2-stub.elf"), b"old-stub").unwrap();
        fs::write(root.join("sel4.elf"), b"old-kernel").unwrap();
        fs::write(root.join("loader.img"), b"old-loader").unwrap();
        fs::write(root.join("sel4.elf.sha256"), b"keep-sidecar").unwrap();
        let image = sample_image();
        image.write_to(root).unwrap();
        assert!(!root.join("lerux-limine.conf").exists());
        assert!(!root.join("lerux-grub.cfg").exists());
        assert!(!root.join("mb2-stub.elf").exists());
        assert!(!root.join("sel4.elf").exists());
        assert!(!root.join("loader.img").exists());
        assert_eq!(
            fs::read(root.join("sel4.elf.sha256")).unwrap(),
            b"keep-sidecar"
        );
        assert_eq!(fs::read(root.join("boot/sel4.elf")).unwrap(), b"kernel");
        assert_eq!(fs::read(root.join("boot/mb2-stub.elf")).unwrap(), b"stub");
        assert_eq!(fs::read(root.join("boot/loader.img")).unwrap(), b"loader");
        assert_eq!(fs::read(root.join("EFI/BOOT/BOOTX64.EFI")).unwrap(), b"efi");
        let cfg = fs::read_to_string(root.join("boot/grub/grub.cfg")).unwrap();
        let stub_at = cfg.find("multiboot2 /boot/mb2-stub.elf").expect("stub");
        let kernel_at = cfg.find("module2 /boot/sel4.elf").expect("kernel");
        let loader_at = cfg.find("module2 /boot/loader.img").expect("loader");
        assert!(stub_at < kernel_at && kernel_at < loader_at, "{cfg}");
        assert!(!cfg.contains("multiboot2 /boot/sel4.elf"), "{cfg}");
        image.write_to(root).unwrap();
        assert_eq!(fs::read(root.join("boot/sel4.elf")).unwrap(), b"kernel");
    }
}
