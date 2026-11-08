# Swift Wi-Fi plan

Last updated: 2026-11-08

**Status:** Phase 1 is done and is the full-panel dump plus the root-port readback. Its first photograph eliminated the root port, and Phase 3's ASPM clear is on the next ISO. Phases run in order. Phases 2 through 5 are the Swift radio. Phases 6 through 8 are the application path and do not need the laptop. Phase 9 is an alternative route, not a step.

**Goal:** A lerux application on the Acer Swift X reaches a host on the internet over the MT7921 radio. [plan-lionsos.md](plan-lionsos.md) Milestone 27 stays the radio exit (`ping ok`). This plan is the route from the current stall to that exit, and from that exit to an application.

**Why a separate plan:** [swift-wifi-assessment.md](swift-wifi-assessment.md) found the driver sequence faithful to Linux v6.8 and the debugging loop the problem: one or two bits of information per ISO cycle, on a machine with no serial port and no QEMU model. Each phase below is chosen to raise the information per boot or to move work off the laptop.

## Global constraints

- Leave the firmware download order in `userspace/pds/swift-mt7921/src/device.rs` as it is until a boot shows a word that names a later step. A readback is a change of information, not a change of sequence.
- The panel is 120 columns by 67 rows. The console owns rows 0 to 24. The probe owns rows 25 to 30. Rows 31 to 65 are the dump, laid out by `lerux_swift::dump`, and row 66 is the `END OF OUTPUT` marker, so a photograph that shows it shows the whole panel. A row is at most 120 bytes. A new value is a byte formatter in `lerux-swift` with a host test of the exact bytes.
- A photograph of the panel is the record of a boot. Every number on it is in hexadecimal with no `0x`. Keep the row and column of each field stable between ISOs, so two photographs can be compared.
- `just check`, `just check-pd`, and `just iso` are sequential. `just iso` is the image that goes on the stick, and it rebuilds the Swift protection domains before packing.

## Phase 1 — Full-panel dump and root-port readback

**Exit:** One photograph of the Swift panel holds the MT7921 configuration space, the Wi-Fi root-port configuration space, the NVMe root-port configuration space, the IOMMU registers, the WFDMA register block before and after the boot attempt, the six ring blocks, the prefetch table, the posted descriptor, the semaphore command, and the first receive descriptor.

