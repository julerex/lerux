# Swift Wi-Fi plan

Last updated: 2026-11-08

**Status:** Phase 1 is done. The 8 November 2026 photograph of the ASPM clear left the stall at `fw sem`, so link power is eliminated. The Phase 4 capture shows Linux loads firmware on this unit. The first write that differed was `HIF_REMAP_L1`, and `rr` / `wr` now keep its high half. The next photograph is a cold boot of that image. Phases 2 through 5 are the Swift radio. Phases 6 through 8 are the application path and do not need the laptop. Phase 9 is an alternative route, not a step.

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

The Linux call also disables the L1 PM substates (L1.1 and L1.2). That control register is in the extended capability past byte 256, and the probe reads configuration space through port `0xcf8`, which stops at byte 255.

The photograph of 8 November 2026 showed `lnk 0142>0040` on the card and `lnk 0042>0040` on `00:02.2`, with the stall still `fw sem d00 c01 s0100 b2000 g75`. Bits 1:0 are clear on both ends, so the link cannot enter L1 and the L1 substate register is not the next measurement.

## Phase 4 — Linux ground truth on the same laptop

**Exit:** `support/swift-linux/out/` holds `lspci -vvv` for the MT7921 and its root port, the `dmesg` lines matching `mt7921|AMD-Vi`, and a report of `mt76:reg_rr` and `mt76:reg_wr` from one `modprobe mt7921e`.

The capture is [support/swift-linux/README.md](../support/swift-linux/README.md). `just iso-arch-and-capture` lists the USB sticks and writes the current Arch install image and `capture.sh` onto the one you choose. Boot that stick from power-off with `modprobe.blacklist=mt7921e` on the kernel line. The install image has no network until this driver is up, so the script reads those two events from the kernel tracer; they are the events `trace-cmd` would record. The addresses in the report are BAR offsets, because `mt76` traces the access after remap. `map_reg` in `userspace/crates/lerux-swift/src/mt7921.rs` is the translation back to the logical addresses in `device.rs`. The same run saves the NVMe endpoint and its root port, which is the Phase 2 comparison the panel photograph did not resolve. `out/` is gitignored because the files name the unit.

The 8 November 2026 capture is that `out/` directory. `verdict.txt` says `probe unfinished` because the script waits for the words `Firmware init done`, and that line is absent from `dmesg`. `lspci-wifi-after.txt` says the driver in use is `mt7921e`. The log prints the patch header and `WM Firmware Version: ____010000` within half a second, with no `Failed to get patch semaphore`. `CONN_ON_MISC` (`e00f0`) reads `00000000` at 83.930 s, `00000001` at 83.950 s, and `00000003` at 84.240 s, which is `FW_N9_RDY`.

These writes match `device.rs` through driver-own. `CONN_ON_LPCTL` is written `00000001` then `00000002`. `WFSYS_RST` (`40140`) goes `00000011`, `00000010`, `00000001`, `00000011`. `GLO_CFG` (`d4208`) goes `00103870`, `5030b870`, `5030b875`. `TOP_LPCR` (`40010`) is written `00000002` and reads back `00000000`. `PCIE_MAC_PM` (`10194`) goes `00000e0f` to `00000f0f`.

The first write whose value differed is `HIF_REMAP_L1` (`fe24c`), on the chip-id read. Linux reads `18451844` and writes `18457001`: bits 15:0 become the page `7001`, and bits 31:16 stay `1845`. `rr` and `wr` stored that page alone, so the same step wrote `00007001`. The same pattern is the wfsys window: Linux writes `18451800`, and the old code wrote `00001800`. The panel still read chip `00007961` and `wfsys 00000011`, so those two windows answered with the short write.

The next value both sides leave behind is `RST_DTX_PTR` (`d420c`). Linux writes `ffffffff` once, and the later reset paths in the same trace write `ffffffff` again. `dma_enable` writes `ffffffff` and then `0`, and that `0` is what is present when lerux posts the semaphore. Linux's one `DMASHDL_SW` write while `e00f0` is still `00000000` stores `10000000`, during `dma_disable`. The second bypass and the ring rebind do not appear in the trace before `e00f0` reads `00000003`. Ring base, size, and index are programmed with `writel` on the queue registers, so they are absent from this trace on both the Linux side and any comparison that only has `regs.txt`.

