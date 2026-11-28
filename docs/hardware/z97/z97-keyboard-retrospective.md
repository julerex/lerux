# Z97 keyboard bring-up retrospective

Last updated: 2026-11-27

The Microsoft transceiver on the Gigabyte Z97-D3H types after [PR 249](https://github.com/julerex/lerux/pull/249). Two faults were in the way, in two layers. This is why the first boots did not name them, and what a later review by Claude Opus 5.5 High would have done with the same screen lines. The working rules are in [.cursor/skills/metal-boot-one-change/SKILL.md](../.cursor/skills/metal-boot-one-change/SKILL.md).

The desk picture is the firmware linear framebuffer. The on-screen menu is the Video Graphics Array (VGA) text page, and the user was reading the `usb:` line painted there. Kernel output stays on COM1.

## What was actually wrong

**Command layer.** The seL4 kernel in the Microkit software development kit (SDK) is built with `CONFIG_IOMMU`. The Z97 firmware publishes a DMA Remapping (DMAR) table, and one remapping unit covers every PCI device. seL4 turns Intel Virtualization Technology for Directed I/O (VT-d) on from that table. Microkit maps no device into an input–output memory management unit (IOMMU) domain, so the eXtensible Host Controller Interface (xHCI) controller could not read its command ring or its event ring at `0x6200_0000`. Port status is memory-mapped input/output (MMIO). The CPU could still see that ports 4, 10, 14, and 16 were connected. Enable Slot never produced a completion event.

`disable_iommu` on the GRUB (GRand Unified Bootloader) command line, forwarded by the boot trampoline `support/boot/mb2-stub`, is the change that moved the stage letter. That commit (`a8aadcd`) changed nothing else. The next boot reached the device-descriptor stage on every port, so the command layer is proven.

**Control-transfer layer.** After Address Device, GET_DESCRIPTOR produced no transfer event on any port (`d` and completion code 0). The image that made the keyboard type (`b1599e8`) dropped the Chain bit from the Setup and Data Transfer Request Blocks (TRBs) and also wrote 0 into Max Endpoint Service Interval Time (ESIT) Payload on a control endpoint. Those two behaviour changes shipped together. The boot cannot say which one the controller required.

Linux `xhci_queue_ctrl_tx` in `drivers/usb/host/xhci-ring.c` sets neither TRB Chain. Setup is `TRB_IDT | TRB_TYPE(TRB_SETUP)` plus the Transfer Type field (the comment cites xHCI 1.0/1.1 section 6.4.1.2.1). Data is `TRB_TYPE(TRB_DATA)`, with Interrupt-on Short Packet and Direction In when the request is an IN. Status is `TRB_IOC | TRB_TYPE(TRB_STATUS)`. Linux still tracks the whole control request as one software transfer descriptor (`urb_priv->td[0]`). "Three transfer descriptors" and "the Setup TRB is not allowed the Chain bit" were stronger than that file. The specification text was not opened. During the session the Chain-bit claim came from memory.

The same commit also starts a USB 3 endpoint 0 at 512 bytes. The transceiver is full speed (`045e:07b2` on port 4). That branch does not run for it, so it is not the fix. Linux `xhci_setup_addressable_virt_dev` in `xhci-mem.c` does start full speed at 64 bytes and SuperSpeed at 512, and sets endpoint 0 `tx_info` to `EP_AVG_TRB_LENGTH(8)`, which leaves Max ESIT Payload at 0. Lerux still starts full speed at 8. The keyboard works with that, so the Linux 64-byte guess is not required for this receiver.

## Where the two approaches split

The screen lines were enough. The early images added behaviour beside the line that would have read them.

| Boot the user reported | What that line already showed | What shipped in the image being tested | What the next line could not decide |
| --- | --- | --- | --- |
| Sign-on, no keystrokes, no `usb:` line | Nothing about USB. The sign-on only shows that the shell ran. | — | — |
| `usb: xhci no keyboard, ports 4 10 14 16` | MMIO worked. Four devices were connected. No keyboard bound. A command timeout sets `broken` and stops the walk, so this line already allowed "port 4's first command never completed". | Settle wait, reject a mouse interface that precedes a keyboard interface, search every port for the mouse, and the port list with no stage letter. | Whether any of those behaviour changes mattered. The Holtek-as-keyboard theory was never on an image that could show it. |
| `4s` and `cc 0` | Port 4 reset. Enable Slot produced no event for 500 ms. The other ports were not tried. Enable Slot never touches the USB wire. | 20 ms after reset, 10 ms after Address Device, 500 ms timestamp-counter deadlines, stage letters, completion code. | The pauses and the deadlines. The letter killed the timing theory and the "timeout too short" theory. It could not kill the pauses, which are still in the driver. |
| `d` on every port, `cc 0` | Enable Slot and Address Device completed, so the rings were reachable. GET_DESCRIPTOR timed out on every device. | `disable_iommu` only. | Nothing in that image. This is the clean boot. |
| Keyboard types | Control transfers complete for the transceiver. | Chain bit cleared, Max ESIT Payload 0 on control endpoints, USB 3 packet size 512, and an `ep` field on timeout. | Which of the Chain bit and Max ESIT Payload the controller required. The 512-byte change cannot affect port 4. `ep` has not been seen on the metal. |

Claude's branch after the first `usb:` line is the command layer: connected ports and no command event means the controller is not doing direct memory access (DMA). The candidates are VT-d, a bad ring address, or the controller not running. They are not reset recovery and not which interface is the keyboard.

That was knowable before the second image. The same machine is the desk, so the DMAR table was already on it. `deps/microkit-sdk/board/x86_64_generic/release/include/kernel/gen_config.h` already had `CONFIG_IOMMU`. QEMU with `-device intel-iommu` later faulted the controller (`00:14.0`) at `0x62000100`, which is the event-ring segment table, and printed the same `s` and `cc 0` line. Those three checks happened after the timing image, not before it.

Reading the live `045e:07b2` descriptors was the right method on the wrong layer. Interface 0 is the boot keyboard and interface 1 is the mouse, so the new mouse rule was not rejecting the transceiver. No descriptor is read until after Address Device. The failure at `s` is three stages earlier.

A QEMU `usb-kbd` that types, without `intel-iommu`, does not see either metal fault. QEMU completed the chained Setup TRB. The session said that plainly only on the last image.

The same packing shows up one commit earlier in the I217 driver (`53a8a30`): timestamp-counter waits, an ich8lan-shaped reset, and no link wait, in one image. The sign-on appeared. Which of the three was required is open. No Linux e1000e file was open when that reset was written.

## What to keep

- The `usb:` line on the VGA page, with a host test of the exact bytes. The desk user cannot read COM1 from the monitor.
- The stage letter and the completion code. `4s` and `cc 0` placed the fault in one boot.
- `disable_iommu` instead of programming VT-d page tables Microkit does not install. The option name came from `strings` on `sel4.elf` (`disable_iommu`, `cmdline_str_disable_iommu`), and `support/boot/mb2-stub/src/stub.c` forwards the Multiboot2 command-line tag.
- The QEMU VT-d reproduction before burning the stick, including `Boot config: disable_iommu = true` on the fixed image.
- Saying when a QEMU pass cannot see the metal property under test.

## Still unproven

- The 300 ms settle wait, `find_shell_keyboard`, and the all-ports mouse search.
- The 20 ms and 10 ms recovery pauses, and the 500 ms deadlines.
- Whether the Chain bit, Max ESIT Payload, or both, is what the Z97 controller required.
- The `ep` readout. It belongs on the next control-transfer timeout: 0 disabled, 1 running, 2 halted, 3 stopped, 4 TRB error (`EP_STATE_*` in Linux `xhci.h`). It earns its place only after Address Device has completed.
