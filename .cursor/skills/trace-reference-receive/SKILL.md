---
name: trace-reference-receive
description: >
  Use when a lerux driver copies a Linux driver (mt7921, Bluetooth, NVMe,
  e1000e, GENET, or another), when the device reports activity but the host
  counts nothing (a mark such as `rx 0` beside a non-zero `bcn`), when adding
  a receive or poll loop that keeps one packet kind and skips the rest, or
  when writing a definition in docs/context.md. Trace where Linux receives
  the frame before copying more commands, count every discard, and keep
  guesses out of definitions.
---

# Trace the reference receive path

The Swift survey printed `rx 0` for many boots while the firmware reported beacons. Every agent compared mt7921 command sequences with Linux and added commands. The frames were already reaching the host on an MCU ring, and `wait_survey` dropped them without a count. The full account is [docs/swift-wifi-survey-retrospective.md](../../../docs/swift-wifi-survey-retrospective.md).

## Before copying another command

When the host does not see a frame the device says it has, find where Linux receives that frame first. Follow it from the interrupt to the stack that consumes it, through the shared core and not only the chip files.

For mt76 that path is `mt76_dma_rx_process` in `dma.c`, which calls `drv->rx_skb` for every queue, the MCU queues included, and then `mt7921_queue_rx_skb` in `mac.c`, which dispatches on the packet type in RXD0. A type-7 packet with flag 1 is renamed `PKT_TYPE_NORMAL_MCU` and handled as a frame. The name sounds like an MCU event. Read the dispatch, not the name.

Download the core file when it is missing from the local Linux sources, at the Linux tag the protection-domain header names. The running kernel and a newer mainline tag are different trees. A comparison of init and start commands that leaves the receive side out is not "matching Linux".

## Count every discard

Every loop that reads a ring and keeps only the packet it wants counts what it skips, by ring. Put that count in the mark the panel or the serial line prints. A mark that counts only the expected path cannot tell "nothing arrived" from "it arrived and was dropped".

The Swift mark has `rx` (data ring) and `ev` (frames found on an MCU ring) for this reason. A new receive path adds the same kind of field before the next boot.

## Keep guesses out of definitions

`docs/context.md` says what a ring, a register, or a stage is. It does not say what a ring is believed to carry. Agents read that file first and treat it as fact. The old data-ring definition said the card copies scan frames there, and every later boot was read through that sentence.

When a belief is still a hypothesis, write it in the plan doc or on the pull request as a hypothesis.

## Read the boot history as evidence

- When several boots in a row test variations of one theory, or a change is added, removed, and added again, stop and check the theory against the reference driver.
- Read a boot line against what it can show. A panel that reached `wait` finished init, so a later `wifi err` is not an init failure.
- A counter that rose after a change does not prove the change was needed. The Swift survey kept the other-beacon clear, the DMA receive length, and the station registration, and earlier images had no `ev` field to show whether any of them mattered. Name such changes as unproven when you report them.
- A stop letter cannot falsify edits that share its image. Paint what was posted before changing key data, frame type, or the transmit queue.
