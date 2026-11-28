# Image signing keys (Phase 67)

`smoke.ed25519` is a **dev/CI-only** ed25519 secret. Do not use it for release images. The same key signs `loader.img` and ADR-010 `LRW1` modules (`lerux prog pack`).

```bash
lerux keygen --out support/payload/fixtures/keys/smoke.ed25519
lerux sign --board qemu_virt_aarch64
lerux verify-image --board qemu_virt_aarch64 --key support/payload/fixtures/keys/smoke.ed25519.pub --require-sig
```
