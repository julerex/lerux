# ADR-008: Host-backed FS for QEMU (inject, not virtio-9p)

## Status

Accepted (Phase 63)

## Date

2026-08-16

## Context

Phase 63 wants a host file visible in the guest through existing `FsRequest` / `FsResponse`, so a developer can edit on the host and `cat` in QEMU without hand-formatting `disk.img`.

Options from the plan: virtio-9p (`-virtfs`), NFS-over-user-net, or a host tool that injects into `disk.img`.

Constraints:

- Same IPC (`FsRequest`) so shell / edit / backup do not grow a second protocol.
- Guest stays `#![no_std]`.
- A correct virtio-9p MMIO driver + 9P2000.L client is a new device PD, larger than one phase.
- NFS needs a network stack client and a host NFS daemon.

## Decision

1. **v1 is disk inject.** `lerux fs-host seed --dir <path>` formats LERUXFS2 and copies regular files into `/host/` on `support/disk.img`. The existing `fs-server` LERUXFS2 backend mounts that image. Apps use `/host/hello.txt` over postcard FS IPC.

2. **Board** `qemu_virt_aarch64_fs_host` reuses the fs SDF. `just test-fs-host` writes `build/fs-host/hello.txt`, seeds, and expects `lerux-fs: host hello ok`.

3. **virtio-9p / NFS** stay the follow-on if live host↔guest sharing is required. They need a dedicated ADR amendment and a driver PD.

## Alternatives considered

### virtio-9p in this phase

Pros: live share, LionsOS-shaped. Cons: new virtio device, 9P2000.L, QEMU virtfs security model, extra MMIO slot on virt. Rejected for v1 size.

### NFS client in fs-server

Pros: also live. Cons: depends on net-server + a host NFS export; worse for a deterministic smoke.

## Consequences

- Host → guest is a **seed step**, not a live mount. Re-seed after editing host files.
- `/host` is a LERUXFS2 directory (≤16 files, 256 KiB each, 22-byte names).
- Guest path `/host/…` works from shell once the disk is seeded (workstation wire is Phase 70).
