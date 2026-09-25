//! Pack `support/prog/smoke.rs` into a signed `LRW1` blob (ADR-010).

use std::{fs, path::Path, process::Command};

use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signer, SigningKey};

use crate::process::repo_root;

/// Compile `src` (default `support/prog/smoke.rs`) and write a signed blob.
pub fn pack(root: &Path, key: &Path, out: &Path, src: Option<&Path>) -> Result<()> {
    let src = match src {
        Some(path) => path.to_path_buf(),
        None => root.join("support/prog/smoke.rs"),
    };
    if !src.is_file() {
        bail!("missing {}", src.display());
    }
    let wasm_tmp = tempfile::NamedTempFile::new().context("wasm temp file")?;
    let status = Command::new("rustc")
        .args(lerux_prog::SMOKE_RUSTC_ARGS)
        .arg(&src)
        .arg("-o")
        .arg(wasm_tmp.path())
        .status()
        .context("spawn rustc (wasm32-unknown-unknown)")?;
    if !status.success() {
        bail!("rustc failed for {}", src.display());
    }
    let wasm = fs::read(wasm_tmp.path()).context("read wasm")?;
    if wasm.len() > lerux_prog::MAX_WASM_LEN {
        bail!(
            "wasm is {} bytes; max is {}",
            wasm.len(),
            lerux_prog::MAX_WASM_LEN
        );
    }
    let secret = fs::read(key).with_context(|| format!("read {}", key.display()))?;
    let secret: [u8; 32] = secret
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("ed25519 secret key must be 32 bytes"))?;
    let signing = SigningKey::from_bytes(&secret);
    let mut message = vec![0u8; lerux_prog::HEADER_LEN + wasm.len()];
    let message_len = lerux_prog::signing_message(&wasm, &mut message)
        .map_err(|err| anyhow::anyhow!("signing message: {err:?}"))?;
    let signature = signing.sign(&message[..message_len]);
    let mut sig = [0u8; lerux_prog::SIG_LEN];
    sig.copy_from_slice(&signature.to_bytes());
    let mut blob = vec![0u8; message_len + lerux_prog::SIG_LEN];
    let n = lerux_prog::encode(&wasm, &sig, &mut blob)
        .map_err(|err| anyhow::anyhow!("encode blob: {err:?}"))?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(out, &blob[..n]).with_context(|| format!("write {}", out.display()))?;
    println!(
        "==> wrote {} (wasm {} bytes, blob {n} bytes)",
        out.display(),
        wasm.len()
    );
    Ok(())
}

/// `lerux prog pack` with repo-relative defaults resolved from `root` when the
/// path is relative.
pub fn pack_cli(key: &Path, out: &Path, src: Option<&Path>) -> Result<()> {
    let root = repo_root()?;
    let key = resolve(&root, key);
    let out = resolve(&root, out);
    let src = src.map(|path| resolve(&root, path));
    pack(&root, &key, &out, src.as_deref())
}

fn resolve(root: &Path, path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_smoke_verifies_and_runs() {
        let root = repo_root().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("smoke.lrw");
        pack(&root, &root.join("support/keys/smoke.ed25519"), &out, None).unwrap();
        let blob = fs::read(&out).unwrap();
        let vk = fs::read(root.join("support/keys/smoke.ed25519.pub")).unwrap();
        let vk: [u8; 32] = vk.as_slice().try_into().unwrap();
        let wasm = lerux_prog::verify(&blob, &vk).unwrap();
        let mut memory = vec![0u8; lerux_prog::MEMORY_LEN];
        let mut logged = Vec::new();
        lerux_prog::run(wasm, &mut memory, |chunk| logged.extend_from_slice(chunk)).unwrap();
        assert_eq!(logged, lerux_prog::SMOKE_LOG);
    }
}
