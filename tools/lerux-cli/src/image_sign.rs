//! Phase 67: host-side ed25519 signatures for `loader.img`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

use crate::image_digest;

pub fn sig_path(loader: &Path) -> PathBuf {
    let mut s = loader.as_os_str().to_os_string();
    s.push(".sig");
    PathBuf::from(s)
}

pub fn keygen(secret_path: &Path) -> Result<()> {
    if let Some(parent) = secret_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let sk = SigningKey::generate(&mut OsRng);
    fs::write(secret_path, sk.to_bytes())
        .with_context(|| format!("write {}", secret_path.display()))?;
    let pub_path = pub_path_for(secret_path);
    fs::write(&pub_path, sk.verifying_key().to_bytes())
        .with_context(|| format!("write {}", pub_path.display()))?;
    println!(
        "==> wrote {} and {} (ed25519 smoke/dev key)",
        secret_path.display(),
        pub_path.display()
    );
    Ok(())
}

fn pub_path_for(secret: &Path) -> PathBuf {
    let mut p = secret.as_os_str().to_os_string();
    p.push(".pub");
    PathBuf::from(p)
}

fn load_signing_key(path: &Path) -> Result<SigningKey> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("ed25519 secret key must be 32 bytes"))?;
    Ok(SigningKey::from_bytes(&arr))
}

fn load_verifying_key(path: &Path) -> Result<VerifyingKey> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("ed25519 public key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&arr).context("invalid ed25519 public key")
}

pub fn sign(loader: &Path, secret_path: &Path) -> Result<()> {
    if !loader.is_file() {
        bail!("missing image {}", loader.display());
    }
    let sk = load_signing_key(secret_path)?;
    let bytes = fs::read(loader).with_context(|| format!("read {}", loader.display()))?;
    let sig = sk.sign(&bytes);
    let out = sig_path(loader);
    fs::write(&out, sig.to_bytes()).with_context(|| format!("write {}", out.display()))?;
    println!("==> wrote {} (ed25519)", out.display());
    Ok(())
}

/// Verify digest always; verify `.sig` when present or when `require_sig`.
pub fn verify_image(loader: &Path, pub_path: Option<&Path>, require_sig: bool) -> Result<()> {
    image_digest::verify_sidecar(loader)?;
    let sig = sig_path(loader);
    if !sig.is_file() {
        if require_sig {
            bail!(
                "missing {} (pass --require-sig only after `lerux sign`)",
                sig.display()
            );
        }
        return Ok(());
    }
    let Some(pub_path) = pub_path else {
        bail!("signature present but no --key / public key given");
    };
    let vk = load_verifying_key(pub_path)?;
    let bytes = fs::read(loader).with_context(|| format!("read {}", loader.display()))?;
    let sig_bytes = fs::read(&sig).with_context(|| format!("read {}", sig.display()))?;
    let arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("signature must be 64 bytes"))?;
    let signature = Signature::from_bytes(&arr);
    vk.verify(&bytes, &signature)
        .map_err(|_| anyhow::anyhow!("ed25519 signature check failed for {}", loader.display()))?;
    println!("==> verified {} (ed25519)", loader.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_sign_verify_and_require_sig() {
        let tmp = tempfile::tempdir().unwrap();
        let secret = tmp.path().join("smoke.ed25519");
        let loader = tmp.path().join("loader.img");
        fs::write(&loader, b"signed-bytes").unwrap();
        image_digest::write_sidecar(&loader).unwrap();
        keygen(&secret).unwrap();
        sign(&loader, &secret).unwrap();
        let pubk = pub_path_for(&secret);
        verify_image(&loader, Some(&pubk), true).unwrap();
        fs::write(&loader, b"tampered").unwrap();
        image_digest::write_sidecar(&loader).unwrap();
        assert!(verify_image(&loader, Some(&pubk), true).is_err());
    }
}
