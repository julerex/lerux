//! Rows 52 to 66 of the panel: the WFDMA registers after the boot attempt,
//! the ring registers, the prefetch table, the posted TX17 descriptor, the
//! semaphore command it points at, the first RX event descriptor, and the
//! same registers read before the first MCU command.
//!
//! The row layout is `lerux_swift::dump`. The register names are
//! `lerux_swift::mt7921::DUMP_REGS`, so one photograph of the panel reads
//! back against that table.

use lerux_swift::{
    dump::{
        address_label, write_hex_row, write_named_row, NAMES_PER_ROW, PANEL_COLS, ROW_BOUNCE,
        ROW_PREFETCH, ROW_REGS_AFTER, ROW_REGS_BEFORE, ROW_RINGS, ROW_RX_DESC, ROW_TX_DESC,
        WORDS_PER_ROW,
    },
    mt7921::{write_ring, DUMP_PREFETCH, DUMP_REGS, DUMP_RINGS},
};

use crate::{
    device::{Snapshot, BOUNCE_WORDS, DMA_PHYS, OFF_BOUNCE, OFF_RX_EVT, OFF_TX17},
    paint_full,
};

const REG_ROWS: usize = DUMP_REGS.len().div_ceil(NAMES_PER_ROW);
const RINGS_PER_ROW: usize = 3;
const _: () = assert!(REG_ROWS == 3);
const _: () = assert!(DUMP_RINGS.len() == 2 * RINGS_PER_ROW);
const _: () = assert!(DUMP_PREFETCH.len() == 3);
const _: () = assert!(BOUNCE_WORDS <= 2 * WORDS_PER_ROW);

/// Paint every dump row. `before` is the register read before the first MCU
/// command; `after` is the snapshot taken when `start` returned.
pub fn paint_mac_dump(before: &[u32; DUMP_REGS.len()], after: &Snapshot) {
    paint_regs(ROW_REGS_AFTER, b"aft", &after.regs);
    paint_rings(after);
    paint_prefetch(after);
    paint_words(ROW_TX_DESC, DMA_PHYS + offset(OFF_TX17), &after.tx_desc);
    paint_words(
        ROW_BOUNCE,
        DMA_PHYS + offset(OFF_BOUNCE),
        &after.bounce[..WORDS_PER_ROW],
    );
    paint_words(
        ROW_BOUNCE + 1,
        DMA_PHYS + offset(OFF_BOUNCE + 4 * WORDS_PER_ROW),
        &after.bounce[WORDS_PER_ROW..],
    );
    paint_words(ROW_RX_DESC, DMA_PHYS + offset(OFF_RX_EVT), &after.rx_desc);
    paint_regs(ROW_REGS_BEFORE, b"pre", before);
}

fn paint_regs(row: usize, tag: &[u8], values: &[u32; DUMP_REGS.len()]) {
    let mut names: [&[u8]; NAMES_PER_ROW] = [b""; NAMES_PER_ROW];
    for index in 0..REG_ROWS {
        let start = index * NAMES_PER_ROW;
        let end = (start + NAMES_PER_ROW).min(DUMP_REGS.len());
        for (slot, (name, _)) in names.iter_mut().zip(&DUMP_REGS[start..end]) {
            *slot = name;
        }
        let mut line = [b' '; PANEL_COLS];
        let n = write_named_row(&mut line, tag, &names[..end - start], &values[start..end]);
        paint_full(row + index, &line[..n]);
    }
}

fn paint_rings(after: &Snapshot) {
    for (index, chunk) in after.rings.chunks(RINGS_PER_ROW).enumerate() {
        let mut line = [b' '; PANEL_COLS];
        let mut n = 0;
        for (i, regs) in chunk.iter().enumerate() {
            let (name, _) = DUMP_RINGS[index * RINGS_PER_ROW + i];
            if n > 0 {
                line[n] = b' ';
                n += 1;
            }
            n += write_ring(&mut line[n..], name, *regs);
        }
        paint_full(ROW_RINGS + index, &line[..n]);
    }
}

fn paint_prefetch(after: &Snapshot) {
    for (index, (words, base)) in after.prefetch.iter().zip(DUMP_PREFETCH).enumerate() {
        paint_words(ROW_PREFETCH + index, base, words);
    }
}

fn paint_words(row: usize, address: u32, words: &[u32]) {
    let mut line = [b' '; PANEL_COLS];
    let n = write_hex_row(&mut line, &address_label(address), words);
    paint_full(row, &line[..n]);
}

fn offset(off: usize) -> u32 {
    u32::try_from(off).expect("dma window offset")
}
