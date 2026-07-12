# CI and local smoke tests

GitHub Actions workflow: [`.github/workflows/rust.yml`](../.github/workflows/rust.yml).

## Pipeline

1. **check** — `just check` (`cargo fmt --all --check` + clippy on host crates; no SDK).
2. **sdk** — Docker image, fetch sources, build Microkit SDK (cached), **prebuild patched SP804 QEMU** (cached), upload SDK artifact.
3. **check-pd** — `just check-pd` (cross-target clippy on PD + shared userspace crates; needs SDK artifact).
4. **smoke** — 29 parallel matrix jobs; each restores SDK artifact, per-job `build/` cache, and SP804 QEMU (init/composed/blk-composed/http-composed/net-composed/ipc-composed/workstation only; init-riscv/init-x86 use stock QEMU). Serial captures land in `build/smoke-logs/` and upload as `smoke-serial-<id>` (Phase 57).
5. **package** — Phase 40: build `edit` / `chat-client` / `http-file-browser` ELFs for workstation, pin sha256, upload artifacts.

```mermaid
flowchart LR
  check[check job]
  sdk[sdk job]
  checkPd[check-pd job]
  smoke[smoke matrix x29]
  package[package ELF artifacts]
  sdk --> checkPd
  sdk --> smoke
  sdk --> package
```

Local mirror: `just check` (format + clippy for `lerux-cli` and `lerux-interface-types`); `just check-pd` after `just build-sdk` (or `just check-all` for both).

## Smoke matrix

| Job ID | Command | Notes |
|--------|---------|-------|
| `aarch64` | `just test` | Serial hello on virt |
| `x86_64` | `BOARD=x86_64_generic just test` | COM1 serial |
| `riscv64` | `just test-riscv` | NS16550 MMIO |
| `virtio` | `just disk-img && just test-virtio` | aarch64 virtio blk/net + TCP RX |
| `echo` | `just test-echo` | aarch64 echo IPC |
| `x86-echo` | `just test-x86-echo` | x86 echo IPC |
| `riscv-echo` | `just test-riscv-echo` | RISC-V echo IPC |
| `riscv-virtio` | `just disk-img && just test-riscv-virtio` | RISC-V virtio |
| `init` | `just test-init` | PL031 + SP804; patched QEMU |
| `init-riscv` | `just test-init-riscv` | Goldfish RTC + rdtime; stock QEMU |
| `init-x86` | `just test-init-x86` | CMOS RTC + TSC; stock QEMU |
| `composed` | `just disk-img && just test-composed` | init + virtio in one system |
| `blk-composed` | `just disk-img && just test-blk-composed` | init + block IPC; patched QEMU |
| `http` | `just test-http` | virtio-net HTTP `GET /` via hostfwd |
| `http-composed` | `just test-http-composed` | init + HTTP; patched QEMU |
| `x86-http` | `just test-x86-http` | x86 q35 PCI virtio-net HTTP via hostfwd |
| `riscv-http` | `just test-riscv-http` | RISC-V MMIO virtio-net HTTP via hostfwd |
| `x86-virtio` | `just disk-img && just test-x86-virtio` | x86 q35 PCI virtio-blk/net + TCP RX |
| `blk` | `just test-blk` | aarch64 block IPC over virtio-blk |
| `riscv-blk` | `just test-riscv-blk` | RISC-V block IPC |
| `x86-blk` | `just test-x86-blk` | x86 PCI virtio-blk block IPC |
| `net` | `just test-net` | aarch64 net IPC over virtio-net (UDP TX) |
| `fetch` | `just test-fetch` | aarch64 HTTP fetch over net IPC (TCP + DNS) |
| `riscv-net` | `just test-riscv-net` | RISC-V net IPC |
| `x86-net` | `just test-x86-net` | x86 PCI virtio-net net IPC |
| `net-composed` | `just test-net-composed` | init + net IPC; patched QEMU |
| `ipc-composed` | `just disk-img && just test-ipc-composed` | init + blk/net IPC; patched QEMU |
| `fs` | `just disk-img && just test-fs` | aarch64 FS IPC |
| `workstation` | `just disk-img && just test-workstation` | supervisor+shell+edit+chat+http-fs; SP804 + hostfwd curl |

Local mirror: `just test-all` (requires full SDK; creates `support/disk.img` once).

## Hardware serial smoke (Phase 47)

Expect strings live in **`support/smoke-expects.toml`** (shared by QEMU and hw-serial).  
Test modes for `lerux test`:

