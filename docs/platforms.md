# Platform tiers (Phase 59)

“Workstation” is a **product concept** (supervisor + FS + net + shell + apps), not a single board name. Profiles select the layout; boards supply arch-specific drivers and QEMU/hardware vars.

## Tiers

| Tier | Platforms | Profile / board examples | Smoke |
|------|-----------|---------------------------|-------|
| **1** | aarch64 QEMU virt; RPi4 | `workstation` → `qemu_virt_aarch64_workstation`; `workstation-rpi4` | CI: `workstation`; HW optional |
| **2** | RISC-V virt; x86_64 q35 | `workstation-riscv`, `workstation-x86` | CI: `workstation-riscv`, `workstation-x86` |
| **3** | Other Microkit boards | Bring-up boards only (serial/echo/virtio slices) | Per-board |
| **HW x86** | Gigabyte Z97-D3H (this desktop) | Phase 82: `pc_z97_d3h` VGA shell + PS/2 or USB boot keyboard + COM1; native disk and Ethernet are [Milestones 19–22](plan-lionsos.md#milestone-19--advanced-host-controller-interface-disk) | Optional `test-hw` |

## Arch drivers

| Role | aarch64 virt | RISC-V virt | x86_64 q35 | Z97-D3H metal |
|------|--------------|-------------|------------|----------------|
| Serial | PL011 + serial-virt | NS16550 MMIO + serial-virt | [q35 devices](#q35-devices) | [Z97 desk devices](#z97-desk-devices) |
| Block | virtio-blk MMIO | virtio-blk MMIO | [q35 devices](#q35-devices) | [Z97 desk devices](#z97-desk-devices) |
| Net | virtio-net MMIO (unified-dma) | virtio-net MMIO (unified-dma) | [q35 devices](#q35-devices) | [Z97 desk devices](#z97-desk-devices) |
| RTC | PL031 | Goldfish RTC | [q35 devices](#q35-devices) | [Z97 desk devices](#z97-desk-devices) |
| Timer | SP804 (patched QEMU) | `rdtime` CSR | [q35 devices](#q35-devices) | [Z97 desk devices](#z97-desk-devices) |

App channel ends (shell↔fs/net/apps, log, config) are **shared** across workstation profiles; only driver protection-domain names and layout templates change.

## x86 device map

q35 and the Z97 desk share a kernel and do not share a chipset. Names that both machines use are defined in [GLOSSARY.md](../GLOSSARY.md). A row is one device and the protection domain that programs it. "In the tree" means that protection domain crate exists today.

### q35 devices

QEMU machine `q35` with OVMF and the Limine ISO (same path as metal UEFI). The virtio boards stay. Milestones 19 and 20 add `ich9-ahci` and an 82574 on new boards.

| Category | Device | lerux driver | In the tree |
|----------|--------|--------------|-------------|
| Serial | NS16550 at COM1 (`0x3f8`, pin 4) | `serial-driver` on the hello, virtio, and console images. `sddf-serial-driver` on the device-framework images | yes |
| Text console | Standard VGA, VGA text page at `0xb8000`, cursor through the CRTC | `console-driver` on the console image | yes |
| Keyboard | i8042 PS/2 controller | `console-driver` on the console image | yes |
| Keyboard | `qemu-xhci` and a USB boot keyboard, attached by `just test-x86-usb-kbd` | `console-driver` | yes |
| Wall clock | CMOS real-time clock | `cmos-rtc-driver` on the init image | yes |
| Interval timer | time-stamp counter | `tsc-timer-driver` on the init image | yes |
| Disk | `virtio-blk-pci` | `virtio-pci-driver`, and `sddf-blk-driver` on the device-framework images | yes |
| Network | `virtio-net-pci` | `virtio-pci-driver`, and `sddf-net-driver` on the device-framework images | yes |
| Disk | `ich9-ahci` (`8086:2922` at `00:1f.2`), Advanced Host Controller Interface | `sddf-ahci-driver`, [Milestone 19](plan-lionsos.md#milestone-19--advanced-host-controller-interface-disk) | yes |
| Network | `e1000e`, the 82574 (`8086:10d3`) | `sddf-e1000e-driver`, [Milestone 20](plan-lionsos.md#milestone-20--qemu-ethernet) | yes |

### Z97 desk devices

The Gigabyte GA-Z97-D3H. Firmware scanout uses the HD Graphics 4600 when Initial Display Output is `IGFX`. The RX 560 stays unused. The onboard Ethernet port is the I217-V. The RTL8852BE is a wireless card, and the probe only reads its configuration space.

| Category | Device | lerux driver | In the tree |
|----------|--------|--------------|-------------|
| Serial | NS16550 at COM1 (`0x3f8`), COMA header | `serial-driver` | yes |
| Text console | VGA text page at `0xb8000`, cursor through the CRTC. The HD Graphics 4600 scans it out. The Intel display engine is not programmed | `console-driver` | yes |
| Linear framebuffer | The mode firmware publishes (recorded boot: `0xd0000000`, 1024 by 768). The HD Graphics 4600 scans it out | none. The desk-browser spec names `browser-ui` to map this buffer | no `browser-ui` crate |
| Keyboard | i8042 PS/2 controller, rear combo port | `console-driver` | yes |
| Keyboard | Intel xHCI at `00:14.0` and a USB boot keyboard on a root port | `console-driver` | yes |
| Wall clock | CMOS real-time clock | `cmos-rtc-driver` on the init image | yes |
| Interval timer | time-stamp counter | `tsc-timer-driver` on the init image | yes |
| Disk | Z97 Serial ATA controller in Advanced Host Controller Interface mode. PCI identity not recorded | `sddf-ahci-driver`, [Milestone 22](plan-lionsos.md#milestone-22--desk) | no |
| Network | I217-V, `8086:153b` at `00:19.0` | `sddf-e1000e-driver`, [Milestone 22](plan-lionsos.md#milestone-22--desk) | no |
| Wireless | RTL8852BE, `10ec:b852`, behind root port `00:1c.6` | `rtl8852be-probe` reads configuration space and does not map the card | yes, probe only |
| Display | HD Graphics 4600, `8086:0412` at `00:02.0` | none | no |
| Display | RX 560, `1002:67ff` at `01:00.0` | none | no |

## Commands

```bash
just test-workstation          # Tier 1 aarch64 (SP804 QEMU)
just test-workstation-riscv    # Tier 2 RISC-V
just test-workstation-x86      # Tier 2 x86
just iso                                 # Z97-D3H hybrid USB ISO (BIOS/CSM Limine, sel4_32.elf; does not write a disk)
just deploy-pc DEST=/path/to/usb-fat      # Z97-D3H VGA shell onto a FAT volume (Multiboot 2)

lerux profile build workstation
lerux profile build workstation-riscv
lerux profile build workstation-x86
```
