use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use tar::Archive;

use crate::{
    install::toolchains_dir,
    process::{binary_parent, download, ensure_dir, run_checked},
};

pub fn install_deb_tool(
    root: &Path,
    name: &str,
    deb_url: &str,
    binary: &str,
) -> Result<std::path::PathBuf> {
    if let Ok(path) = which::which(binary) {
        return binary_parent(&path);
    }

    let install_root = toolchains_dir(root).join(name);
    let bin_path = install_root.join("usr/bin").join(binary);
    if bin_path.is_file() {
        return Ok(install_root.join("usr/bin"));
    }

    ensure_dir(&toolchains_dir(root))?;
    let tmp = tempfile::NamedTempFile::new().context("temp file")?;
    eprintln!("==> Downloading {binary} from {deb_url}");
    download(deb_url, tmp.path())?;
    if install_root.exists() {
        fs::remove_dir_all(&install_root)?;
    }
    ensure_dir(&install_root)?;
    run_checked(
        "dpkg-deb",
        &[
            "-x",
            &tmp.path().to_string_lossy(),
            &install_root.to_string_lossy(),
        ],
    )?;

    if !bin_path.is_file() {
        bail!("{binary} not found in {deb_url}");
    }
    Ok(install_root.join("usr/bin"))
}

pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    ensure_dir(dest)?;
    let file = fs::File::open(archive).with_context(|| format!("open {}", archive.display()))?;
    let decoder = GzDecoder::new(file);
    let mut tar = Archive::new(decoder);
    tar.unpack(dest).context("unpack tar.gz")?;
    Ok(())
}