Done. `swift-pci-probe` finds the bus 0 bridge whose secondary bus is 3, sets memory space and bus master on it the way Linux `pci_enable_bridge` does, and paints the word before the write beside the word after it on row 31. The three 256-byte configuration spaces are rows 32 to 49. The IOMMU registers are row 50. `swift-mt7921` reads `DUMP_REGS` just before the patch semaphore and again, with the rings, the prefetch table, and the DMA words, when `start` returns, then paints rows 51 to 65. Row 66 is the end marker. The row map is in [swift-boot-screen.md](swift-boot-screen.md#the-dump-rows-31-to-66).

The first photograph (8 November 2026) read `rp 00:02.2 sec 03 cmd 0007>0007`: the root port already forwarded, so suspect 1 is eliminated. The same boot showed `nvme: missing` and `bt: port 3 desc`, so the disk and the USB controller both fetch from the reserved window and the Wi-Fi card is the odd one out of three. The card read `lnk 0142 pm 0008`: D0, with ASPM L1 and clock power management on. Both root ports read `lnk 0042`. The upload was 1024 pixels across, so rows 32 to 66 were not legible and the Phase 2 comparison waits for a closer photograph.

## Phase 2 — Differential against the NVMe path

**Exit:** A written list of every configuration-space field that differs between `03:00.0` plus its root port and `02:00.0` plus its root port, from one photograph.

Rows 32 to 49 are the three spaces. Compare the command and status words at `+04`, the capability chain from `+34`, the PCI Express capability's device control and link control, and the power-management control and status. Row 31 already extracts command, link control, and PM control for each. The NVMe reaches `0x1f00000`, so a field that is set on its path and clear on the Wi-Fi path is a candidate. Record the list in this document under this phase.

The `bt:` row is the third witness. On the first photograph it read `port 3 desc`, past `slot`, so the xHCI on bus 4 also fetches from the reserved window and the Wi-Fi path is the odd one out.

Rows 32 to 49 hold the card, its root port, and the disk's root port, not the disk itself. Both root ports already read `cmd 0007 lnk 0042` on row 31, so the endpoint-against-endpoint half of this comparison has no rows yet. If the first legible photograph shows the two ports identical, rows 44 to 49 can carry `02:00.0` instead of its port.

## Phase 3 — Link power and warm-boot residue

**Exit:** The stall is reproduced or not reproduced from a genuine cold boot, and the ASPM state of the endpoint and the root port is on the photograph.

Linux calls `mt76_pci_disable_aspm` on the endpoint and the parent before it touches the chip. Through `pci_disable_link_state(L0S | L1 | CLKPM)` that clears link control bits 1:0 on both functions and bit 8 on the endpoint. The first photograph showed those bits set (`lnk 0142` on the card, `0042` on the port), so the probe now clears them in that order, endpoint then port, before `swift-mt7921` starts, and row 31 paints each link word before and after (`lnk 0142>0040`, `lnk 0042>0040`). Boot from power-off at least once, since the Linux form of this stall (`Failed to get patch semaphore`) is often reported after a warm restart.

The Linux call also disables the L1 PM substates (L1.1 and L1.2). That control register is in the extended capability past byte 256, and the probe reads configuration space through port `0xcf8`, which stops at byte 255. If the stall stays at `fw sem` with `lnk …>0040` on both functions, the next readback is that register through ECAM (the MCFG base, a new region in the template), painted before and after the same way. Clock power management being on is the hint that L1.2 is armed.

## Phase 4 — Linux ground truth on the same laptop

**Exit:** `support/swift-linux/` holds `lspci -vvv` for `03:00.0` and its root port, the `dmesg` lines matching `mt7921|AMD-Vi`, and a `trace-cmd` report of `mt76:reg_rr` and `mt76:reg_wr` from one `modprobe mt7921e`.

A live USB gives more in one boot than ten ISO cycles. The trace is register offsets and values of a bring-up that works on this exact unit. Diff that trace against `DUMP_REGS` and the sequence in `device.rs`. The first register whose value differs at the same step is the lead. Add that directory to `.gitignore` when the files land, since they name the unit; the diff is written into this document.

## Phase 5 — Driver-side deltas

**Exit:** `device.rs` matches the Linux order byte for byte, or each remaining delta has a photograph that justifies it.

Only after Phases 1 to 4. Three steps are not in Linux and were added on hypothesis: `RST_DTX_PTR = 0` after `~0`, `rebind` after `TX_DMA_EN`, and the second `DMASHDL_BYPASS`. The L1 remap writes the whole `0xfe24c` word where Linux masks. Revert each one in its own commit and photograph the result. Then `ping ok` on the metal panel is the Milestone 27 exit.

## Phase 6 — Application path on QEMU

**Exit:** The browser image on `qemu_virt_aarch64_browser_sddf` resolves a name through DNS at `10.0.2.3`, verifies a certificate against a bundled root set, and follows one redirect.

None of this needs the laptop. The shell hardcodes `10.0.2.2` and the TLS name `host`. Add a DNS client over the existing UDP path, a root store (`webpki-roots` or the equivalent for the pinned TLS crate), and `Location` handling to the fetch. The smoke stays `pixel 000000ff` and `maths 9`; a second script step fetches by name.

## Phase 7 — Compose the fetch roles into the Swift image

**Exit:** `console-swift.system.template` holds `net_virt_tx`, `net_virt_rx`, `net_copy`, and a shell that links smoltcp, with `console-driver` as the output and `swift_mt7921` as the driver slot.

Template and channel work. The channel ids come from the profile manifest. The shell is the same crate as the fetch image, so the only new code is the console output in place of the serial client. The queue side can be proven on QEMU by composing the same roles over the e1000e driver on the q35 image first.

## Phase 8 — The radio as an sDDF net driver

**Exit:** `swift-mt7921` enqueues received Ethernet frames on the standard receive queue and dequeues transmit frames from the standard transmit queue; `ping ok` comes from the shell, not from the driver.

Reshape the monolithic test. 802.11 to 802.3 translation, per-frame CCMP, and the association keepalive stay in the driver. The DHCP and ICMP code in `join.rs` moves out, since smoltcp does both. Only the last boot of this phase needs the laptop.

## Phase 9 — Alternative route: USB Ethernet

Not a step in order. A USB Ethernet adapter over the xHCI driver that Bluetooth needs anyway skips firmware download, WPA2, and CCMP. QEMU emulates `qemu-xhci` and `usb-net`, so the whole path is QEMU-testable. If "Swift on the internet" matters more than Wi-Fi specifically, run this instead of Phases 2 to 5, and keep Phases 6 and 7 as they are.