| Mode | Flag / env | Behaviour |
|------|------------|-----------|
| `auto` (default) | `--mode auto` or unset | QEMU boards → QEMU; hardware boards → hw-serial **if** `LERUX_HW_SERIAL` is set, else image-only success |
| `qemu` | `--mode qemu` | Force QEMU (fails on hardware-only boards) |
| `hw-serial` | `--mode hw-serial` or `LERUX_TEST_MODE=hw-serial` | Read serial device; requires `LERUX_HW_SERIAL` |

**Golden path (RPi4 workstation):**

```bash
# 1. Deploy loader.img to SD boot (Phase 52)
DEST=/media/$USER/boot just deploy-rpi4
# 2. U-Boot: fatload mmc 0 0x10000000 loader.img; go 0x10000000
# 3. Host smoke (serial free — not held by screen):
BOARD=rpi4b_4gb_workstation LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw
#    → boot expects + scripted ls/pwd/ip from support/smoke-expects.toml
```

Optional env:

| Variable | Default | Purpose |
|----------|---------|---------|
| `LERUX_HW_SERIAL` | (required for hw-serial) | TTY path, e.g. `/dev/ttyUSB0` |
| `LERUX_HW_BAUD` | `115200` | Serial baud |
| `LERUX_HW_LOCK_DIR` | `$TMPDIR/lerux-hw-locks` | Advisory lock directory (single-writer) |
| `LERUX_HW_LOCK_WAIT_SECS` | `300` | Wait for lock |
| `LERUX_TEST_MODE` | `auto` | Same as `--mode` |

Locks prevent two local jobs from racing the same board (`{board}.lock` with PID; stale locks are stolen if the PID is gone).

### Self-hosted CI workflow

Workflow: [`.github/workflows/hw-serial.yml`](../.github/workflows/hw-serial.yml) (`workflow_dispatch` only).

1. Runner labels: `self-hosted`, `lerux-hw`
2. Repo variable `LERUX_HW_ENABLED=true` (job is skipped otherwise — no infinite queue on github-hosted)
3. Repo variable `LERUX_HW_SERIAL` (device path on the runner)
4. Optional `LERUX_HW_BAUD`

Cloud PR smoke **does not** require a Pi. Hardware remains an opt-in lab path.

## Caches

| Cache | Key inputs | Restored in |
|-------|------------|-------------|
| Workspace | `deps/versions.toml`, `tools/lerux-cli` | sdk |
| SDK | versions + `SDK_CACHE_SUFFIX` | sdk |
| SP804 QEMU | patch + `install-qemu-sp804.sh` | sdk (build), smoke init/composed/blk-composed/http-composed/net-composed/ipc-composed/workstation (restore) |
| Shared `build/target/` | `Cargo.lock` | check-pd, smoke (all jobs) |
| Per-smoke `build/<board>/` | `Cargo.lock` + matrix job id | smoke |

SP804 QEMU is built once in the **sdk** job so `init`, `composed`, `blk-composed`, `http-composed`, and `net-composed` do not each cold-build QEMU (~4 min). Cache paths include install prefix, source tree, and tarball.

Caches are saved with `if: always()` when the artifact exists, so a failing smoke job still retains partial `build/` and a completed QEMU install.

## Patched QEMU

Stock QEMU `virt` lacks SP804 at `0x90d0000`. Init, composed, blk-composed, http-composed, and net-composed smokes use `cargo run -p lerux-cli -- install sp804-qemu`. The installer prints **only** the install `bin` directory on stdout (build logs go to stderr).

## Troubleshooting

| Symptom | Likely cause |
|---------|----------------|
| `Argument list too long` on `python3` | Corrupted `PATH` from capturing QEMU build stdout — fixed in SP804 installer stderr/stdout split |
| Init passes, timer times out | Stock QEMU used instead of patched build — check `which qemu-system-aarch64` |
| Composed flaky on `init ok` | Serial/debug interleaving — `boot-init` notifies `hello` before virtio (composed-sync) |
| `x86-http`: serial shows `listening`, `curl` times out | Stale QEMU or `tcp-echo-server` on host **18080** — see [boards.md — x86 HTTP inbound](boards.md#x86-http-inbound-operational-notes); smoke recipe kills both before start |
| `x86-http`: QEMU idle at `listening` | Expected until host `curl` — guest waits on driver notifications; use `just test-x86-http` or curl from another terminal |
| `x86-virtio` fails `TCP RX ok` after `x86-http` | Usually port **18080** still in use; kill stale QEMU/echo server, rerun virtio test |
| Piping `just test … \| tail` shows no output then SIGTERM | `tail` buffers until the command exits; run smoke tests without `tail` when debugging |
| `libglib2.0-dev` missing locally | Install build deps or use Docker (`lerux-dev` image) |