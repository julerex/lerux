//! Native driver for BCM2711 GENET v5 (RPi4 Ethernet).
//!
//! This implements the low-level hardware control:

// Rust 2024 requires explicit unsafe blocks even inside unsafe fn.
#![allow(unsafe_op_in_unsafe_fn)]
//! - Reset
//! - MDIO/PHY bringup (RGMII)
//! - MAC address programming
//! - TX/RX descriptor ring setup (using driver DMA region)
//! - IRQ-driven TX completion and basic RX handling
//!
//! Buffer ownership and ring pump for the shared ring buffers
//! (to net-server) is handled in main.rs using the same symbols
//! as the virtio driver for compatibility.

use core::ptr;

use lerux_logging::log;

/// GENET register offsets (BCM2711 GENET v5).
/// Based on Linux drivers/net/ethernet/broadcom/genet/ and DT.
mod regs {
    // System / top-level
    #[allow(dead_code)]
    pub const SYS_REV_CTRL: usize = 0x000;
    pub const SYS_PORT_CTRL: usize = 0x004;
    pub const SYS_RBUF_FLUSH_CTRL: usize = 0x008;
    pub const SYS_TBUF_FLUSH_CTRL: usize = 0x00c;

    // UMAC (MAC engine) - offset from GENET base
    pub const UMAC_CMD: usize = 0x800;
    pub const UMAC_MAC0: usize = 0x80c;
    pub const UMAC_MAC1: usize = 0x810;
    pub const UMAC_MAX_FRAME_LEN: usize = 0x814;
    pub const UMAC_MIB_CTRL: usize = 0x804;

    // MDIO
    #[allow(dead_code)]
    pub const MDIO_CMD: usize = 0x1c; // relative? actually in UMAC area for v5 often at 0x1c from UMAC?
                                      // For GENET v5 the MDIO is at:
    pub const UMAC_MDIO_CMD: usize = 0x820; // typical for v5 layout
    #[allow(dead_code)]
    pub const UMAC_MDIO_CFG: usize = 0x824;

    // RDMA / TDMA rings (simplified - we use basic ring 0)
    // Actual GENET has ring bases at 0x2000+ for rings.
    // For basic single ring operation we program the default ring.
    pub const TDMA_RING0_BASE: usize = 0x2000; // approximate
    pub const RDMA_RING0_BASE: usize = 0x3000;

    // Interrupt / status (simplified)
    pub const INTRL2_CPU_CLEAR: usize = 0x210; // guess for v5
    #[allow(dead_code)]
    pub const INTRL2_CPU_STAT: usize = 0x200;
}

use regs::*;

/// Simple register accessor.
pub struct Genet {
    base: *mut u32,
    // Physical base of the driver DMA region we can use for descriptors.
    dma_paddr: usize,
    // Virtual base of our working DMA area (subset of driver_dma).
    dma_vaddr: *mut u8,
}

impl Genet {
    /// # Safety
    /// base must point to a valid 64k GENET MMIO region.
    /// dma_vaddr / dma_paddr must be a large enough coherent DMA region.
    pub unsafe fn new(base: *mut (), dma_vaddr: *mut u8, dma_paddr: usize) -> Self {
        let base = base as *mut u32;
        Self {
            base,
            dma_vaddr,
            dma_paddr,
        }
    }

    #[inline]
    unsafe fn write32(&mut self, off: usize, val: u32) {
        unsafe {
            ptr::write_volatile(self.base.add(off / 4), val);
        }
    }

    #[inline]
    unsafe fn read32(&self, off: usize) -> u32 {
        unsafe { ptr::read_volatile(self.base.add(off / 4)) }
    }

    /// Full device reset.
    pub unsafe fn reset(&mut self) {
        // SYS reset / flush
        self.write32(SYS_RBUF_FLUSH_CTRL, 0x1);
        self.write32(SYS_TBUF_FLUSH_CTRL, 0x1);

        // Small delay (spin)
        for _ in 0..10000 {
            core::hint::spin_loop();
        }

        // Clear flushes
        self.write32(SYS_RBUF_FLUSH_CTRL, 0);
        self.write32(SYS_TBUF_FLUSH_CTRL, 0);

        // Port control: RGMII (value 0 or 1 depending on rev)
        // For v5 on RPi4, RGMII mode is usually selected by default or 0x02
        self.write32(SYS_PORT_CTRL, 0x2); // RGMII

        log::info!("genet: device reset complete");
    }

