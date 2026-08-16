# Observability and ops (Phase 57)

Diagnose a failed boot or hung service from **serial + one host command**.

## Guest tools

| Command | Purpose |
|---------|---------|
| `dmesg` | Recent log ring (tagged `E[shell] …`) |
| `dmesg --pd shell` | Filter by PD tag (`shell`, `fs`, `net`, `supervis`) |
| `dmesg -l warn` | Min level (`error` / `warn` / `info` / `debug`) |
| `ps` / `top` | Service table with state (`ready` / `start` / `degraded` / `error`) |
| `status <id>` | One service: ready, state, last error string |
| `config get log.level` | Policy applied at boot as log-server min level |

Log ring capacity is 48 lines; `dmesg` returns the last 6 matching lines. Serial lines are also emitted live as `L[tag] message`.

## Host tools

| Command | Purpose |
|---------|---------|
| `just test` / `lerux test` | Smoke; always writes `build/smoke-logs/<board>.serial.log` |
| `lerux diagnose <log>` | Summarize faults, watchdog, errors from a capture |
| `just diagnose` | Same, default workstation smoke log path |
| `just bench` / `just bench-check` | Microbenches; `--check` enforces thresholds |

```bash
# After a failed smoke:
cargo run -q -p lerux-cli -- diagnose build/smoke-logs/qemu_virt_aarch64_workstation.serial.log

# Live pipe (if you capture QEMU yourself):
qemu-system-aarch64 … 2>&1 | tee /tmp/lerux.serial.log
cargo run -q -p lerux-cli -- diagnose /tmp/lerux.serial.log
```

CI uploads `build/smoke-logs/` as artifact `smoke-serial-<matrix.id>` on every smoke job.

## QEMU-only developer loop (Phase 70)

Work without a board. Default smokes stay cold-boot.

```bash
# Interactive workstation (SP804 QEMU for aarch64)
just test-echo                         # small loop
lerux run --board qemu_virt_aarch64 --gdb
# another terminal: gdb-multiarch -ex 'target remote :1234'

# Do not persist disk.img writes
lerux test --board qemu_virt_aarch64_fs --snapshot

# Host files into /host (Phase 63 inject), then boot
printf 'hello from host\n' > build/fs-host/hello.txt
lerux fs-host seed --dir build/fs-host
just test-fs-host

# Sign and verify before a QEMU boot
lerux sign --board qemu_virt_aarch64
lerux verify-image --board qemu_virt_aarch64 --key support/keys/smoke.ed25519.pub --require-sig

# FAT remains the small-file alternate
just test-fs-fat
```

Monitor is already on stdio (`mon:stdio`): `Ctrl-A c` then `savevm` / `loadvm` for a manual snapshot. CI never uses `--snapshot` or `--gdb`.

Benches stay aarch64 (`just bench`) unless numbers on RISC-V/x86 prove stable.

## Fault path

See [`debug.md`](debug.md): `just test-debug` for hierarchy faults; `just test-isolation` for crash-then-FS (Phase 60); QEMU gdbstub for interactive backtraces. Production workstation images stay without a debug parent (ADR-005). Trust map: [`security.md`](security.md).