`rr` and `wr` now keep bits 31:16 of `fe24c` and store the page in bits 15:0, then read the register back before the window access. `remap_l1` is that merge, and its host test is the three words from the trace. The `dtx` zero stays until a boot of this image shows a later word. What this boot changed about the earlier investigation is [below](#what-the-linux-boot-showed).

## What the Linux boot showed

The photographs through 8 November had closed every suspect the panel can show.

The descriptor is a live command for the bounce buffer (`slot 00 01f66000 d0`). Transmit is enabled, and the DMA index stays 0 (`fw sem d00 c01 s0100 b2000 g75`). The root port already had memory space and bus master (`cmd 0007>0007`). The card's own command word is `cmd 0006`. Clearing link power on both ends (`lnk 0142>0040` on the card, `lnk 0042>0040` on `00:02.2`) left the stall on `fw sem`. The IOMMU exclusion registers are zero. The NVMe admin commands completed, and the USB controller reached `bt: port 3 desc`. Both of those devices fetch from the same reserved RAM the Wi-Fi card is asked to fetch from. The Wi-Fi card is the one whose counter does not move.

[swift-wifi-assessment.md](swift-wifi-assessment.md) treated the driver text as a faithful copy of Linux v6.8 and put the remaining cause on platform permission and power. It recorded that `fe24c` was written as a whole word where Linux masks, and set that aside: the stall was already there, and the panel's chip id and wfsys readbacks were the right values (`00007961`, `wfsys 00000011`). Ten ISO cycles of painting another permission could not show a high half that those readbacks do not contain.

One Arch boot on the same laptop is the measurement those photographs could not make. The stick is `just iso-arch-and-capture`. The kernel line carries `modprobe.blacklist=mt7921e`, and the boot is from power-off. The files are in `support/swift-linux/out/`, gitignored because they name the unit.

Linux loads the firmware on this unit. `dmesg` prints the patch header (`HW/SW Version: 0x8a108a10, Build Time: 20260224110909a`) and `WM Firmware Version: ____010000, Build Time: 20260224110949` within half a second of `modprobe`. There is no `Failed to get patch semaphore` and no `Timeout for initializing firmware`. `lspci-wifi-after.txt` leaves the driver in use as `mt7921e`. `CONN_ON_MISC` reads `00000000`, then `00000001`, then `00000003`, and `00000003` is `FW_N9_RDY`. From about a second after the load until the trace stops, `iwd` is using the card. `verdict.txt` says `probe unfinished` because the script waits for the words `Firmware init done`, and that line is not in the kernel log. The string is in `mt792x-lib`, and this boot never printed it. The driver staying bound, the firmware version lines, and `FW_N9_RDY` are the result.

The register trace of that success matches the writes the panel had been used to judge. Own, wfsys, `GLO_CFG`, driver-own, and `L0S_DIS` take the values listed in Phase 4. The fork is earlier than the semaphore, at the chip-id read. Linux reads `HIF_REMAP_L1` as `18451844` and writes `18457001`. The page `7001` replaces bits 15:0, and bits 31:16 stay `1845`. The code stored the page alone (`00007001`), and the wfsys window the same way (`18451800` against `00001800`). A correct chip id on the panel was evidence that the window answered, and it hid the half of the register the window read does not return.

`RST_DTX_PTR` is the next value that remains different at the semaphore. Linux writes `ffffffff` and leaves it, including on the later reset paths in the same trace. `dma_enable` writes `ffffffff` and then `0`. The second `DMASHDL_BYPASS` and the ring rebind do not appear before `FW_N9_RDY`. Ring base, size, and cpu index are programmed with `writel` on the queue registers, below the `mt76` tracepoint, so `regs.txt` has nothing to say about them.

`rr` and `wr` now perform that masked update and read `fe24c` back before the window access. The download order, the `dtx` zero, the second bypass, and the rebind are unchanged. The next photograph is a cold boot. The row 22 word is the result. A word past `fw sem` names the following step. Another `fw sem` makes the `dtx` zero the next change.

## Phase 5 — Driver-side deltas

**Exit:** `device.rs` matches the Linux order byte for byte, or each remaining delta has a photograph that justifies it.

Only after Phases 1 to 4. Three steps are not in Linux and were added on hypothesis: `RST_DTX_PTR = 0` after `~0`, `rebind` after `TX_DMA_EN`, and the second `DMASHDL_BYPASS`. The L1 remap now keeps bits 31:16, which is the Phase 4 change, and its photograph comes first. Revert each of the three in its own commit after that photograph and record the result. Then `ping ok` on the metal panel is the Milestone 27 exit.

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
