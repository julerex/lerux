# Platform tiers (Phase 59)

“Workstation” is a **product concept** (supervisor + FS + net + shell + apps), not a single board name. Profiles select the layout; boards supply arch-specific drivers and QEMU/hardware vars.

## Tiers

| Tier | Platforms | Profile / board examples | Smoke |
|------|-----------|---------------------------|-------|
| **1** | aarch64 QEMU virt; RPi4 | `workstation` → `qemu_virt_aarch64_workstation`; `workstation-rpi4` | CI: `workstation`; HW optional |
| **2** | RISC-V virt; x86_64 q35 | `workstation-riscv`, `workstation-x86` | CI: `workstation-riscv`, `workstation-x86` |
| **3** | Other Microkit boards | Bring-up boards only (serial/echo/virtio slices) | Per-board |
| **HW x86** | Gigabyte Z97-D3H (this desktop) | Phase 82: `pc_z97_d3h` VGA shell + PS/2 + COM1; native e1000e/AHCI not yet | Optional `test-hw` |

## Arch drivers

| Role | aarch64 virt | RISC-V virt | x86_64 q35 | Z97-D3H metal |
|------|--------------|-------------|------------|----------------|
| Serial | PL011 + serial-virt | NS16550 MMIO + serial-virt | COM1 ioport + serial-virt | COM1 log (`pc_z97_d3h`); VGA text shell on the monitor, PS/2 keyboard |
| Block | virtio-blk MMIO | virtio-blk MMIO | virtio-pci combo | *(AHCI later)* |
| Net | virtio-net MMIO (unified-dma) | virtio-net MMIO (unified-dma) | virtio-pci combo (unified-dma) | *(e1000e later)* |
| RTC | PL031 | Goldfish RTC | CMOS | CMOS (not in hello slice) |
| Timer | SP804 (patched QEMU) | `rdtime` CSR | TSC | TSC (not in hello slice) |

App channel ends (shell↔fs/net/apps, log, config) are **shared** across workstation profiles; only driver PD names and layout templates change.

## Commands

```bash
just test-workstation          # Tier 1 aarch64 (SP804 QEMU)
just test-workstation-riscv    # Tier 2 RISC-V
just test-workstation-x86      # Tier 2 x86
just iso                                 # Z97-D3H hybrid USB ISO (does not write a disk)
just deploy-pc DEST=/path/to/usb-fat      # Z97-D3H VGA shell onto a FAT volume (Multiboot 2)

lerux profile build workstation
lerux profile build workstation-riscv
lerux profile build workstation-x86
```
