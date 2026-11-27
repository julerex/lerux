# lerux

## Overview

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored**. `just build-sdk` builds it from the checkout in `deps/workspace/`. `just fetch-sdk` downloads the prebuilt Microkit SDK 2.3.0. lerux-owned code is Rust protection domains, shared crates, and host tooling.

## Quick start

Build one UEFI USB ISO and boot it on the Acer Swift X or on another x86 PC. `just make-lerux-iso-swift` packs `swift_sfx14_41g`. The stub probes the firmware framebuffer. When that buffer is the Swift panel (`0xfc90000000`, above 4 GiB) the image paints the panel and keeps `setup-wifi`. On the Z97 desk, the Inspiron, and any other x86 machine it paints the VGA text page at `0xb8000`. It does not map the Inspiron AHCI disk, and it does not touch the Swift NVMe unless that probe selected the panel. First boot, including Wi-Fi: [`docs/setup.md`](docs/setup.md). Board reference: [`docs/boards.md`](docs/boards.md#acer-swift-x-install-path).

**Prerequisites:** Linux, `git`, `just`, `rustup`, `cmake`, `ninja`, `libclang-dev`, `clang` (for the Multiboot 2 trampoline), and a USB flash drive. Packing the ISO needs `xorriso` and `grub-mkimage`. On Arch those packages are `libisoburn` and `grub` (`sudo pacman -S libisoburn grub`). Debian packages are `xorriso` and `grub-efi-amd64-bin`. Do not overwrite the host OS disk.

```bash
just fetch          # clone seL4 + microkit into deps/workspace/
just fetch-sdk      # download prebuilt Microkit SDK 2.3.0
# or: just build-sdk   # build the SDK from source instead
just make-lerux-iso-swift # pack build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso
just burn-lerux-iso-swift # pack that ISO, then write it to a USB stick
just burn-lerux-iso-z97   # same for the Z97 I217 shell (also -inspiron)
just boot-lerux-usb       # one-shot UEFI boot of that stick, then back to this host OS
```

On the build machine itself, `just boot-lerux-usb` sets UEFI `BootNext` for one reboot and leaves the lasting boot order alone, so Omarchy (or whatever already boots) returns on the next power-on. Otherwise reboot and open the firmware boot menu. Choose the USB entry that says **UEFI** (GRUB `EFI/BOOT/BOOTX64.EFI`). GRUB reads `/boot/grub/grub.cfg` and boots immediately. It Multiboot 2-loads `/boot/mb2-stub.elf` with modules `/boot/sel4.elf` and `/boot/loader.img`. The stub places the kernel at 1 MiB after `ExitBootServices`.

On the Swift the panel is the firmware GOP buffer. On 2026-10-29 it showed `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`. The shell band is 240 by 40 cells of the VGA 8×16 font, light gray on black. The sign-on is `Lerux` and the short git describe, the login line, the archiso message of the day, and `[root@lerux ~]# `. The keyboard is the built-in i8042. This machine has no COM port. The prompt runs the shell, including `setup-wifi`. Device rows sit below the band. Device roles: [`docs/platforms.md`](docs/platforms.md#swift-x-devices). On a generic x86 PC the same ISO uses the 80×25 VGA text page and leaves Swift Wi-Fi, the NVMe window, and the Inspiron disk alone. A QEMU hello line does not prove the panel.

## Quick start (QEMU)

**Prerequisites:** the same host tools as above, plus `qemu-system-aarch64`. x86 QEMU smokes also need `ovmf`, `xorriso`, and `grub-mkimage` (Arch: `libisoburn` and `grub`. Debian: `xorriso` and `grub-efi-amd64-bin`). Optionally the [ARM GNU bare-metal toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (`aarch64-none-elf-gcc`, 12.2.Rel1) for `just build-sdk`. Python 3 is only required for `just build-sdk` (upstream Microkit `build_sdk.py`).

```bash
just fetch          # clone seL4 16.0.0 + microkit 2.3.0
just build-sdk      # build Microkit SDK from source (auto-downloads ARM toolchain if needed)
# or: just fetch-sdk   # download prebuilt SDK 2.3.0 (no compile step)
just qemu           # browser and maths menu on qemu_virt_aarch64_browser_sddf (Ctrl-A x to quit)
just qemu-x86-64    # filesystem shell on x86_64_generic_fs_sddf
just run            # hello image on qemu_virt_aarch64 (override with BOARD=… just run)
```

Smoke test:

```bash
just test
```

`just test` with no `BOARD` boots `qemu_virt_aarch64_browser_sddf` and waits at the menu, `1 browser` or `2 maths`. `just qemu` leaves that menu running. Quit with `Ctrl-A x`. The smoke matrix and `just test-all` are in [`docs/ci.md`](docs/ci.md).

## Documentation

| Doc | Purpose |
|-----|---------|
| [docs/README.md](docs/README.md) | Documentation index |
| [docs/setup.md](docs/setup.md) | First boot of the Swift X live image: build the ISO, write the stick, and join Wi-Fi |
| [docs/boards.md](docs/boards.md) | Board reference and the Swift X install path |
| [docs/ci.md](docs/ci.md) | CI pipeline, caches, troubleshooting |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Layers, dependency hierarchy, and call graph |
| [docs/platforms.md](docs/platforms.md) | Platform notes and device maps |
| [docs/acknowledgements.md](docs/acknowledgements.md) | Dependencies and inspirations |

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.
