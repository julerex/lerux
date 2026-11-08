# BlueChip: emulation feasibility and the lerux port gap

Investigation written 2026-11-08. No decision yet. This page records whether the Anduril BlueChip laptop can be emulated well enough to test lerux without the machine, what would break lerux on it, and a proxy board that covers the parts worth covering now.

## Summary

A faithful BlueChip emulator is not feasible today. The machine's specification is not public, no emulator models any Qualcomm Snapdragon SoC, and the Snapdragon X boot chain is signed Qualcomm firmware running under the Gunyah hypervisor. A QEMU proxy board that reproduces the properties of BlueChip that would break lerux is feasible, and it turns the eventual metal port into a driver exercise instead of an architecture exercise. Another Snapdragon X Elite laptop is a closer stand-in than any emulator.

## What BlueChip is

Anduril and Qualcomm announced BlueChip on 2026-10-27: a rugged, United States assembled laptop for government, defense, and first responders, built on the Snapdragon X Series. Orders open in 2027. Processor SKU, memory, display, ports, and price are not announced ([Anduril press release](https://www.anduril.com/news/anduril-and-qualcomm-unveil-bluechip-to-rebuild-american-computer-manufacturing), [TechCrunch](https://techcrunch.com/2026/10/27/anduril-qualcomm-launch-bluechip-a-new-kind-of-us-made-laptop/)).

The one hard technical fact is the SoC family. Snapdragon X (X1E80100 "Hamoa" class, or the newer X2) is characterised by the Linux mainline work ([Linaro EOSS slides](https://static.sched.com/hosted_files/eoss24/ae/EOSS-X-Elite-Linux.pdf), [Qualcomm mainline status](https://linux-msm.github.io/mainline-status/soc/hamoa)):

- Oryon cores, ARMv8.7-A.
- GICv3 with an ITS. The ITS is used for PCIe MSI on Linux.
- ARM generic timer.
- Qualcomm GENI serial engine (QUP) for the UART. Not a PL011.
- Qualcomm DWC PCIe controllers with QMP PHYs for NVMe, Wi-Fi (WCN785x, `ath12k`), and WWAN.
- Synopsys DWC3 USB, xHCI-compatible host.
- MDSS display controller with eDP. Firmware leaves a GOP framebuffer.
- Adreno GPU.
- UEFI firmware. Boot flow is ROM, XBL, Gunyah at EL2, UEFI at EL1, then the EFI application. The OS is entered at EL1. Device tree is selected by the firmware or the EFI loader for Linux; ACPI exists for Windows.

## Why a real emulator is out of reach

- No specification to emulate. Board-level facts (console UART, keyboard path, embedded controller, panel, Wi-Fi module, memory map) are exactly what an emulator models, and none are published. An emulator built now is a guess.
- No Qualcomm SoC model exists in QEMU for any Snapdragon generation ([QEMU targets](https://www.qemu.org/docs/master/system/targets.html)). A Hamoa machine needs new device models for the GENI serial engine, GCC and RPMh clocks, TLMM pinctrl, SMMU-500, the DWC PCIe controller and QMP PHY, DWC3, SPMI PMICs, PDC, and MDSS. That is an SoC bring-up in software, verifiable only against hardware.
- The boot chain cannot be reproduced. XBL and the UEFI are signed Qualcomm binaries tied to the secure boot ROM, and Gunyah holds EL2. Running that firmware in an emulator means emulating the secure world. A QEMU machine would also need a replacement open firmware.
- seL4 has no Snapdragon platform. The supported Arm list is embedded boards and `qemu-arm-virt` ([seL4 platforms](https://sel4.systems/platforms.html)). Microkit 2.3.0's board list has no Qualcomm entry. A kernel platform port (`src/plat/<name>`: device tree, GIC base, timer frequency, debug UART) and a Microkit `BoardInfo` are prerequisites for lerux on BlueChip whether or not an emulator exists.

## What would break lerux on BlueChip

The current aarch64 board is `qemu_virt_aarch64` in [`support/boards.toml`](../support/boards.toml). [`tools/lerux-cli/src/qemu.rs`](../tools/lerux-cli/src/qemu.rs) launches it as `virt,virtualization=on`, `-cpu cortex-a53`, GICv2, and injects `loader.img` with `-device loader` at `0x70000000`. The Microkit `qemu_virt_aarch64` board builds the kernel with `KernelArmHypervisorSupport`, so the loader is entered at EL2.

| BlueChip property | lerux today | Gap |
|---|---|---|
| OS entered at EL1 (Gunyah at EL2) | Hypervisor kernel, QEMU `virtualization=on` | Non-hypervisor kernel. Microkit 2.3.0 ships `qemu_virt_aarch64_el1` (`KernelArmHypervisorSupport` off) for this |
| GICv3 | GICv2 | seL4 16.0.0 `qemu-arm-virt` takes `QEMU_GIC_VERSION=3`, which sets `KernelArmGicV3`. The prebuilt SDK has no such board |
| UEFI hand-off, no GRUB Multiboot 2 | x86 uses `support/mb2-stub` ([ADR-014](decisions/014-uefi-mb2-stub.md)) | An aarch64 PE/COFF stub: `ExitBootServices`, record the GOP mode, place `loader.img`, MMU and caches off, jump. New code |
| ARMv8.7 Oryon | `cortex-a53` | seL4 `ARM_CPU` on `qemu-arm-virt` accepts `cortex-a53`, `cortex-a57`, and `cortex-a72`. Pair a `cortex-a72` kernel with QEMU `-cpu max` to shake out feature assumptions |
| GENI SE UART console | PL011 through `sel4-pl011-driver` | New driver from the `qcom_geni_serial` register map. No QEMU model. Metal only |
| NVMe over DWC PCIe | `swift-nvme` (x86, read-only) and the x86 configuration-space and MSI code | ECAM enumeration and INTx-to-SPI routing on aarch64. seL4 Arm has no MSI or ITS path, so INTx only |
| USB keyboard on DWC3, or I2C-HID (unknown) | USB HID boot keyboard in `console-driver` (x86 xHCI, MSI) | xHCI logic ports. The interrupt becomes an SPI |
| Firmware GOP framebuffer | `lerux-desk` paints the Swift GOP buffer | Arch-neutral once the stub records the mode |
| Wi-Fi WCN785x (`ath12k`) | `swift-mt7921` (MediaTek) | No QEMU model and a large driver. Not proxyable |

The Swift X image (`swift_sfx14_41g`) is the most laptop-complete lerux image and it is x86. Its i8042, VGA text page, and I/O port PCI code do not carry to aarch64. `lerux-desk` painting, the NVMe queue logic, and the USB HID logic carry when they are MMIO-only.

## Proxy board: `qemu_virt_aarch64_bluechip`

Three increments. Each is a `boards.toml` entry plus its smoke, following [`.cursor/skills/port-and-smoke/SKILL.md`](../.cursor/skills/port-and-smoke/SKILL.md).

### Tier 1: configuration

- A Microkit board variant of `qemu_virt_aarch64_el1` with `QEMU_GIC_VERSION=3` and `ARM_CPU=cortex-a72`.
- `qemu.rs` gains `virtualization=off,gic-version=3` and `-cpu max` for this board.
- The sDDF images run unchanged on it. Any that do not are real BlueChip findings about EL1 and GICv3.
- The board variant is an edit to `build_sdk.py` in the Microkit tree. [AGENTS.md](../AGENTS.md) says not to modify `deps/workspace/`. This needs a decision record for a carried patch, and a `build-sdk` path beside `fetch-sdk` in [`ci.md`](ci.md).

### Tier 2: UEFI boot

- EDK2 for aarch64 (Debian package `qemu-efi-aarch64`, QEMU `-bios QEMU_EFI.fd`) boots an EFI system partition that carries the aarch64 stub and `loader.img`.
- The stub is the aarch64 analog of `mb2-stub`: UEFI calling convention, `ExitBootServices`, write the GOP mode record, place `loader.img` at the Microkit link address, MMU and caches off, jump at EL1. No Multiboot 2 information block.
- Reuses the FAT and packing logic in [`tools/lerux-cli/src/iso.rs`](../tools/lerux-cli/src/iso.rs).
- With `virtualization=off`, EDK2 and the stub run at EL1, which mirrors the Gunyah hand-off.
- This is the most valuable piece because it is the one lerux cannot borrow from the x86 Swift path.

### Tier 3: peripheral proxies

- QEMU `nvme` on the `virt` PCIe root. ECAM, INTx on SPIs 35 to 38.
- `qemu-xhci` with `usb-kbd`, as `just test-x86-usb-kbd` already attaches on q35.
- `ramfb` for the panel.
- Each is a port of an existing lerux driver to MMIO ECAM and SPI interrupts, with a host test of the rendered system description.
- The real DWC controller's configuration space and INTx map differ. This tier validates lerux's PCIe layer, not the Qualcomm controller.

### Not worth proxying

GENI UART, Wi-Fi, PMIC, clocks, and the display controller. Firmware leaves clocks and the panel configured, so lerux can live on the GOP buffer at first. UART and Wi-Fi need hardware.

## Effort

Technical scope, not a calendar.

- Tier 1: small. One SDK board variant, two QEMU flags, one `boards.toml` entry, one decision record. The risk is the carried Microkit patch.
- Tier 2: medium. One new `no_std` PE/COFF stub for aarch64 with a linker script and the cache and MMU teardown sequence, plus partition packing and a smoke. Same shape as `mb2-stub`, different ABI.
- Tier 3: medium to large. ECAM PCI enumeration on Arm, NVMe queue rework for aarch64, xHCI interrupt through an SPI, each with a host test of the rendered system description.
- Real BlueChip port beyond the proxy: seL4 platform port and Microkit board, GENI UART driver, Qualcomm PCIe bring-up (PERST and PHY are done by firmware on laptops, the INTx map is board specific), the keyboard path once the design is known, and no Wi-Fi for a long time. All of it is gated on hardware or published device tree and ACPI tables.
- A faithful QEMU machine model of BlueChip: not a credible plan. It is a new SoC in QEMU plus an open firmware replacement, with nothing to validate against.

## A stand-in that exists

The closest thing to a BlueChip emulator today is another Snapdragon X Elite laptop. The SoC, GIC, UART, PCIe controller, and the Gunyah EL1 boot are the same family. Mainline device trees for the Lenovo ThinkPad T14s Gen 6, Lenovo Yoga Slim 7x, and Dell XPS 13 9345 give the memory map and interrupt numbers that no emulator can ([Linux on Snapdragon X](https://computingforgeeks.com/linux-snapdragon-x2-laptops/)). lerux already has the metal path: ISO packing, the `test-hw` serial smoke, and the Swift X install path in [boards.md](boards.md). A board for one of those laptops would be a more honest BlueChip stand-in than any QEMU machine, and the work transfers because Anduril is not redesigning the SoC.

## Open questions

- Which Snapdragon X SKU BlueChip ships, and whether the firmware exposes EL2.
- The console path. A laptop has no exposed UART; the Swift X path paints the panel instead. The GENI driver may only matter for a debug header.
- Keyboard: USB HID on DWC3, or I2C-HID behind the embedded controller.
- Whether the firmware publishes a device tree, ACPI only, or both.

## References

- [boards.md](boards.md)
- [platforms.md](platforms.md)
- [ADR-014](decisions/014-uefi-mb2-stub.md)
- [`support/boards.toml`](../support/boards.toml)
- [`deps/versions.toml`](../deps/versions.toml): seL4 16.0.0, Microkit 2.3.0
- [Microkit 2.3.0 `build_sdk.py`](https://github.com/seL4/microkit/blob/2.3.0/build_sdk.py): `qemu_virt_aarch64_el1`
- [seL4 16.0.0 `qemu-arm-virt` config](https://github.com/seL4/seL4/blob/16.0.0/src/plat/qemu-arm-virt/config.cmake): `QEMU_GIC_VERSION`, `ARM_CPU`
