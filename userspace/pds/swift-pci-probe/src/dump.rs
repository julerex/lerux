//! Rows 31 to 51 of the panel: the MT7921 function, the root port above it,
//! the root port above the `NVMe`, and the AMD IOMMU registers.
//!
//! The root port is found by walking bus 0 for a type 1 header whose
//! secondary bus is the device's bus. Linux `pci_enable_bridge` sets memory
//! space and bus master on that port before the endpoint is used; this
//! probe does the same on the Wi-Fi port and paints the word it read before
//! the write beside the word it reads after.

use lerux_swift::{
    config_address,
    dump::{
        bdf_label, config_words, is_bridge, offset_label, secondary_bus, write_function_summary,
        write_hex_row, write_port_summary, write_wide, PortReadback, CONFIG_BYTES, CONFIG_ROWS,
        PANEL_COLS, ROW_BYTES, ROW_IOMMU, ROW_NVME_PORT, ROW_SUMMARY, ROW_WIFI_CONFIG,
        ROW_WIFI_PORT, WORDS_PER_ROW,
    },
    MT7921_BDF, NVME_BDF,
};

use crate::{in32, out32, paint};

/// Offsets of the IOMMU registers on rows 50 and 51, in paint order:
/// device table, command buffer, event log, control, exclusion base,
/// exclusion limit, status.
pub const IOMMU_DUMP_OFFSETS: [usize; 7] = [0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x2020];

pub struct Pci {
    pub cap: u64,
    pub port: u16,
}

struct Port {
    bdf: (u8, u8, u8),
    config: [u8; CONFIG_BYTES],
    before: Option<u16>,
}

impl Pci {
    fn read_config(&self, (bus, dev, func): (u8, u8, u8), out: &mut [u8; CONFIG_BYTES]) {
        let data = self.port.wrapping_add(4);
        for reg in (0usize..CONFIG_BYTES).step_by(4) {
            let at = u8::try_from(reg).expect("config offset");
            out32(self.cap, self.port, config_address(bus, dev, func, at));
            let word = in32(self.cap, data);
            out[reg..reg + 4].copy_from_slice(&word.to_le_bytes());
        }
    }

