# QEMU Bring-up for lerux

This directory contains **lerux-only** tooling for running the vendored Redox kernel under QEMU. It is not part of upstream `redox-os/kernel`. See **[../../docs/vendored.md](../../docs/vendored.md)** for how lerux diverges from Redox (boot paths, loaders, build layout).

The kernel still uses Redox's custom **`KernelArgs` handoff** (not multiboot2/Limine). Upstream expects the Redox bootloader to supply that structure; lerux can synthesize it via the `direct-boot` feature (`just qemu-direct`) or via the loaders in this directory.

## Current Status

- Kernel builds cleanly with no nasm requirement (see root of the project).
- The kernel uses a **custom handoff format** (`KernelArgs` struct) rather than standard multiboot2 or Limine protocol.
- The bootloader (or our loader here) is responsible for:
  - Entering long mode (x86_64).
  - Setting up basic paging (identity map low memory + the higher-half mapping at `0xFFFFFFFF80000000`).
  - Loading the kernel ELF (respecting the `AT(...)` load addresses from the linker script).
  - Allocating a stack.
  - Constructing a valid `KernelArgs` (memory map in "areas", env block, hwdesc/RSDP or DTB, contiguous bootstrap/initfs region).
  - Placing the pointer to `KernelArgs` in `RDI` and jumping to the `kstart` symbol.

## Boot Handoff Details (x86_64)

From `kernel/src/arch/x86_shared/start.rs`:

- Entry symbol: `kstart` (naked).
- On entry the bootloader must have `RDI` = pointer to `KernelArgs`.
- `kstart` sets up an internal stack and passes `RSI` = stack_end to the Rust `start(args_ptr, stack_end)`.
- The kernel then does its own GDT/IDT/RMM/paging setup (it expects a minimal environment from the loader).

See:
- `kernel/src/startup/mod.rs` for the exact `KernelArgs` layout.
- `linkers/x86_64.ld` for the virtual/physical layout (higher half, dummy zero page hack for some bootloaders).

## Quick Start (Recommended First Smoke Test)

The most reliable way to run the bring-up right now:

```sh
cd qemu
# 1. Build the kernel (adjust features as needed)
make -C .. BUILD=build KERNEL_CARGO_FEATURES=serial_debug all

# 2. Run the improved launcher (now has nice colored feedback + nasm detection)
./run.sh
```

### Most reliable manual QEMU invocation (works on almost every system)

```sh
qemu-system-x86_64 \
  -m 512M \
  -smp 1 \
  -serial stdio \
  -display none \
  -no-reboot \
  -device loader,file=loader.bin,addr=0x100000 \
  -device loader,file=build/kernel,addr=0x200000   # optional: place kernel at a known phys addr
```

Then in the QEMU monitor (press **Ctrl+A** then **c**):
```
set $pc = 0x100000
c
```

You should see the loader run and (once we improve module loading) the kernel's early messages.

## Files in This Directory

- `README.md` — this file.
- `run.sh` — the main launch script.
- `loader.asm` + `loader.S` + `loader.ld` — two versions of a minimal x86_64 long-mode loader:
  - `loader.asm` (NASM) — preferred when nasm is available.
  - `loader.S` (GNU as) — fallback that works with the binutils `as` that ships on most Linux systems. The `run.sh` automatically chooses the best one available.
- `minimal-bootstrap.tar` (generated or empty) — placeholder for the bootstrap/initfs region the kernel expects.

## Roadmap / Next Steps for Bring-up

1. **BSP-only smoke test** (current goal) — reach `kmain`, see the first logs, hit the first panic in a controlled way.
2. Provide a real (small) initfs containing at least a "hello world" Rust binary + the minimum schemes the kernel spawns.
3. Improve the loader to be written in Rust (no nasm for the loader either) — fits the "Only Rust" spirit.
4. Add Limine support (either teach the kernel the Limine protocol, or keep a tiny bridge loader).
5. Full multi-core bring-up (fix/verify the pure-Rust trampoline bytes under real AP startup).
6. ACPI / device bring-up, graphical debug, etc.
7. Automated `cargo test` style integration tests that boot under QEMU and assert on serial output.

## Tips

- Use `-s -S` + GDB (or the `gdb=yes` style from the old Makefile docs) for debugging.
- Add `log-level=info` or kernel features as needed via environment variables passed to `run.sh`.
- Start with `-smp 1` and the `multi_core` feature off.
- The graphical debug window may appear; you can also force serial-only.

## References

- Original Redox kernel docs (building + debugging section).
- `kernel/src/startup/mod.rs` and `arch/x86_shared/start.rs`.
- Limine documentation (the linker script has comments about Limine compatibility).

**Detailed boot walk-through**: see [qemu-x86-boot-sequence.md](qemu-x86-boot-sequence.md) for a line-by-line account from the Xen PVH note through `kstart`, `start`, `kmain`, and the scheduler (with exact source locations and real serial output).

Let's get this thing booting!
