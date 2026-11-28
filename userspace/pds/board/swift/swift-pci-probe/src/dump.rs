//! Program the MT7921 link the way Linux does, without painting the dump.
//!
//! The root port is the bus 0 bridge whose secondary bus is the card's bus.
//! Linux `pci_enable_bridge` sets memory space and bus master on that port.
//! Linux `mt76_pci_disable_aspm` then clears ASPM on the endpoint and its
//! parent, and clock power management on the endpoint, before the driver
//! touches the chip. The panel no longer paints the configuration spaces,
//! the IOMMU registers, or the end marker that used to follow `bt: port 3 desc`.

use lerux_swift::{
    config_address,
    dump::{
        is_bridge, link_control_offset, secondary_bus, CONFIG_BYTES, ENDPOINT_LINK_CLEAR,
        PORT_LINK_CLEAR,
    },
    MT7921_BDF,
};

use crate::{in32, out32};

/// Link status shares the dword with link control. Bits 14 and 15 of the
/// status half are write-one-to-clear, so a read-modify-write leaves them 0.
const LINK_STATUS_W1C: u32 = 0xc000_0000;

pub struct Pci {
    pub cap: u64,
    pub port: u16,
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

    /// Memory space and bus master on a bridge.
    fn enable_bridge(&self, (bus, dev, func): (u8, u8, u8)) {
        let data = self.port.wrapping_add(4);
        out32(self.cap, self.port, config_address(bus, dev, func, 0x04));
        let command = in32(self.cap, data);
        out32(self.cap, self.port, config_address(bus, dev, func, 0x04));
        out32(self.cap, data, (command & 0xffff) | 0x06);
    }

    /// Clear `bits` of the link control register whose dword starts at the
    /// capability offset. A missing PCI Express capability leaves the link alone.
    fn clear_link(&self, (bus, dev, func): (u8, u8, u8), config: &[u8; CONFIG_BYTES], bits: u16) {
        let Some(offset) = link_control_offset(config) else {
            return;
        };
        let Ok(offset) = u8::try_from(offset) else {
            return;
        };
        let data = self.port.wrapping_add(4);
        out32(self.cap, self.port, config_address(bus, dev, func, offset));
        let word = in32(self.cap, data);
        if word == u32::MAX {
            return;
        }
        out32(self.cap, self.port, config_address(bus, dev, func, offset));
        out32(self.cap, data, word & !LINK_STATUS_W1C & !u32::from(bits));
    }

    /// Clear ASPM and clock power management on the MT7921.
    fn endpoint(&self) {
        let mut config = [0u8; CONFIG_BYTES];
        self.read_config(MT7921_BDF, &mut config);
        self.clear_link(MT7921_BDF, &config, ENDPOINT_LINK_CLEAR);
    }

    /// Enable the root port above the MT7921, then clear its ASPM bits.
    fn wifi_port(&self) {
        let Some(bdf) = self.find_root_port(MT7921_BDF.0) else {
            return;
        };
        self.enable_bridge(bdf);
        let mut config = [0u8; CONFIG_BYTES];
        self.read_config(bdf, &mut config);
        self.clear_link(bdf, &config, PORT_LINK_CLEAR);
    }
}

/// Endpoint first, then the parent, the order `mt76_pci_disable_aspm` uses.
/// The `NVMe` root port is not read or written.
pub fn program_wifi_link(pci: &Pci) {
    pci.endpoint();
    pci.wifi_port();
}
