# Boards

Board names are the `BOARD=` value for `just run`, `just test`, and `just build`. Metadata lives in [`support/boards.toml`](../support/boards.toml).

## Reference

| Board | Arch | Smoke command | PDs (summary) |
|-------|------|---------------|---------------|
| `qemu_virt_aarch64` | aarch64 | `just test` | hello + serial |
| `qemu_virt_aarch64_echo` | aarch64 | `just test-echo` | echo client/server + serial |
| `qemu_virt_aarch64_virtio` | aarch64 | `just test-virtio` | hello + serial + virtio blk/net |
| `qemu_virt_aarch64_blk` | aarch64 | `just test-blk` | blk client/server + serial + virtio-blk |
| `qemu_virt_aarch64_blk_composed` | aarch64 | `just test-blk-composed` | boot-init + init drivers + blk IPC + virtio-blk |
| `qemu_virt_aarch64_init` | aarch64 | `just test-init` | boot-init + PL031 + SP804 + serial |
| `qemu_virt_aarch64_composed` | aarch64 | `just test-composed` | boot-init + hello virtio + all drivers |
| `qemu_virt_aarch64_http` | aarch64 | `just test-http` | serial + virtio-net + http-server |
| `qemu_virt_aarch64_http_composed` | aarch64 | `just test-http-composed` | boot-init + init drivers + virtio-net + http-server |
| `qemu_virt_riscv64` | riscv64 | `just test-riscv` | hello + serial (MMIO UART) |
| `qemu_virt_riscv64_echo` | riscv64 | `just test-riscv-echo` | echo + serial |
| `qemu_virt_riscv64_virtio` | riscv64 | `just test-riscv-virtio` | hello + serial + virtio |
| `qemu_virt_riscv64_blk` | riscv64 | `just test-riscv-blk` | blk client/server + serial + virtio-blk |
| `qemu_virt_riscv64_http` | riscv64 | `just test-riscv-http` | serial + virtio-net + http-server |
| `x86_64_generic` | x86_64 | `BOARD=x86_64_generic just test` | hello + serial (COM1) |
| `x86_64_generic_echo` | x86_64 | `just test-x86-echo` | echo + serial |
| `x86_64_generic_virtio` | x86_64 | `just test-x86-virtio` | hello + serial + virtio-pci blk/net |
| `x86_64_generic_blk` | x86_64 | `just test-x86-blk` | blk client/server + serial + virtio-pci blk |
| `x86_64_generic_http` | x86_64 | `just test-x86-http` | serial + virtio-pci net + http-server |

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
| `aarch64_blk` | blk | virtio-blk + `disk.img` |
| `aarch64_blk_composed` | blk-composed | patched SP804 QEMU + virtio-blk + `disk.img` |
| `aarch64_composed` | composed | patched SP804 QEMU + virtio + `disk.img` |
| `aarch64_http` | http | virtio-net + `hostfwd=tcp::18080-:8080` |
| `aarch64_http_composed` | http-composed | patched SP804 QEMU + virtio-net + `hostfwd` |
| `riscv64` | riscv hello/echo | `-kernel loader.img` |
| `riscv64_virtio` | riscv virtio | MMIO virtio buses + `disk.img` |
| `riscv64_blk` | riscv blk | MMIO virtio-blk bus.0 + `disk.img` |
| `riscv64_http` | riscv http | MMIO virtio-net bus.1 + `hostfwd=tcp::18080-:8080` |
| `x86_64` | x86 hello/echo | `-machine q35` + `-kernel sel4_32.elf` + `-initrd loader.img` |
| `x86_64_virtio` | x86 virtio | q35 + PCI virtio-blk/net + `disk.img` |
| `x86_64_blk` | x86 blk | q35 + PCI virtio-blk + `disk.img` |
| `x86_64_http` | x86 http | q35 + PCI virtio-net + `hostfwd=tcp::18080-:8080` |

## Composed board

`qemu_virt_aarch64_composed` runs two app PDs in one system:

- **boot-init** — RTC + SP804 via serial IPC.
- **hello** — virtio blk/net via serial IPC; waits for `boot-init` notify before probing virtio.

See [plan.md](plan.md) Phases 15 and 24.

## HTTP boards

`qemu_virt_aarch64_http` serves `GET /` on guest port **8080** (`10.0.2.15`). QEMU user netdev forwards host `127.0.0.1:18080` → guest `:8080`; smoke uses `curl` after serial shows `lerux-http: listening`.

`qemu_virt_aarch64_http_composed` runs boot-init (RTC + SP804) then http-server over virtio-net — same notify gate as composed hello. See [plan.md](plan.md) Phase 17.

`x86_64_generic_http` uses the same HTTP PD and hostfwd layout on QEMU **q35** with PCI virtio-net via `virtio-pci-driver` (net-only). See [plan.md](plan.md) Phase 19.

`qemu_virt_riscv64_http` serves HTTP over MMIO virtio-net on `virtio-mmio-bus.1` (same layout as riscv virtio hello). See [plan.md](plan.md) Phase 22.

### x86 HTTP inbound (operational notes)

On x86, `http-server` returns from `init()` after printing `lerux-http: listening` and handles inbound `GET /` via virtio-pci-driver notifications (same model as aarch64 HTTP).

**Automated smoke (preferred):**

```bash
just test-x86-http
```

`lerux test` retries HTTP checks for up to 30s and always terminates QEMU on exit (avoids orphan instances on port 18080).

**Interactive QEMU:**

```bash
BOARD=x86_64_generic_http just qemu-x86_64-http
# other terminal, after "listening" (brief pause or retry helps):
sleep 1 && curl http://127.0.0.1:18080/
```

**Port 18080 — one listener at a time.** Host port 18080 is shared by:

| Consumer | Command / context |
|----------|-------------------|
| x86/aarch64/riscv HTTP hostfwd | `just test-x86-http`, `just test-http`, `just test-riscv-http` |
| TCP echo (virtio outbound tests) | `just test-x86-virtio`, `cargo run -p lerux-cli -- tcp-echo 18080` |

Do **not** run background QEMU and `just test-x86-http` concurrently. A stale QEMU or leftover `tcp-echo-server` on 18080 makes `curl` hit the wrong endpoint and time out even when the new guest has reached `listening`.

**Cleanup before retry:**

```bash
pkill -f 'tcp-echo 18080'
pkill -f 'qemu-system-x86_64.*hostfwd=tcp::18080-:8080'
just test-x86-http
```

`just qemu-x86_64-http` and the `x86_64_http` smoke recipe run the same `pkill` patterns before starting QEMU.