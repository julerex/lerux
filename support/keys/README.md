# Image signing keys (Phase 67)

`smoke.ed25519` is a **dev/CI-only** ed25519 secret. Do not use it for release images.

```bash
lerux keygen --out support/keys/smoke.ed25519
lerux sign --board qemu_virt_aarch64
lerux verify-image --board qemu_virt_aarch64 --key support/keys/smoke.ed25519.pub --require-sig
```