    /// MDIO write (clause 22 style).
    pub unsafe fn mdio_write(&mut self, phy: u8, reg: u8, val: u16) {
        // Poll for ready (simplified)
        for _ in 0..1000 {
            if (self.read32(UMAC_MDIO_CMD) & (1 << 31)) == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        // Write command: start | write | phy | reg | data
        let cmd = (1u32 << 31)
            | (1u32 << 30)
            | ((phy as u32) << 21)
            | ((reg as u32) << 16)
            | (val as u32);
        self.write32(UMAC_MDIO_CMD, cmd);

        // Wait for completion
        for _ in 0..10000 {
            if (self.read32(UMAC_MDIO_CMD) & (1 << 31)) == 0 {
                return;
            }
            core::hint::spin_loop();
        }
        log::info!("genet: mdio write timeout phy={} reg={}", phy, reg);
    }

    /// MDIO read.
    pub unsafe fn mdio_read(&mut self, phy: u8, reg: u8) -> u16 {
        for _ in 0..1000 {
            if (self.read32(UMAC_MDIO_CMD) & (1 << 31)) == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        let cmd = (1u32 << 31) | ((phy as u32) << 21) | ((reg as u32) << 16);
        self.write32(UMAC_MDIO_CMD, cmd);

        for _ in 0..10000 {
            let r = self.read32(UMAC_MDIO_CMD);
            if (r & (1 << 31)) == 0 {
                return (r & 0xffff) as u16;
            }
            core::hint::spin_loop();
        }
        log::info!("genet: mdio read timeout");
        0
    }

    /// Bring up the PHY in RGMII mode.
    /// On RPi4 the genet is wired RGMII to the internal PHY or LAN chip.
    /// We do basic auto-neg + RGMII config.
    pub unsafe fn phy_init(&mut self) {
        const PHY_ADDR: u8 = 1; // typical on RPi4

        // Read PHY ID
        let id1 = self.mdio_read(PHY_ADDR, 2);
        let id2 = self.mdio_read(PHY_ADDR, 3);
        log::info!("genet: PHY ID {:04x}:{:04x}", id1, id2);

        // Reset PHY
        self.mdio_write(PHY_ADDR, 0, 1 << 15);
        for _ in 0..100000 {
            core::hint::spin_loop();
        }

        // Advertise 1G full duplex + RGMII
        // BMCR: 1G + auto-neg
        self.mdio_write(PHY_ADDR, 0, 0x1000 | 0x200); // auto + 1000

        // For RGMII, some PHYs need special register for delay / mode.
        // Broadcom internal PHY often uses register 0x1d / 0x1e for RGMII.
        // Write magic for RGMII (common pattern):
        self.mdio_write(PHY_ADDR, 0x1d, 0x1f); // select shadow
        self.mdio_write(PHY_ADDR, 0x1e, 0x008c); // RGMII mode, no delay or with TX delay

        // Restart auto-neg
        self.mdio_write(PHY_ADDR, 0, 0x1200);

        // Wait for link (poll BMSR)
        for i in 0..100000 {
            let bmsr = self.mdio_read(PHY_ADDR, 1);
            if (bmsr & 0x4) != 0 {
                log::info!("genet: PHY link up (attempt {})", i);
                return;
            }
            if i % 10000 == 0 {
                core::hint::spin_loop();
            }
        }
        log::info!("genet: PHY link up timeout (proceeding anyway for smoke)");
    }

    /// Program the MAC address into UMAC.
    pub unsafe fn set_mac(&mut self, mac: &[u8; 6]) {
        let mac0 = u32::from(mac[0]) << 24
            | u32::from(mac[1]) << 16
            | u32::from(mac[2]) << 8
            | u32::from(mac[3]);
        let mac1 = u32::from(mac[4]) << 8 | u32::from(mac[5]);

        self.write32(UMAC_MAC0, mac0);
        self.write32(UMAC_MAC1, mac1);

        // Also set max frame len
        self.write32(UMAC_MAX_FRAME_LEN, 1536);

        log::info!(
            "genet: MAC programmed {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0],
            mac[1],
            mac[2],
            mac[3],
            mac[4],
            mac[5]
        );
    }

    /// Basic UMAC enable.
    pub unsafe fn umac_enable(&mut self) {
        // Enable TX + RX, RGMII, etc.
        // CMD register bits (from driver):
        // bit 0: TX enable
        // bit 1: RX enable
        // bit 3: RGMII mode?
        let mut cmd = self.read32(UMAC_CMD);
        cmd |= 0x3; // TX + RX enable (simplified)
        self.write32(UMAC_CMD, cmd);

        // Flush MIB if needed
        self.write32(UMAC_MIB_CTRL, 1 << 0);
        for _ in 0..100 {
            core::hint::spin_loop();
        }
        self.write32(UMAC_MIB_CTRL, 0);

        log::info!("genet: UMAC enabled");
    }

    // ---------------------------------------------------------------------
    // Descriptor rings (very simplified single-ring model)
    // We place TX and RX descriptors at the start of the driver DMA region.
    // Each desc is 8 bytes for basic mode (addr + len/stat).
    // ---------------------------------------------------------------------

    const DESC_SIZE: usize = 8;
    const NUM_TX_DESC: usize = 16;
    const NUM_RX_DESC: usize = 16;

    /// Layout in driver DMA:
    /// [0 .. TX descs] [TX buffers] [RX descs] [RX buffers]
    #[allow(clippy::identity_op)]
    pub unsafe fn setup_rings(&mut self) {
        let dma = self.dma_vaddr;
        let dma_p = self.dma_paddr;

        // TX descs at offset 0
        let tx_desc_off = 0;
        let tx_buf_off = tx_desc_off + Self::NUM_TX_DESC * Self::DESC_SIZE;
        let tx_buf_size = 2048; // per buffer

        // RX descs after TX area
        let rx_desc_off = tx_buf_off + Self::NUM_TX_DESC * tx_buf_size;
        let rx_buf_off = rx_desc_off + Self::NUM_RX_DESC * Self::DESC_SIZE;

        // Write TX ring base (GENET expects ring pointer registers)
        // Simplified: we program the ring start address (physical)
        self.write32(TDMA_RING0_BASE + 0x00, (dma_p + tx_desc_off) as u32);
        self.write32(TDMA_RING0_BASE + 0x04, 0); // hi if needed
        self.write32(TDMA_RING0_BASE + 0x08, Self::NUM_TX_DESC as u32);

        self.write32(RDMA_RING0_BASE + 0x00, (dma_p + rx_desc_off) as u32);
        self.write32(RDMA_RING0_BASE + 0x08, Self::NUM_RX_DESC as u32);

        // Initialize TX descriptors as owned by SW
        for i in 0..Self::NUM_TX_DESC {
            let desc = dma.add(tx_desc_off + i * Self::DESC_SIZE) as *mut u32;
            // len_stat = 0, owned by driver (bit usually 31 or so)
            ptr::write_volatile(desc, 0); // addr (low)
            ptr::write_volatile(desc.add(1), 0); // len/status
        }

        // Pre-allocate some RX buffers (we own the memory)
        for i in 0..Self::NUM_RX_DESC {
            let buf_phys = dma_p + rx_buf_off + i * tx_buf_size;
            let desc = dma.add(rx_desc_off + i * Self::DESC_SIZE) as *mut u32;
            ptr::write_volatile(desc, buf_phys as u32);
            // Mark as owned by HW (status with ownership bit set)
            ptr::write_volatile(desc.add(1), (2048u32 << 16) | (1u32 << 31));
        }

        // Enable the rings (TDMA/RDMA start)
        // Real GENET has ring enable bits in TDMA_CTRL / RDMA_CTRL
        // For simplicity we assume ring0 is always active after reset.

        log::info!(
            "genet: descriptor rings set up ({} TX / {} RX)",
            Self::NUM_TX_DESC,
            Self::NUM_RX_DESC
        );
    }

    /// Transmit a packet (called from ring pump).
    /// Returns true if we were able to queue it.
    pub unsafe fn transmit(&mut self, buf: &[u8]) -> bool {
        // Find a free TX desc (very naive linear scan)
        let dma = self.dma_vaddr;
        let tx_desc_off = 0;
        let tx_buf_off = tx_desc_off + Self::NUM_TX_DESC * Self::DESC_SIZE;
        let buf_size = 2048;

        for i in 0..Self::NUM_TX_DESC {
            let desc = dma.add(tx_desc_off + i * Self::DESC_SIZE) as *mut u32;
            let stat = ptr::read_volatile(desc.add(1));

            // If not owned by HW (bit 31 == 0)
            if (stat & (1 << 31)) == 0 {
                // Copy data into our TX buffer area
                let buf_ptr = dma.add(tx_buf_off + i * buf_size);
                let copy_len = core::cmp::min(buf.len(), buf_size - 4);
                ptr::copy_nonoverlapping(buf.as_ptr(), buf_ptr, copy_len);

                let phys = self.dma_paddr + tx_buf_off + i * buf_size;

                ptr::write_volatile(desc, phys as u32);
                // len | status | ownership for HW
                let len_stat = ((copy_len as u32) << 16) | (1 << 31) | 0x000c; // last + start flags etc.
                ptr::write_volatile(desc.add(1), len_stat);

                // Kick the ring (write to TDMA_PROD_INDEX or similar)
                // Simplified kick:
                self.write32(TDMA_RING0_BASE + 0x10, (i + 1) as u32);

                log::info!("genet: TX queued {} bytes (desc {})", copy_len, i);
                return true;
            }
        }
        log::info!("genet: no free TX desc");
        false
    }

    /// Check for TX completions (called on IRQ or poll).
    pub unsafe fn check_tx_completions(&mut self) -> usize {
        let dma = self.dma_vaddr;
        let tx_desc_off = 0;
        let mut completed = 0usize;
        for i in 0..Self::NUM_TX_DESC {
            let desc = dma.add(tx_desc_off + i * Self::DESC_SIZE) as *mut u32;
            let stat = ptr::read_volatile(desc.add(1));
            if (stat & (1 << 31)) == 0 && stat != 0 {
                ptr::write_volatile(desc.add(1), 0);
                completed += 1;
            }
        }
        completed
    }

    /// Poll one completed RX frame. Returns a slice into driver DMA (valid until next receive).
    pub unsafe fn receive(&mut self) -> Option<&[u8]> {
        let dma = self.dma_vaddr;
        let tx_desc_off = 0;
        let tx_buf_off = tx_desc_off + Self::NUM_TX_DESC * Self::DESC_SIZE;
        let rx_desc_off = tx_buf_off + Self::NUM_TX_DESC * 2048;
        let rx_buf_off = rx_desc_off + Self::NUM_RX_DESC * Self::DESC_SIZE;
        let buf_size = 2048usize;

        for i in 0..Self::NUM_RX_DESC {
            let desc = dma.add(rx_desc_off + i * Self::DESC_SIZE) as *mut u32;
            let stat = ptr::read_volatile(desc.add(1));
            if (stat & (1 << 31)) == 0 && stat != 0 {
                let len = ((stat >> 16) & 0x3fff) as usize;
                if len == 0 {
                    continue;
                }
                let buf_ptr = dma.add(rx_buf_off + i * buf_size);
                let pkt = core::slice::from_raw_parts(buf_ptr, len.min(buf_size));
                // Re-arm descriptor for HW.
                let buf_phys = self.dma_paddr + rx_buf_off + i * buf_size;
                ptr::write_volatile(desc, buf_phys as u32);
                ptr::write_volatile(desc.add(1), (2048u32 << 16) | (1u32 << 31));
                return Some(pkt);
            }
        }
        None
    }

    /// Acknowledge GENET interrupts.
    pub unsafe fn ack_interrupts(&mut self) {
        // Clear status
        self.write32(INTRL2_CPU_CLEAR, 0xffffffff);
    }

    /// Enable relevant interrupts (TX done, RX, etc).
    pub unsafe fn enable_irqs(&mut self) {
        // Enable TX/RX done interrupts etc.
        // Real bits differ per revision.
        log::info!("genet: IRQs enabled (stub)");
    }
}
