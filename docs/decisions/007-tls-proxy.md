# ADR-007: TLS proxy PD for outbound HTTPS

## Status

Accepted (Phase 51 TLS stretch)

## Date

2026-08-16

## Context

Phase 51 wants `fetch https://…` on QEMU without putting a certificate store or rustls into every app PD. Apps already speak cleartext postcard RPC (`NetRequest`) to `net-server`. rustls + a root store is large (hundreds of KiB) and needs `alloc`, a time source, and an RNG — a poor fit for `fetch-client` / `shell`.

Constraints:

- Userspace stays `#![no_std]` (optional `alloc` in service PDs).
- Smokes must stay deterministic: no public-internet origin.
- Untrusted apps must not map NIC DMA (ADR-003).
- `aws-lc-rs` / `ring` are awkward on `aarch64-sel4-microkit`; a RustCrypto provider is the portable path.

## Decision

1. **Dedicated `tls-proxy` PD** is the sole rustls owner. Apps use new `TlsRequest` / `TlsResponse` (Connect / Send / Recv / Close / Poll) — cleartext HTTP over postcard. The proxy is the only net-server TCP client on the fetch-tls board.

2. **Crypto:** `rustls` 0.23 + `rustls-rustcrypto` (no_std + alloc). Session I/O uses rustls’s **unbuffered** API so we never need `std::io`.

3. **Trust store for the smoke:** a committed test CA (`support/tls/lerux-smoke-ca.der`). Host `lerux https-one` serves with the matching server cert (SAN `host`, `10.0.2.2`, `127.0.0.1`). `webpki-roots` is an optional crate feature for real outbound HTTPS later — not required for the QEMU gate.

4. **Time / RNG:** rustls cert checks use a fixed `TimeProvider` inside the smoke cert window (not wall-clock RTC). `getrandom` is a custom ChaCha-style CSPRNG seeded at init. Both are **smoke-grade**; a hardware RNG and RTC time are follow-ups.

5. **Do not** put rustls inside `net-server` or skip certificate verification.

## Alternatives considered

### rustls inside `net-server` (`TlsConnect` on `NetRequest`)

Fewer PDs, but the already-trusted stack grows by the cert store and crypto. Rejected so net-server stays L3/L4.

### rustls inside `fetch-client`

Duplicates the store in every TLS app. Rejected.

### `dangerous()` skip-verify or public `https://example.com`

Fails the deterministic-smoke bar and does not exercise verification.

### `embedded-tls` instead of rustls

Smaller, but the roadmap named rustls; staying on rustls keeps the verification story aligned with host `https-one`.

## Consequences

- New board `qemu_virt_aarch64_fetch_tls`: fetch-client → tls-proxy → net-server → virtio-net.
- Host helper `lerux https-one` (port 8443) pairs with QEMU user-net `10.0.2.2`.
- Workstation / shell `fetch https://` is a follow-up (extra PD + channels + QoS).
- `webpki-roots` and a real entropy/time source remain stretch.

## References

- [`plan-arch.md`](../plan-arch.md) Phase 51
- [ADR-003](003-net-virtualiser.md)
- [rustls unbuffered API](https://docs.rs/rustls/0.23/rustls/unbuffered/index.html)
- [rustls-rustcrypto](https://github.com/RustCrypto/rustls-rustcrypto)
