//! Signed Wasm modules for the `program-runtime` protection domain (ADR-010).
//!
//! A downloaded program is an `LRW1` blob: a closed Wasm subset plus an ed25519
//! signature over the header and module bytes. [`run`] executes `start` after
//! [`verify`] succeeds. The only import is `lerux.log(ptr, len)`.
//!
//! This is not a general Wasm runtime. Opcodes, sections, and imports outside
//! the smoke program's subset fail the module before `start` is entered.
//! Upstream `wasmi` / `wasmtime` are not used.

#![no_std]

#[cfg(test)]
extern crate std;

mod wasm;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub use wasm::run;

/// ASCII magic at the start of an `LRW1` blob.
pub const MAGIC: &[u8; 4] = b"LRW1";

/// Blob version this crate accepts.
pub const VERSION: u8 = 1;

/// Maximum Wasm payload inside one blob.
pub const MAX_WASM_LEN: usize = 4096;

/// Bytes of header before the Wasm payload (`magic`, version, `wasm_len`).
pub const HEADER_LEN: usize = 7;

/// Ed25519 signature length.
pub const SIG_LEN: usize = 64;

/// Linear memory the smoke module asks for (one Wasm page).
pub const MEMORY_LEN: usize = 65536;

/// Log line the smoke program passes to `lerux.log` when `1 + 1 == 2`.
pub const SMOKE_LOG: &[u8] = b"lerux-prog: ran";

/// `rustc` flags `lerux prog pack` uses. Host tests compile with the same list
/// so the interpreter is checked against the module the guest runs.
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
    /// Wasm payload empty or larger than [`MAX_WASM_LEN`].
    WasmLen,
    /// Ed25519 check failed, or the verifying key is not a valid point.
    Signature,
    /// Wasm header, section id, or section order.
    Section,
    /// Type section is outside the `(i32, i32) -> ()` and `() -> ()` pair.
    Type,
    /// Import is not the single function `lerux.log`.
    Import,
    /// More or fewer defined functions than the one `start` body.
    Function,
    /// Memory is not exactly one page.
    Memory,
    /// Global is not an `i32` initialized with `i32.const`.
    Global,
    /// No function export named `start`.
    Export,
    /// Code section shape.
    Code,
    /// Active data segment does not fit in memory.
    Data,
    /// Opcode outside the subset.
    Opcode,
    /// Value stack underflow, overflow, or leftover values on `() -> ()`.
    Stack,
    /// Load, store, or `log` address outside linear memory.
    Bounds,
    /// `block` / `br_if` / `end` do not nest.
    Control,
    /// Too many locals, or a local is not `i32`.
    Local,
    /// `call` target is not `lerux.log`.
    Call,
}

/// Write the bytes that the signature covers: magic, version, length, Wasm.
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

/// Check the ed25519 signature and return the Wasm payload.
///
/// The signature covers [`signing_message`] (header and Wasm), not the Wasm alone.
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
