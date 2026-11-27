---
name: metal-boot-one-change
description: >
  Use when a metal x86 boot can see a device in MMIO but a command or DMA
  transfer never completes (xHCI stage letter s or d with cc 0, e1000e
  OpenError, a QEMU usb-kbd that types while the desk does not). One
  behaviour change per image. Check VT-d before a USB device theory.
  Open the Linux function before copying a TRB flag.
---

# One metal change, then the stage letter

The Z97 keyboard took four images because behaviour changes shipped beside the line that would have named the layer. The account is [docs/z97-keyboard-retrospective.md](../../../docs/z97-keyboard-retrospective.md). The Swift survey states the same packing rule for the MediaTek radio.

## Two layers

Port status is memory-mapped input/output (MMIO). The CPU reads connected ports with no direct memory access (DMA).

A command completion (Enable Slot, Address Device) is the eXtensible Host Controller Interface (xHCI) controller reading the command ring and writing the event ring. A missing command event, with ports connected, means the controller is not doing DMA. Reset recovery, packet size, and which interface is the keyboard cannot cause it.

A transfer completion (GET_DESCRIPTOR and later setup requests) starts only after Address Device has completed. A timeout there is the transfer ring, the Transfer Request Block (TRB) flags, or the endpoint context.

`cc 0` means no event arrived. It is not a device stall (completion code 6).

## One behaviour change

Ship at most one behaviour change in an image the user will boot. A line that only reports may ride along.

The stage letter is `r` reset, `s` enable slot, `a` address device, `d` device descriptor, `g` configuration descriptor, `n` no shell keyboard, `p` set configuration or boot protocol, `i` interrupt endpoint. `k` is the keyboard that bound. The formatter and its host test live in `userspace/pds/console-driver/src/usb.rs`.

If `disable_iommu` leaves the letter at `s`, the next report is the controller status register (USBSTS) halted and host-system-error bits. Do not add that field before a boot has stayed at `s`.

Add `ep N` only after a `d`, `g`, or `p` timeout. The values match Linux `EP_STATE_*` in `drivers/usb/host/xhci.h`: 0 disabled, 1 running, 2 halted, 3 stopped, 4 TRB error. `ep 1` with `cc 0` means the controller still holds the transfer. `ep 2` or `ep 4` means the endpoint context or the device.

## Before the first metal boot of an x86 DMA driver

1. Read `CONFIG_IOMMU` in `deps/microkit-sdk/board/x86_64_generic/release/include/kernel/gen_config.h`. The SDK kernel turns Intel VT-d (Virtualization Technology for Directed I/O) on when the firmware DMA Remapping (DMAR) table is present. Microkit maps no device into an input–output memory management unit (IOMMU) domain.
2. On the target, read the DMAR table (`journalctl -k | rg DMAR`). A unit with flags `0x1` covers every PCI device. This Z97 build machine is that desk.
3. Boot QEMU with `-machine q35,kernel-irqchip=split -device intel-iommu,intremap=on` before burning a stick. A fault `vtd_iommu_translate` at `0x62000100` is the xHCI event-ring segment table (`DMA_PHYS + 0x100`). The I217 window is `0x6000_0000`. The fixed image prints `Boot config: disable_iommu = true`. The option string is in `sel4.elf`. The boot trampoline `support/mb2-stub` forwards the Multiboot2 command-line tag.

A QEMU `usb-kbd` without `intel-iommu` does not see VT-d, and QEMU accepts a Setup TRB that has the Chain bit set. Say which metal property the run cannot see before counting it.

## Cite the file you opened

Linux `xhci_queue_ctrl_tx` (`drivers/usb/host/xhci-ring.c`) does not set `TRB_CHAIN` on the Setup, Data, or Status TRB of a control transfer. Setup is `TRB_IDT | TRB_TYPE(TRB_SETUP)` plus Transfer Type. Linux still tracks that request as one software transfer descriptor. Do not write "three transfer descriptors" or "the bit is illegal" unless the specification section was opened.

`xhci_setup_addressable_virt_dev` (`xhci-mem.c`) starts full speed at 64 bytes and SuperSpeed at 512, and sets endpoint 0 `tx_info` to `EP_AVG_TRB_LENGTH(8)` so Max ESIT Payload is 0. Matching that function is not proof the Z97 controller requires the edit. The transceiver on port 4 is full speed, so a USB 3 packet-size change cannot be what made it type.

If the Linux file or the specification is not on the machine, say so in the reply. A claim from memory stays a hypothesis until the next boot's letter isolates it.
