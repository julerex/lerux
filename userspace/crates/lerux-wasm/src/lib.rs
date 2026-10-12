//! Signed ordinary WebAssembly for the runtime protection domain.
//!
//! A downloaded program is still an `LRW1` envelope: magic, version, the
//! WebAssembly payload, and an ed25519 signature over the header and payload.
//! [`run_signed`] checks that signature before [`run`] instantiates the module.
//! A bad signature never instantiates it.
//!
//! The payload is ordinary WebAssembly. This crate does not keep an opcode
//! allowlist. Execution is the `wasmi` interpreter. The WebAssembly Micro
//! Runtime is not linked.

#![no_std]

extern crate alloc;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

mod exec;

pub use exec::run;

/// ASCII magic at the start of a signed blob.
pub const MAGIC: &[u8; 4] = b"LRW1";

/// Blob version this crate accepts.
pub const VERSION: u8 = 1;

/// Maximum WebAssembly payload inside one blob.
pub const MAX_WASM_LEN: usize = 4096;

/// Bytes of header before the WebAssembly payload (`magic`, version, `wasm_len`).
pub const HEADER_LEN: usize = 7;

/// Ed25519 signature length.
pub const SIG_LEN: usize = 64;

/// Log line the postcard smoke program passes to `lerux.log` when `1 + 1 == 2`.
pub const SMOKE_LOG: &[u8] = b"lerux-prog: ran";

/// `rustc` flags `lerux prog pack` uses. Host tests compile with the same list
/// so the runtime is checked against the module the postcard guest runs.
pub const SMOKE_RUSTC_ARGS: &[&str] = &[
    "--target",
    "wasm32-unknown-unknown",
    "--edition",
    "2024",
    "-C",
    "panic=abort",
    "-C",
    "opt-level=z",
    "-C",
    "overflow-checks=off",
    "-C",
    "lto=yes",
    "-C",
    "debuginfo=0",
    "-C",
    "link-arg=-zstack-size=4096",
];

/// Why a blob or module was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `LRW1` magic missing.
    Magic,
    /// Blob version is not [`VERSION`].
    Version,
    /// Buffer ended early, or the blob length does not match the header.
    Truncated,
    /// WebAssembly payload empty or larger than [`MAX_WASM_LEN`].
    WasmLen,
    /// Ed25519 check failed, or the verifying key is not a valid point.
    Signature,
    /// The bytes are not a module this runtime can instantiate.
    Module,
    /// The module has no `start` export of type `() -> ()`.
    Export,
    /// `start` trapped. The module was instantiated.
    Trap,
}

/// Host functions an ordinary module may import from `lerux`.
///
/// `log` receives bytes from the module's linear memory. `read` copies a file
/// into `dst` and returns the byte count, or a negative value. `socket`
/// returns the file-descriptor number, or a negative value.
pub trait Imports {
    /// Print `bytes`. The caller owns the slice for this call only.
    fn log(&mut self, bytes: &[u8]);

    /// Read `path` into `dst`. Return the count, or a negative value.
    fn read(&mut self, path: &[u8], dst: &mut [u8]) -> i32;

    /// Allocate the client's socket. Return its number, or a negative value.
    fn socket(&mut self) -> i32;
}

/// Check the signature and instantiate the payload.
///
/// [`Error::Signature`] means the module was not instantiated.
pub fn run_signed<H: Imports + 'static>(
    blob: &[u8],
    verifying_key: &[u8; 32],
    host: &mut H,
) -> Result<(), Error> {
    let wasm = verify(blob, verifying_key)?;
    run(wasm, host)
}

/// Write the bytes that the signature covers: magic, version, length, WebAssembly.
pub fn signing_message(wasm: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let wasm_len = wasm_len(wasm)?;
    let n = HEADER_LEN + wasm_len;
    if out.len() < n {
        return Err(Error::Truncated);
    }
    out[0..4].copy_from_slice(MAGIC);
    out[4] = VERSION;
    out[5..7].copy_from_slice(&(wasm_len as u16).to_le_bytes());
    out[HEADER_LEN..n].copy_from_slice(wasm);
    Ok(n)
}

/// Write a complete blob: [`signing_message`] plus the 64-byte signature.
pub fn encode(wasm: &[u8], signature: &[u8; SIG_LEN], out: &mut [u8]) -> Result<usize, Error> {
    let n = signing_message(wasm, out)?;
    let total = n + SIG_LEN;
    if out.len() < total {
        return Err(Error::Truncated);
    }
    out[n..total].copy_from_slice(signature);
    Ok(total)
}

/// Check the ed25519 signature and return the WebAssembly payload.
///
/// The signature covers [`signing_message`] (header and payload), not the payload alone.
/// Failure does not instantiate a module.
pub fn verify<'a>(blob: &'a [u8], verifying_key: &[u8; 32]) -> Result<&'a [u8], Error> {
    if blob.len() < HEADER_LEN + SIG_LEN {
        return Err(Error::Truncated);
    }
    if blob.get(..4) != Some(MAGIC.as_slice()) {
        return Err(Error::Magic);
    }
    if blob[4] != VERSION {
        return Err(Error::Version);
    }
    let wasm_len = u16::from_le_bytes([blob[5], blob[6]]) as usize;
    if wasm_len == 0 || wasm_len > MAX_WASM_LEN {
        return Err(Error::WasmLen);
    }
    let total = HEADER_LEN + wasm_len + SIG_LEN;
    if blob.len() != total {
        return Err(Error::Truncated);
    }
    let message = &blob[..HEADER_LEN + wasm_len];
    let mut sig_bytes = [0u8; SIG_LEN];
    sig_bytes.copy_from_slice(&blob[HEADER_LEN + wasm_len..]);
    let signature = Signature::from_bytes(&sig_bytes);
    let key = VerifyingKey::from_bytes(verifying_key).map_err(|_| Error::Signature)?;
    key.verify(message, &signature)
        .map_err(|_| Error::Signature)?;
    Ok(&blob[HEADER_LEN..HEADER_LEN + wasm_len])
}

fn wasm_len(wasm: &[u8]) -> Result<usize, Error> {
    if wasm.is_empty() || wasm.len() > MAX_WASM_LEN {
        return Err(Error::WasmLen);
    }
    Ok(wasm.len())
}
