//! Host checks for the ADR-010 smoke module: verify, run, and reject.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use ed25519_dalek::{Signer, SigningKey};
use lerux_prog::{encode, run, signing_message, verify, Error, MEMORY_LEN, SIG_LEN, SMOKE_LOG};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn smoke_key() -> SigningKey {
    let bytes = fs::read(repo_root().join("support/keys/smoke.ed25519")).unwrap();
    let arr: [u8; 32] = bytes.as_slice().try_into().unwrap();
    SigningKey::from_bytes(&arr)
}

fn smoke_vk() -> [u8; 32] {
    let bytes = fs::read(repo_root().join("support/keys/smoke.ed25519.pub")).unwrap();
    bytes.as_slice().try_into().unwrap()
}

fn compile(src: &Path, out: &Path) {
    let status = Command::new("rustc")
        .args(lerux_prog::SMOKE_RUSTC_ARGS)
        .arg(src)
        .arg("-o")
        .arg(out)
        .status()
        .expect("spawn rustc");
    assert!(status.success(), "rustc {src:?}");
}

fn sign_wasm(wasm: &[u8]) -> Vec<u8> {
    let mut message = vec![0u8; wasm.len() + 7];
    let n = signing_message(wasm, &mut message).unwrap();
    let sig = smoke_key().sign(&message[..n]);
    let mut blob = vec![0u8; n + SIG_LEN];
    let mut sig_bytes = [0u8; SIG_LEN];
    sig_bytes.copy_from_slice(&sig.to_bytes());
    encode(wasm, &sig_bytes, &mut blob).unwrap();
    blob
}

fn execute(wasm: &[u8]) -> Result<Vec<u8>, Error> {
    let mut memory = vec![0u8; MEMORY_LEN];
    let mut logged = Vec::new();
    run(wasm, &mut memory, |chunk| logged.extend_from_slice(chunk))?;
    Ok(logged)
}

#[test]
fn committed_module_runs() {
    let blob = fs::read(repo_root().join("support/prog/smoke.lrw")).unwrap();
    let wasm = verify(&blob, &smoke_vk()).unwrap();
    assert_eq!(execute(wasm).unwrap(), SMOKE_LOG);
}

#[test]
fn recompiled_smoke_runs() {
    let src = repo_root().join("support/prog/smoke.rs");
    let out = std::env::temp_dir().join(format!("lerux-smoke-{}.wasm", std::process::id()));
    compile(&src, &out);
    let wasm = fs::read(&out).unwrap();
    let _ = fs::remove_file(&out);
    assert!(wasm.len() <= lerux_prog::MAX_WASM_LEN);
    let blob = sign_wasm(&wasm);
    let wasm = verify(&blob, &smoke_vk()).unwrap();
    assert_eq!(execute(wasm).unwrap(), SMOKE_LOG);
}

#[test]
fn flipped_byte_fails_verify_and_does_not_run() {
    let src = repo_root().join("support/prog/smoke.rs");
    let out = std::env::temp_dir().join(format!("lerux-flip-{}.wasm", std::process::id()));
    compile(&src, &out);
    let wasm = fs::read(&out).unwrap();
    let _ = fs::remove_file(&out);
    let mut blob = sign_wasm(&wasm);
    let flip_at = 7 + wasm.len() / 2;
    blob[flip_at] ^= 0x01;
    assert_eq!(verify(&blob, &smoke_vk()), Err(Error::Signature));
}

#[test]
fn sum_not_two_does_not_log() {
    let src_text = fs::read_to_string(repo_root().join("support/prog/smoke.rs")).unwrap();
    let edited = src_text.replacen("if sum == 2", "if sum == 3", 1);
    assert_ne!(edited, src_text);
    let dir = std::env::temp_dir();
    let src = dir.join(format!("lerux-ne-{}.rs", std::process::id()));
    let out = dir.join(format!("lerux-ne-{}.wasm", std::process::id()));
    fs::write(&src, edited).unwrap();
    compile(&src, &out);
    let wasm = fs::read(&out).unwrap();
    let _ = fs::remove_file(&src);
    let _ = fs::remove_file(&out);
    let blob = sign_wasm(&wasm);
    let wasm = verify(&blob, &smoke_vk()).unwrap();
    assert_eq!(execute(wasm).unwrap(), b"");
}

#[test]
fn unknown_opcode_is_rejected_before_log() {
    let wasm = unreachable_module();
    let mut logged = false;
    let mut memory = vec![0u8; MEMORY_LEN];
    let err = run(&wasm, &mut memory, |_| logged = true);
    assert_eq!(err, Err(Error::Opcode));
    assert!(!logged);
}

/// One-page module whose `start` body is `unreachable`.
fn unreachable_module() -> Vec<u8> {
    fn uleb(mut n: u32) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (n & 0x7f) as u8;
            n >>= 7;
            if n != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if n == 0 {
                break;
            }
        }
        out
    }
    fn section(id: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![id];
        out.extend(uleb(body.len() as u32));
        out.extend_from_slice(body);
        out
    }
    fn name(bytes: &[u8]) -> Vec<u8> {
        let mut out = uleb(bytes.len() as u32);
        out.extend_from_slice(bytes);
        out
    }

    let mut module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let types = {
        let mut body = uleb(2);
        body.extend_from_slice(&[0x60, 2, 0x7f, 0x7f, 0]);
        body.extend_from_slice(&[0x60, 0, 0]);
        body
    };
    module.extend(section(1, &types));
    let mut import = uleb(1);
    import.extend(name(b"lerux"));
    import.extend(name(b"log"));
    import.extend_from_slice(&[0x00, 0x00]);
    module.extend(section(2, &import));
    module.extend(section(3, &[1, 1]));
    module.extend(section(5, &[1, 0x00, 1]));
    let mut export = uleb(1);
    export.extend(name(b"start"));
    export.extend_from_slice(&[0x00, 1]);
    module.extend(section(7, &export));
    // locals 0, unreachable, end
    let code_body = {
        let inner = [0x00u8, 0x00, 0x0b];
        let mut body = uleb(1);
        body.extend(uleb(inner.len() as u32));
        body.extend_from_slice(&inner);
        body
    };
    module.extend(section(10, &code_body));
    // active data at 0, empty payload (section is required)
    let data = [1u8, 0x00, 0x41, 0x00, 0x0b, 0x00];
    module.extend(section(11, &data));
    module
}
