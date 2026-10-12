//! Host checks: verify, run an ordinary module, and reject a bad signature.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use ed25519_dalek::{Signer, SigningKey};
use lerux_wasm::{
    encode, run, run_signed, signing_message, verify, Error, Imports, SIG_LEN, SMOKE_LOG,
};

struct Capture {
    bytes: Vec<u8>,
    calls: u32,
}

impl Imports for Capture {
    fn log(&mut self, bytes: &[u8]) {
        self.calls += 1;
        self.bytes.extend_from_slice(bytes);
    }

    fn read(&mut self, _path: &[u8], _dst: &mut [u8]) -> i32 {
        -1
    }

    fn socket(&mut self) -> i32 {
        -1
    }
}

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
        .args(lerux_wasm::SMOKE_RUSTC_ARGS)
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

fn execute(wasm: &[u8]) -> Result<Capture, Error> {
    let mut host = Capture {
        bytes: Vec::new(),
        calls: 0,
    };
    run(wasm, &mut host)?;
    Ok(host)
}

#[test]
fn committed_module_runs() {
    let blob = fs::read(repo_root().join("support/prog/smoke.lrw")).unwrap();
    let wasm = verify(&blob, &smoke_vk()).unwrap();
    let host = execute(wasm).unwrap();
    assert_eq!(host.bytes, SMOKE_LOG);
}

#[test]
fn recompiled_smoke_runs() {
    let src = repo_root().join("support/prog/smoke.rs");
    let out = std::env::temp_dir().join(format!("lerux-smoke-{}.wasm", std::process::id()));
    compile(&src, &out);
    let wasm = fs::read(&out).unwrap();
    let _ = fs::remove_file(&out);
    assert!(wasm.len() <= lerux_wasm::MAX_WASM_LEN);
    let blob = sign_wasm(&wasm);
    let wasm = verify(&blob, &smoke_vk()).unwrap();
    let host = execute(wasm).unwrap();
    assert_eq!(host.bytes, SMOKE_LOG);
}

#[test]
fn flipped_byte_fails_verify_and_does_not_instantiate() {
    let src = repo_root().join("support/prog/smoke.rs");
    let out = std::env::temp_dir().join(format!("lerux-flip-{}.wasm", std::process::id()));
    compile(&src, &out);
    let wasm = fs::read(&out).unwrap();
    let _ = fs::remove_file(&out);
    let mut blob = sign_wasm(&wasm);
    let flip_at = 7 + wasm.len() / 2;
    blob[flip_at] ^= 0x01;
    assert_eq!(verify(&blob, &smoke_vk()), Err(Error::Signature));

    let mut host = Capture {
        bytes: Vec::new(),
        calls: 0,
    };
    assert_eq!(
        run_signed(&blob, &smoke_vk(), &mut host),
        Err(Error::Signature)
    );
    assert_eq!(host.calls, 0);
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
    let host = execute(wasm).unwrap();
    assert_eq!(host.bytes, b"");
}

#[test]
fn unreachable_traps_after_the_import_runs() {
    let wasm = wat::parse_str(
        r#"
        (module
          (import "lerux" "log" (func $log (param i32 i32)))
          (memory (export "memory") 1)
          (data (i32.const 0) "x")
          (func (export "start")
            (call $log (i32.const 0) (i32.const 1))
            unreachable)
        )
        "#,
    )
    .unwrap();
    let mut host = Capture {
        bytes: Vec::new(),
        calls: 0,
    };
    assert_eq!(run(&wasm, &mut host), Err(Error::Trap));
    assert_eq!(host.calls, 1);
    assert_eq!(host.bytes, b"x");
}
