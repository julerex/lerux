# Boards

Board names are the `BOARD=` value for `just run`, `just test`, and `just build`. Metadata lives in [`scripts/board_config.py`](../scripts/board_config.py).

## Reference

| Board | Arch | Smoke command | PDs (summary) |
|-------|------|---------------|---------------|
| `qemu_virt_aarch64` | aarch64 | `just test` | hello + serial |
| `qemu_virt_aarch64_echo` | aarch64 | `just test-echo` | echo client/server + serial |
| `qemu_virt_aarch64_virtio` | aarch64 | `just test-virtio` | hello + serial + virtio blk/net |
| `qemu_virt_aarch64_init` | aarch64 | `just test-init` | boot-init + PL031 + SP804 + serial |
| `qemu_virt_aarch64_composed` | aarch64 | `just test-composed` | boot-init + hello virtio + all drivers |
| `qemu_virt_riscv64` | riscv64 | `just test-riscv` | hello + serial (MMIO UART) |
| `qemu_virt_riscv64_echo` | riscv64 | `just test-riscv-echo` | echo + serial |
| `qemu_virt_riscv64_virtio` | riscv64 | `just test-riscv-virtio` | hello + serial + virtio |
| `x86_64_generic` | x86_64 | `BOARD=x86_64_generic just test` | hello + serial (COM1) |
| `x86_64_generic_echo` | x86_64 | `just test-x86-echo` | echo + serial |

## SDK boards

`just build-sdk` compiles kernel + loader for Microkit board names (not always identical to lerux `BOARD`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64 just build-sdk
```

CI sets this via `MICROKIT_BOARDS` in the workflow env.

## QEMU profiles

| `qemu` field | Used by | Extra QEMU args |
|--------------|---------|-----------------|
| `aarch64` | hello, echo | stock `qemu-system-aarch64` virt |
| `aarch64_init` | init | patched SP804 QEMU |
| `aarch64_virtio` | virtio | virtio-net + virtio-blk + `disk.img` |
| `aarch64_composed` | composed | patched SP804 QEMU + virtio + `disk.img` |
| `riscv64` | riscv hello/echo | `-kernel loader.img` |
| `riscv64_virtio` | riscv virtio | MMIO virtio buses + `disk.img` |
| `x86_64` | x86 boards | `-kernel sel4_32.elf` + `-initrd loader.img` |

## Composed board

`qemu_virt_aarch64_composed` runs two app PDs in one system:

- **boot-init** — RTC + SP804 via serial IPC (owns the serial driver channel).
- **hello** — virtio blk/net via debug-print; waits for `boot-init` notify before probing virtio.

See [plan.md](plan.md) Phase 15.