    fn read_head(&self, bdf: (u8, u8, u8)) -> [u8; 32] {
        let (bus, dev, func) = bdf;
        let data = self.port.wrapping_add(4);
        let mut head = [0u8; 32];
        for reg in (0u8..32).step_by(4) {
            out32(self.cap, self.port, config_address(bus, dev, func, reg));
            let word = in32(self.cap, data);
            let at = usize::from(reg);
            head[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
        head
    }

    /// The bus 0 bridge whose secondary bus is `secondary`.
    fn find_root_port(&self, secondary: u8) -> Option<(u8, u8, u8)> {
        for dev in 0..32u8 {
            for func in 0..8u8 {
                let head = self.read_head((0, dev, func));
                if is_bridge(&head) && secondary_bus(&head) == Some(secondary) {
                    return Some((0, dev, func));
                }
            }
        }
        None
    }

    /// Memory space and bus master on a bridge. Returns the word before the write.
    fn enable_bridge(&self, (bus, dev, func): (u8, u8, u8)) -> u16 {
        let data = self.port.wrapping_add(4);
        out32(self.cap, self.port, config_address(bus, dev, func, 0x04));
        let command = in32(self.cap, data);
        out32(self.cap, self.port, config_address(bus, dev, func, 0x04));
        out32(self.cap, data, (command & 0xffff) | 0x06);
        u16::try_from(command & 0xffff).expect("low half")
    }

    fn port(&self, secondary: u8, enable: bool) -> Option<Port> {
        let bdf = self.find_root_port(secondary)?;
        let before = enable.then(|| self.enable_bridge(bdf));
        let mut config = [0u8; CONFIG_BYTES];
        self.read_config(bdf, &mut config);
        Some(Port {
            bdf,
            config,
            before,
        })
    }
}

/// Paint rows 31 to 51. `iommu` is the register snapshot, `None` when the
/// unit was not found or its control register read as all ones.
pub fn paint_probe_dump(pci: &Pci, iommu_base: u64, iommu: Option<[u64; 7]>) {
    let mut wifi = [0u8; CONFIG_BYTES];
    pci.read_config(MT7921_BDF, &mut wifi);
    let wifi_port = pci.port(MT7921_BDF.0, true);
    let nvme_port = pci.port(NVME_BDF.0, false);
    paint_summary(&wifi, wifi_port.as_ref(), nvme_port.as_ref());
    paint_config(ROW_WIFI_CONFIG, MT7921_BDF, &wifi);
    paint_port(ROW_WIFI_PORT, *b"rp", wifi_port.as_ref());
    paint_port(ROW_NVME_PORT, *b"np", nvme_port.as_ref());
    paint_iommu(iommu_base, iommu);
}

fn paint_summary(wifi: &[u8], wifi_port: Option<&Port>, nvme_port: Option<&Port>) {
    let mut line = [b' '; PANEL_COLS];
    let mut n = write_function_summary(&mut line, MT7921_BDF, wifi);
    line[n..n + 3].copy_from_slice(b" | ");
    n += 3;
    n += write_port_summary(&mut line[n..], b"rp", wifi_port.map(readback));
    line[n..n + 3].copy_from_slice(b" | ");
    n += 3;
    n += write_port_summary(&mut line[n..], b"np", nvme_port.map(readback));
    paint_full(ROW_SUMMARY, &line[..n]);
}

fn readback(port: &Port) -> PortReadback<'_> {
    PortReadback {
        bdf: port.bdf,
        config: &port.config,
        before: port.before,
    }
}

fn paint_port(row: usize, tag: [u8; 2], port: Option<&Port>) {
    let Some(port) = port else {
        let mut miss = *b"rp miss";
        miss[..2].copy_from_slice(&tag);
        paint_full(row, &miss);
        for extra in 1..CONFIG_ROWS {
            paint_full(row + extra, b"");
        }
        return;
    };
    paint_config(row, port.bdf, &port.config);
}

fn paint_config(row: usize, (bus, dev, func): (u8, u8, u8), config: &[u8]) {
    let mut words = [0u32; WORDS_PER_ROW];
    for index in 0..CONFIG_ROWS {
        let offset = index * ROW_BYTES;
        let count = config_words(config, offset, &mut words);
        let label = if index == 0 {
            bdf_label(bus, dev, func)
        } else {
            offset_label(offset)
        };
        let mut line = [b' '; PANEL_COLS];
        let n = write_hex_row(&mut line, &label, &words[..count]);
        paint_full(row + index, &line[..n]);
    }
}

fn paint_iommu(base: u64, regs: Option<[u64; 7]>) {
    let Some(regs) = regs else {
        paint_full(ROW_IOMMU, b"iommu miss");
        paint_full(ROW_IOMMU + 1, b"");
        return;
    };
    let mut line = [b' '; PANEL_COLS];
    let mut n = write_wide(&mut line, b"iommu", base);
    n += write_wide(&mut line[n..], b"ctrl", regs[3]);
    n += write_wide(&mut line[n..], b"excl", regs[4]);
    n += write_wide(&mut line[n..], b"limit", regs[5]);
    paint_full(ROW_IOMMU, &line[..n]);
    let mut line = [b' '; PANEL_COLS];
    let mut n = write_wide(&mut line, b"devtb", regs[0]);
    n += write_wide(&mut line[n..], b"cmdb", regs[1]);
    n += write_wide(&mut line[n..], b"evtlog", regs[2]);
    n += write_wide(&mut line[n..], b"sts", regs[6]);
    paint_full(ROW_IOMMU + 1, &line[..n]);
}

/// Pad to the full panel width so a second paint erases the first.
fn paint_full(row: usize, text: &[u8]) {
    let mut line = [b' '; PANEL_COLS];
    let n = text.len().min(PANEL_COLS);
    line[..n].copy_from_slice(&text[..n]);
    paint(row, &line);
}
