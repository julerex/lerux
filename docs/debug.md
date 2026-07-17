# Debugging protection domains

Phase 46 provides two complementary paths on **QEMU aarch64**:

1. **In-tree fault parent** — Microkit hierarchy delivers a child’s fault to `debug-handler`, which logs IP/address (smoke: `just test-debug`).
2. **Host GDB via QEMU gdbstub** — attach `gdb-multiarch` for interactive backtraces and breakpoints without forked seL4/Microkit.

Full in-guest GDB RSP ([libgdb](https://github.com/au-ts/libgdb)) needs non-upstream kernel/Microkit patches; see [ADR-005](decisions/005-debug-pd.md).

## Smoke: fault parent (`just test-debug`)

```bash
just test-debug
# or: BOARD=qemu_virt_aarch64_debug just test
```

Expected log sequence (debug UART):

1. `lerux-debug: ready (parent fault handler)`
2. `crash-demo: about to fault`
3. `lerux-debug: fault child=1 …`
4. `lerux-debug: VmFault ip=… addr=…`
5. `lerux-debug: crash-demo stopped (no restart)`
6. `lerux-debug: crash dump child=1 count=…` (Phase 57; machine-parseable for `lerux diagnose`)

Layout: `crash-demo` is a **child** of `debug-handler` in `debug.system.template` (`id="1"` → `Child::new(1)`).

## Optional workstation fault parent (Phase 57)

Default **workstation** stays lean: flat PDs, no hierarchy (ADR-005). To catch faults in a bulk app:

1. Nest that PD under `debug-handler` in a **debug-only** system template (do not ship as the production profile).
2. Keep `just test-debug` as the CI fault-path smoke.
3. On failure, use the serial capture:

```bash
just test-debug
# or after any smoke:
just diagnose LOG=build/smoke-logs/qemu_virt_aarch64_debug.serial.log
```

Host GDB remains the interactive path for production-image PDs (gdbstub, no template change).

## QEMU gdbstub + gdb-multiarch (backtraces)

### 1. Build a board image

Any aarch64 board works; the debug demo is a good start:

```bash
just image BOARD=qemu_virt_aarch64_debug
# ELF + loader under build/qemu_virt_aarch64_debug/
```

PD ELFs (with symbols in debug/release builds that retain them):

- `build/qemu_virt_aarch64_debug/debug-handler.elf`
- `build/qemu_virt_aarch64_debug/crash-demo.elf`
- Loader: `build/qemu_virt_aarch64_debug/loader.img`

### 2. Run QEMU with gdbstub

```bash
# From repo root (adjust paths if your build-dir differs)
qemu-system-aarch64 \
  -machine virt,virtualization=on \
  -cpu cortex-a53 \
  -m size=2G \
  -serial mon:stdio \
  -nographic \
  -device loader,file=build/qemu_virt_aarch64_debug/loader.img,addr=0x70000000,cpu-num=0 \
  -s -S
```

- `-s` — listen on `tcp::1234` (short for `-gdb tcp::1234`)
- `-S` — do not start CPUs until GDB continues

`lerux-cli` does not pass `-s` by default (automated smokes would hang). Add it when debugging interactively.

### 3. Attach GDB

```bash
gdb-multiarch build/qemu_virt_aarch64_debug/crash-demo.elf
```

Inside GDB:

```
(gdb) target remote :1234
(gdb) # Optional: also load parent symbols
(gdb) add-symbol-file build/qemu_virt_aarch64_debug/debug-handler.elf
(gdb) break main   # or a Rust symbol if demangled / known
(gdb) continue
```

After the null write in `crash-demo`, the PD faults. From the **host** view you can still inspect registers and memory that QEMU exposes; the seL4 fault is handled in-guest by `debug-handler`.

For a hung (non-faulting) PD, interrupt with Ctrl-C in GDB after `continue` and use `bt` / `info registers` with the appropriate ELF loaded.

### 4. Tips

| Tip | Detail |
|-----|--------|
| Architecture | `set architecture aarch64` if GDB does not auto-detect |
| Multiple PDs | Load one ELF at a time; VAs are per-PD address spaces |
| Release builds | Symbols may be thinner; use `debug` Microkit config for kernel UART |
| Workstation | Same QEMU flags; prefer a smaller board first |

## RPi4

Hardware GDB (OpenOCD / JTAG) is out of Phase 46 scope. On device, keep serial logging and reproduce issues under QEMU when possible. A future phase may document JTAG once hierarchy debugging is proven on virt.

## Isolation smoke (Phase 60)

```bash
just test-isolation
# BOARD=qemu_virt_aarch64_isolation
```

Combines hierarchy fault handling with the FS stack: after `crash-demo` is suspended, `debug-handler` notifies `fs-client`, which must still get `lerux-fs: round-trip ok`. Trust map: [`security.md`](security.md).

## Related

- [ADR-005](decisions/005-debug-pd.md)
- [security.md](security.md) — Phase 60 threat model + isolation
- Microkit hierarchy example (upstream `example/hierarchy`)
- [boards.md](boards.md) — `qemu_virt_aarch64_debug`, `qemu_virt_aarch64_isolation`
