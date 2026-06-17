# vendor/ — Plain source snapshots

This directory holds pristine copies of third-party and upstream Redox sources used by lerux.

**Rule:** No `.git` directories or live git dependencies. These are static reference trees.

## Vendored snapshots

| Directory                  | Source                                      | Date       | Notes |
|----------------------------|---------------------------------------------|------------|-------|
| `redox-kernel/`            | redox-os/kernel                             | 2026-06-16 | Pristine kernel snapshot (see vendored.md) |
| `redox-bootloader/`        | redox-os/bootloader                         | 2026-06-17 | Pristine bootloader snapshot for reference. Copied from `/home/julian/repos/redox/redox_org/bootloader` (commit 2a718991b3deb343746f2dbb0ee9b3e63a4c47d8). `.git` and build metadata removed. |
| `relibc/`                  | redox-os/relibc                             | 2026-05-30 | Partial, with `.git` cleaned 2026-06-16 |
| `redox-log/`               | redox-os/redox-log                          | 2026-05-30 | Cleaned |
| `redox-recipes/`           | (partial)                                   | 2026-06-17 | Dev recipes only |
| `sbi-rt-0.0.3/`, etc.      | crates.io / vendored crates                 | 2026-06-16 | From `cargo vendor` |

## Cleanup performed on each vendoring

- `rm -rf .git .gitignore .gitlab-ci.yml .helix target`
- Kept Cargo.toml, src/, asm/, linkers/, etc. as-is for reference.
- No modifications to source (except for later in-place work under userspace/ copies).

See `docs/vendored.md` for full tracking, provenance, and lerux divergence notes.
