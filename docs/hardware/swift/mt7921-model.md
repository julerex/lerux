# MT7921 host model

`lerux_swift::mt7921_card::Card` is the Swift card (`PCI 14c3:7961`) for host tests. `swift-mt7921`'s `Mac` is unchanged as a driver. On a test build its register read and write call the card, which updates the same BAR and DMA window the driver polls.

## What it answers

- Ownership (`CONN_ON_LPCTL`), WFSYS init-done, driver-own, chip id `0x7961`, and a cleared WTBL busy bit.
- WFDMA transmit rings: the DMA index moves and `DMA_DONE` is set when the CPU index is written.
- Receive rings: the completion is written at the descriptor the driver will read next, then marked done. A consumed slot stays done until the driver refills a different slot, so the model does not reuse the first descriptor whose done bit is clear. Events use the event ring before N9 ready and the WA ring after it.
- Firmware download for the synthetic patch and RAM images (`patch_image`, `ram_image`): semaphore, scatter, patch finish, and start. `CONN_ON_MISC` then shows N9 ready.
- MCU replies the init path waits on: NIC capability (2T2R, station address), efuse, and unified command status 0.
- A survey: one beacon on the data ring, two on the WA ring (type 7, flag 1), then scan-done with 21 channels and 15 beacons.
- Open-system authentication, association id 1, and EAPOL messages 1 and 3, so `Mac::associate` can finish.

The model completes a doorbell before the write returns. `Mac` polls, so the MSI is not raised.

## Sources

- `docs/hardware/swift/MT7921LEN-Product_Brief_202308.pdf` — MT7921LEN, 802.11a/b/g/n/ac/ax, 2T2R, Bluetooth 5.2 (August 2023).
- Linux v6.8 `mt76`: `mt7921/pci.c`, `mt7921/mac.c` (`mt7921_queue_rx_skb`), `mt7921/mcu.c`, `mt792x_dma.c`, `dma.c` (`mt76_dma_rx_process`), `mt76_connac_mcu.c`.
- `userspace/crates/desk/lerux-swift/src/radio/mt7921.rs` and `userspace/pds/board/swift/swift-mt7921/src/device.rs` — the register map and the sequence the model answers.
- `docs/guide/overview/context.md` (Swift radio survey) and `docs/hardware/swift/swift-wifi-survey-retrospective.md` — which ring a scan frame is read from. The data ring is not the only ring that carries one.
