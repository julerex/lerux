# lerux-filesystem

Lerux-owned fork of the vendored RedoxFS reference at [`userspace/redoxfs/`](../redoxfs/).
This crate preserves **RedoxFS on-disk format compatibility** and guest-visible binary
names (`redoxfs`, `redoxfs-mkfs`, etc.) while allowing internal refactors.

## Development

- Host tests: `just test-lerux-fs` (same harness as the frozen reference)
- Parity gate: `just test-fs-parity` (reference + fork must both pass)
- Coverage: `just coverage-lerux-fs`

The frozen reference copy defines expected behavior; add new tests there first, then
copy them here when changing the spec.

## Upstream

Based on [RedoxFS](https://gitlab.redox-os.org/redox-os/redoxfs) (MIT). See
[`docs/vendored.md`](../../docs/vendored.md) for provenance.
