# lerux

## Overview

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored**. `just build-sdk` builds it from the checkout in `deps/workspace/`. `just fetch-sdk` downloads the prebuilt Microkit SDK 2.3.0. lerux-owned code is Rust protection domains, shared crates, and host tooling.

## Quick start

Build the Acer Swift X image as a UEFI USB ISO, write it to a flash drive, and boot that drive with **UEFI**. The board is `swift_sfx14_41g` (Acer Swift X SFX14-41G). Full procedure: [`docs/boards.md`](docs/boards.md#acer-swift-x-install-path). The Gigabyte Z97-D3H image is `just iso pc_z97_d3h` ([`docs/boards.md`](docs/boards.md#gigabyte-z97-d3h-install-path)).

**Prerequisites:** Linux, `git`, `just`, `rustup`, `cmake`, `ninja`, `libclang-dev`, `clang` (for the Multiboot 2 trampoline), and a USB flash drive. Packing the ISO needs `xorriso` and `grub-mkimage`. On Arch those packages are `libisoburn` and `grub` (`sudo pacman -S libisoburn grub`). Debian packages are `xorriso` and `grub-efi-amd64-bin`. Do not overwrite the host OS disk.

```bash
just fetch          # clone seL4 + microkit into deps/workspace/
just fetch-sdk      # download prebuilt Microkit SDK 2.3.0
# or: just build-sdk   # build the SDK from source instead
just iso            # pack build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso
lsblk               # find the flash drive (e.g. /dev/sdX) — not the OS disk
sudo dd if=build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

Reboot and open the firmware boot menu. Choose the USB entry that says **UEFI** (GRUB `EFI/BOOT/BOOTX64.EFI`). GRUB reads `/boot/grub/grub.cfg` and boots immediately. It Multiboot 2-loads `/boot/mb2-stub.elf` with modules `/boot/sel4.elf` and `/boot/loader.img`. The stub places the kernel at 1 MiB after `ExitBootServices`.

The panel is the firmware GOP buffer. On 2026-10-29 it showed `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`. On 2026-10-30 it then showed `hello lerux` and the menu `1 browser`, `2 maths`, and `3 wifi`. The keyboard is the built-in i8042. This machine has no COM port. `1` or `browser` opens the test page `support/browser/lenna.html` at the top left of the panel. The heading `Lenna` is at the top, and the picture `support/browser/lenna.png` is below it. The shell echoes no key on that page, and `quit` returns the menu. `2` or `maths` reads integer expressions until `quit`. `3` or `wifi` lists the names the radio can hear, then prompts `ssid>` and `psk>`. Device roles: [`docs/platforms.md`](docs/platforms.md#swift-x-devices).

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
| [docs/boards.md](docs/boards.md) | Board reference and the Swift X install path |
| [docs/ci.md](docs/ci.md) | CI pipeline, caches, troubleshooting |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Layers, dependency hierarchy, and call graph |
| [docs/platforms.md](docs/platforms.md) | Platform notes and device maps |
| [docs/acknowledgements.md](docs/acknowledgements.md) | Dependencies and inspirations |

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.
