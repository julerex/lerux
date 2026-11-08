# Swift Wi-Fi assessment

An assessment of the Acer Swift X (`swift_sfx14_41g`) bring-up, written after reading the Swift protection domains, the recent commits, [plan-lionsos.md](plan-lionsos.md) Milestones 24 to 29, and comparing the MT7921 sequence against the Linux v6.8 `mt76` sources side by side. The question was whether the work is on the right track toward lerux applications reaching the internet from that laptop, given that progress through the ISO loop has been slower than expected.

The plan that follows from this assessment is [plan-swift-wifi.md](plan-swift-wifi.md). The screen guide is [swift-boot-screen.md](swift-boot-screen.md). The 8 November 2026 Linux boot is [what the Linux boot showed](plan-swift-wifi.md#what-the-linux-boot-showed). Linux loads the firmware on this laptop. The paragraphs below are the reasoning as it stood before that boot, including the decision to set the `fe24c` full-word write aside.

## Short answer

Yes on the driver, mostly yes on the method, and no on the loop it is being run in.

The MT7921 transcription in `userspace/pds/swift-mt7921/src/device.rs` and `userspace/crates/lerux-swift/src/mt7921.rs` is faithful to Linux, and "read back one permission per boot" is the right kind of reasoning. The slowness is structural: the work is debugging the first bus-master read of a device with no QEMU model, on a machine with no serial port, at roughly one or two bits of information per ISO cycle. That loop will not get faster by itself.

There is also a scope gap between the current exit (`ping ok` inside one protection domain) and "lerux apps connect to the internet" that is worth planning now, because most of it does not need the laptop at all.

## What was verified against Linux v6.8

The register map and sequence were checked rather than the comments:

- `fixed_map`, the L1 remap window (`0xfe24c`, base `0x40000`), `WFDMA0` at `0xd4000`, the ring register layout (`base`, `size`, `cpu_idx`, `dma_idx` at +0, +4, +8, +12), the 14 prefetch words, the `GLO_CFG` bits, `RST_DTX_PTR`, `DUMMY_CR` and `NEED_REINIT`, `MCU2HOST`, the `CONN_ON_LPCTL` own bits, `TOP_LPCR_HOST_BAND0` driver-own (this is `mt7921e_driver_own` in `pci_mcu.c`, so it is correct for the 7921), `WFSYS_SW_RST_B`, chip id and revision, and the TXD, MCU header, and descriptor control word all match.
- The order fw-own, drv-own, chip id, wfsys reset, `dma_disable(force)`, program rings, `dma_enable`, `SWDEF_MODE = 0`, driver own, `L0S_DIS`, patch semaphore matches `mt7921_pci_probe`, `mt7921_dma_init`, `mt792x_dma_enable`, and `mt7921e_mcu_init`.

So the stall at `fw sem d00 c01 s0100 b2000 g75` with `slot 00 01f66000 d0` is not a transcription bug in the ring programming. The engine is enabled, the ring is right, the note is right, and the card never issues, or never completes, the upstream read. That narrows the remaining suspects to platform permission and power state, which is where the recent commits have been looking.

## Suspects to check next, in order

1. **Bus master on the root port, not just the endpoint.** `swift-pci-probe` set `0x06` on `03:00.0` only (`enable_mem` in `userspace/pds/swift-pci-probe/src/main.rs`). A PCIe root port forwards upstream memory requests only if its own Command register has Bus Master Enable set. Linux gets this for free from `pci_enable_device`, which calls `pci_enable_bridge` up the chain. UEFI used the NVMe root port to boot, so that bridge is certainly enabled; it never used the Wi-Fi root port. One extra config read answers this: find the bus 0 bridge whose secondary bus at offset `0x19` is 3 and read its offset `0x04`. This is the same shape as the `cmd` line. The probe now does this and paints the word before and after the write on row 31.
2. **Differential against the device that works.** NVMe DMA to `0x1f00000` demonstrably works. Dump Command, Status, Link Control (the ASPM bits), Device Control, and PM Control/Status (the D-state) for both the NVMe and Wi-Fi endpoints and both their root ports side by side. Whatever differs is the lead. Also note whether the `bt:` row gets past `slot`; if xHCI on bus 4 fetches its command ring from `0x1f20000`, two of three devices reach that RAM window and the Wi-Fi path is the odd one out. Rows 32 to 49 of the panel now hold the three configuration spaces for this comparison.
3. **ASPM and link power state.** Linux calls `mt76_pci_disable_aspm` on both the endpoint and the parent root port before touching the chip. lerux only sets `L0S_DIS` inside the chip's PCIe MAC. The Linux form of this stall is `Message 00000010 (seq 1) timeout` followed by `Failed to get patch semaphore`, and the commonly reported field causes for that on AMD laptops are link and power state and warm-boot residue, not ring programming. Test from a genuine cold boot at least once.
4. **Only after those: driver-side deltas.** Three steps in `device.rs` are not in Linux and were added on hypothesis rather than measurement: writing `RST_DTX_PTR = 0` after `~0`, `rebind` (rewriting ring base, size, and cpu index after `TX_DMA_EN` is set), and the second `DMASHDL_BYPASS` set. The L1 remap also overwrites the whole `0xfe24c` word where Linux does a masked read-modify-write. None of these is a likely cause, since the stall existed before them, but each speculative delta widens the space that later has to be reasoned about. The AGENTS.md rule, "Leave the download sequence unchanged until a boot shows that word", is the right discipline. Revert to byte-for-byte Linux order and change only the environment until a readback says otherwise.

## The highest-leverage move: one Linux session on the same laptop

A live USB on the Swift gives, in a single boot, more than the last ten ISO cycles:

- `lspci -vvv -s 03:00.0`, and the same for its root port: Bus Master Enable, ASPM, and the D-state as firmware left them.
- `dmesg | grep -E 'mt7921|AMD-Vi'`: confirms this exact unit downloads firmware under Linux at all (`ASIC revision: 7961…`, `WM Firmware Version`), and whether the IOMMU is enabled.
- `trace-cmd record -e mt76:reg_rr -e mt76:reg_wr modprobe mt7921e`. `mt76` has `reg_rr` and `reg_wr` tracepoints in `mmio.c`, so this yields the exact register-level sequence and values of a successful bring-up on this hardware, in BAR offsets. Diffing that trace against the lerux sequence is the fastest way to find the difference.

Separately, raise the information per boot: the panel can hold far more than one line. Paint the whole WFDMA block, the descriptor, the endpoint and root-port configuration headers, and the IOMMU registers as a hex block and photograph it, instead of adding one readback per pull request. Rows 31 to 66 of the panel now do this; the layout is in [swift-boot-screen.md](swift-boot-screen.md#the-dump-rows-31-to-66).

## The bigger picture: `ping ok` is not "apps on the internet"

The current `swift-mt7921` is a monolithic test: driver, scan, auth, assoc (`join.rs`), WPA2 supplicant and software CCMP (`lerux-swift/wpa.rs`, `crypto.rs`), DHCP, one ICMP echo, and exit at `PingOk`. The Swift template `userspace/systems/templates/console-swift.system.template` has no net virtualisers, no copier, and no smoltcp client. The app-facing path that is needed already exists, but only on the QEMU images: sDDF `net_driver`, `net_virt_tx` and `net_virt_rx`, `net_copy`, the shell with smoltcp, `lerux-posix` sockets, and `lerux-tls`. The e1000e driver proves that path on x86 (Milestones 20 and 21).

What remains after `ping ok`, and where it can be built:

- Reshape `swift-mt7921` into an sDDF net driver: Ethernet frames on the standard queues, 802.11 to 802.3 header translation, per-frame CCMP, and an association keepalive. Only the final step needs the laptop; the queue side is already tested on QEMU.
- Compose the fetch and browser roles into the Swift image with `console-driver` as the output. This is template work the repository has done many times.
- Real internet rather than a fixture host: DNS resolution, real TLS roots (`webpki-roots` or similar), and redirects. The browser today hardcodes `10.0.2.2` and the TLS name `host`. All of this is testable on QEMU user-net today (slirp provides DNS at `10.0.2.3`) and does not depend on the Wi-Fi stall.

The second and third items can start now, in parallel, so "lerux apps on the internet" becomes true on QEMU and the Swift work reduces to the radio.

One option worth weighing: a USB Ethernet adapter over the xHCI driver that Bluetooth needs anyway. QEMU can emulate `usb-net` and `qemu-xhci`, so the whole path is QEMU-testable, and it skips firmware download, WPA2, and CCMP. It is a materially lower-risk route to a first "Swift on the internet" than MT7921 Wi-Fi, if that milestone matters more than Wi-Fi specifically.
