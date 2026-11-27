# bend_version

This folder is a rewrite of lerux in Bend 2. It is Bend source. It does not wrap the Rust tree, and it does not carry the Bend compiler.

The first slice is the smallest lerux boot: one protection domain that prints a ready line.

## Language entry

The language is Bend 2 from [https://github.com/bendlang/bend](https://github.com/bendlang/bend). Bend 1 programs do not carry over. This slice was written against Bend 2.0.40.

`main.bend` is the entry. Bend runs its `main`. That `main` calls `hello.bend`, the userspace program of the hello protection domain. `LAWS.bend` states the two rules that program must keep, and `PROOF.bend` proves them.

## How to run

Install Bend 2 with the upstream installer. The installer puts the compiler under `~/.bend`. Do not copy that compiler into this repository.

```bash
curl -fsSL https://bend-lang.com/install.sh | sh
export PATH="$HOME/.bend/bin:$PATH"
```

From the repository root:

```bash
bend bend_version/main.bend
bend bend_version/PROOF.bend
```

`bend bend_version/main.bend` prints this line and exits:

```text
lerux: Hello from Rust on seL4 Microkit!
```

The letters seL4 in that line are the serial spelling of Security Enhanced L4. `bend bend_version/PROOF.bend` prints `ALL PROOFS CHECK` when both laws hold. `bend bend_version/main.bend --check-only` checks the entry and does not run it.

`bend bend_version/PROOF.bend --verdict` asks for a second check by the BendTT (Bend type theory) kernel. That kernel needs Lean v4.34.0 (`leanprover/lean4:v4.34.0`), or `BENDTT` pointing at a kernel that is already built. This slice does not fetch Lean.

## What the program does

On the Quick Emulator (QEMU) board `qemu_virt_aarch64`, the smallest lerux boot is the `hello` protection domain. The same ready line is what `qemu_virt_riscv64`, `x86_64_generic`, and `rpi4b_4gb` expect. Init logs:

```text
lerux: Hello from Rust on seL4 Microkit!
```

`hello.bend` is that userspace program. `ready_line` is the serial line. `init` prints it and returns a handler whose constructor is `Booted`. `notified` takes that handler and returns the same handler beside `Unit`. `settle` calls `notified` once and keeps only that `Unit`. `main.bend` runs `init`, then `settle`, then returns. Nothing else is printed.

`Booted` is the constructor name because Bend's base library already uses `Ready`.

## What Bend cannot express

Bend 2 compiles to C, Metal, CUDA (Compute Unified Device Architecture), and JavaScript. Its effects include print, files, and sockets. It has no Security Enhanced L4 (seL4) capabilities, no Microkit protection-domain entry, and no notification or interrupt runtime. `hello.bend` is the userspace rewrite of the hello protection domain. It is not a kernel, and it does not boot on Security Enhanced L4 (seL4).

## Laws

`ready_line` says `Hello.ready_line` is the serial line above. `handler_stays` says `Hello.notified` returns the handler it was given, beside `Unit`.
