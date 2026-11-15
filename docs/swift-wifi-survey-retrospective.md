# Swift Wi-Fi survey retrospective

Last updated: 2026-11-16

The Acer Swift X `setup-wifi` survey printed `rx 0` on every boot until the driver read frames from the MCU rings. The boot that fixed it printed `mt7921: rx 1 ev 16 ok 12 done 1 ch 21 bcn 15` and listed the networks. The boot lines are on [PR 220](https://github.com/julerex/lerux/pull/220). The survey words are defined in [context.md](context.md#swift-radio-survey).

This is the answer to "why did previous agents miss this solution", copied from the session that found it.

---

The survey works now: 16 frames came in on an MCU ring, one on the data ring, and 12 network names were read out of them.

## Why it took so long

The short answer: from the start, every agent assumed that scan frames arrive on the data ring. That assumption shaped every question asked after it, and nothing in the driver could show that it was wrong.

**1. The word "data ring" contained the wrong answer.** `docs/context.md` defined the data ring as the ring where "the card copies a scan frame for the host to read". `AGENTS.md` and the plan docs repeated that, and agents are told to read those files first. Once it was written down as a definition, `rx 0` always read as "the card won't copy frames onto the data ring". Nobody asked whether the frames went somewhere else. The glossary was carrying a guess, not something Linux had shown.

**2. The driver dropped the evidence without counting it.** `wait_survey` and `wait_seq` read the MCU rings, kept only the packet they wanted, and threw everything else away with no count. Your beacons were very likely arriving on every boot and being dropped there. The mark had no field that could show it, so `rx 0` looked identical to "nothing arrived at all". Any receive path that drops packets should count what it drops.

**3. The Linux comparison was thorough at the wrong level.** We compared mt7921 command sequences line by line: init, start, filters, channel domain, power tables, and even v6.8 against 7.3.4. All of that lives in the chip files (`init.c`, `main.c`, `mcu.c`). The rule that decides where a received frame goes lives in the shared mt76 core (`dma.c`), plus `mt7921_queue_rx_skb` in `mac.c`. `dma.c` was not among the Linux sources I had downloaded until this turn. "Match Linux completely" was scoped to init and scan-start, so the receive side was never compared.

**4. The progress was real, but it pointed the wrong way.** Each boot gave a believable next command to try, and `bcn` went from 3 to 16 after station registration. That looked like closing in on a firmware setting, so every theory came from the same family:

- receive filter
- DMA receive length
- the other-beacon drop
- power tables

The one-change-per-boot rule was sound. It just kept testing variations of the same theory. The other-beacon clear was added, removed as "not what Linux does", and added again. That churn was a sign the theory was off.

**5. Linux's naming points away from the answer.** In Linux, scan results reach mac80211 as ordinary received frames. That is easy to read as "they come on the data ring". The detail that matters is a single line: a type-7 packet with flag 1 is renamed `PKT_TYPE_NORMAL_MCU` and treated as a frame. The name sounds like an MCU event, not a beacon.

**6. A misdiagnosis cost a round.** After the `wifi err` boot I first blamed init, and you had to point out that the panel had reached `wait`. That narrowed the next cut correctly, but it shows how easily a boot line was read to fit the current theory.

## Lessons to keep

- **Trace the reference driver's receive path first.** Before copying command sequences, find where the frame you want lands in Linux, end to end through the shared core.
- **Every discard path gets a counter.** A mark that only counts the expected path cannot prove a negative.
- **Keep guesses out of definitions.** A glossary should say what a ring is, not what it is believed to carry.
- **Not every earlier change is proven necessary.** Earlier images couldn't show `ev`, so we don't know which of these mattered: the other-beacon clear, the DMA length, or the station registration. Some may be unnecessary.
