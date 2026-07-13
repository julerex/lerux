# Platform tiers (Phase 59)

“Workstation” is a **product concept** (supervisor + FS + net + shell + apps), not a single board name. Profiles select the layout; boards supply arch-specific drivers and QEMU/hardware vars.

## Tiers

| Tier | Platforms | Profile / board examples | Smoke |
|------|-----------|---------------------------|-------|
| **1** | aarch64 QEMU virt; RPi4 | `workstation` → `qemu_virt_aarch64_workstation`; `workstation-rpi4` | CI: `workstation`; HW optional |
| **2** | RISC-V virt; x86_64 q35 | `workstation-riscv`, `workstation-x86` | CI: `workstation-riscv`, `workstation-x86` |
| **3** | Other Microkit boards | Bring-up boards only (serial/echo/virtio slices) | Per-board |

## Arch drivers

| Role | aarch64 virt | RISC-V virt | x86_64 q35 |
|------|--------------|-------------|------------|
| Serial | PL011 + serial-virt | NS16550 MMIO + serial-virt | COM1 ioport + serial-virt |
| Block | virtio-blk MMIO | virtio-blk MMIO | virtio-pci combo |
| Net | virtio-net MMIO (unified-dma) | virtio-net MMIO | virtio-pci combo |
| RTC | PL031 | Goldfish RTC | CMOS |
| Timer | SP804 (patched QEMU) | `rdtime` CSR | TSC |

App channel ends (shell↔fs/net/apps, log, config) are **shared** across workstation profiles; only driver PD names and layout templates change.

## Commands

```bash
just test-workstation          # Tier 1 aarch64 (SP804 QEMU)
just test-workstation-riscv    # Tier 2 RISC-V
just test-workstation-x86      # Tier 2 x86

lerux profile build workstation
lerux profile build workstation-riscv
lerux profile build workstation-x86
```
