use std::{fs, path::Path};

use anyhow::{Context, Result};
use tar::Archive;
use xz2::read::XzDecoder;

use crate::process::ensure_dir;

pub fn extract_tar_xz(archive: &Path, dest: &Path) -> Result<()> {
    ensure_dir(dest)?;
    let file = fs::File::open(archive).with_context(|| format!("open {}", archive.display()))?;
    let decoder = XzDecoder::new(file);
    let mut tar = Archive::new(decoder);
    tar.unpack(dest).context("unpack tar.xz")?;
    Ok(())
}
