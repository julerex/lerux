//! Native driver for BCM2711 GENET v5 (RPi4 Ethernet).
//!
//! Register map and ring layout follow Linux `drivers/net/ethernet/broadcom/genet/`
//! (BCM2711 uses GENET v5 with v4 DMA descriptor format: 12-byte BDs, ring 16).

// Rust 2024 requires explicit unsafe blocks even inside unsafe fn.
#![allow(unsafe_op_in_unsafe_fn)]

use core::ptr;

use lerux_logging::log;

/// GENET v5 / BCM2711 constants (from `bcmgenet.h` + `bcmgenet_hw_params_v4`).
mod hw {
    pub const GENET_EXT_OFF: usize = 0x0080;
    pub const GENET_INTRL2_0_OFF: usize = 0x0200;
    pub const GENET_INTRL2_1_OFF: usize = 0x0240;
    pub const GENET_UMAC_OFF: usize = 0x0800;

    pub const TDMA_OFFSET: usize = 0x4000;
    pub const RDMA_OFFSET: usize = 0x2000;
    pub const TOTAL_DESC: usize = 256;
    pub const DESC_BYTES: usize = 12; // words_per_bd = 3
    pub const DMA_RING_SIZE: usize = 0x40;
    pub const DESC_RING: usize = 16; // descriptor-based ring index

    pub const TDMA_REG_OFF: usize = TDMA_OFFSET + TOTAL_DESC * DESC_BYTES;
    pub const RDMA_REG_OFF: usize = RDMA_OFFSET + TOTAL_DESC * DESC_BYTES;
    pub const DMA_RINGS_SIZE: usize = DMA_RING_SIZE * (DESC_RING + 1);

    // Global DMA block sits after all per-ring register windows.
    pub const TDMA_GLOBAL_OFF: usize = TDMA_REG_OFF + DMA_RINGS_SIZE;
    pub const RDMA_GLOBAL_OFF: usize = RDMA_REG_OFF + DMA_RINGS_SIZE;
}

#[allow(dead_code)]
mod regs {
    pub const SYS_REV_CTRL: usize = 0x000;
    pub const SYS_PORT_CTRL: usize = 0x004;
    pub const SYS_RBUF_FLUSH_CTRL: usize = 0x008;
    pub const SYS_TBUF_FLUSH_CTRL: usize = 0x00c;

    pub const PORT_MODE_INT_GPHY: u32 = 1;

    pub const EXT_GPHY_CTRL: usize = 0x1c;
    pub const EXT_RGMII_OOB_CTRL: usize = 0x0c;
    pub const EXT_GPHY_RESET: u32 = 1 << 5;
    pub const EXT_CFG_PWR_DOWN: u32 = 1 << 1;
    pub const EXT_CFG_IDDQ_BIAS: u32 = 1 << 0;

    // UniMAC (offsets relative to GENET_UMAC_OFF).
    pub const UMAC_CMD: usize = 0x008;
    pub const UMAC_MAC0: usize = 0x00c;
    pub const UMAC_MAC1: usize = 0x010;
    pub const UMAC_MAX_FRAME_LEN: usize = 0x014;
    pub const UMAC_MIB_CTRL: usize = 0x580;
    pub const UMAC_MDIO_CMD: usize = 0x614;

    pub const CMD_TX_EN: u32 = 1 << 0;
    pub const CMD_RX_EN: u32 = 1 << 1;
    pub const CMD_CRC_FWD: u32 = 1 << 6;
    pub const CMD_PAD_EN: u32 = 1 << 5;
    pub const MIB_RESET_RX: u32 = 1 << 0;

    pub const MDIO_START_BUSY: u32 = 1 << 29;
    pub const MDIO_RD: u32 = 2 << 26;
    pub const MDIO_WR: u32 = 1 << 26;
    pub const MDIO_PMD_SHIFT: u32 = 21;
    pub const MDIO_REG_SHIFT: u32 = 16;

    // INTRL2 (offsets relative to each INTRL2 instance base).
    pub const INTRL2_CPU_STAT: usize = 0x00;
    pub const INTRL2_CPU_CLEAR: usize = 0x08;
    pub const INTRL2_CPU_MASK_CLEAR: usize = 0x14;

    pub const UMAC_IRQ_RXDMA_DONE: u32 = 1 << 13;
    pub const UMAC_IRQ_TXDMA_DONE: u32 = 1 << 16;

    // v4 ring register layout (byte offset within a ring's 0x40 window).
    pub const TDMA_PROD_INDEX: usize = 0x0c;
    pub const TDMA_CONS_INDEX: usize = 0x08;
    pub const RDMA_PROD_INDEX: usize = 0x0c;
    pub const RDMA_CONS_INDEX: usize = 0x08;
    pub const DMA_RING_BUF_SIZE: usize = 0x10;
    pub const DMA_START_ADDR: usize = 0x14;
    pub const DMA_END_ADDR: usize = 0x1c;

    // Global DMA control (offset within global DMA block).
    pub const DMA_CTRL: usize = 0x04;
    pub const DMA_EN: u32 = 1 << 0;

    // Descriptor status bits.
    pub const DMA_OWN: u16 = 0x8000;
    pub const DMA_EOP: u16 = 0x4000;
    pub const DMA_SOP: u16 = 0x2000;
    pub const DMA_TX_APPEND_CRC: u16 = 0x0040;
    pub const DMA_BUFLENGTH_SHIFT: u16 = 16;
    pub const DMA_BUFLENGTH_MASK: u16 = 0x0fff;
}

use hw::*;
use regs::*;

/// Simple register accessor for GENET MMIO.
pub struct Genet {
    base: *mut u8,
    dma_paddr: usize,
    dma_vaddr: *mut u8,
    tx_prod: u16,
    rx_cons: u16,
}

impl Genet {
    /// # Safety
    /// `base` must point to a valid GENET MMIO region; DMA pointers must cover buffers.
    pub unsafe fn new(base: *mut (), dma_vaddr: *mut u8, dma_paddr: usize) -> Self {
        Self {
            base: base as *mut u8,
            dma_vaddr,
            dma_paddr,
            tx_prod: 0,
            rx_cons: 0,
        }
    }

    #[inline]
    unsafe fn write32(&mut self, off: usize, val: u32) {
        unsafe {
            ptr::write_volatile(self.base.add(off) as *mut u32, val);
        }
    }

    #[inline]
    unsafe fn read32(&self, off: usize) -> u32 {
        unsafe { ptr::read_volatile(self.base.add(off) as *const u32) }
    }

    #[inline]
    unsafe fn umac_write(&mut self, off: usize, val: u32) {
        self.write32(GENET_UMAC_OFF + off, val);
    }

    #[inline]
    unsafe fn umac_read(&self, off: usize) -> u32 {
        self.read32(GENET_UMAC_OFF + off)
    }

    #[inline]
    unsafe fn tdma_ring_write(&mut self, ring: usize, reg: usize, val: u32) {
        self.write32(TDMA_REG_OFF + ring * DMA_RING_SIZE + reg, val);
    }

    #[inline]
    unsafe fn rdma_ring_write(&mut self, ring: usize, reg: usize, val: u32) {
        self.write32(RDMA_REG_OFF + ring * DMA_RING_SIZE + reg, val);
    }

    #[inline]
    unsafe fn tdma_global_write(&mut self, reg: usize, val: u32) {
        self.write32(TDMA_GLOBAL_OFF + reg, val);
    }

    #[inline]
    unsafe fn rdma_global_write(&mut self, reg: usize, val: u32) {
        self.write32(RDMA_GLOBAL_OFF + reg, val);
    }

    #[inline]
    unsafe fn tx_desc_ptr(&self, index: usize) -> *mut u32 {
        self.base.add(TDMA_OFFSET + index * DESC_BYTES) as *mut u32
    }

    #[inline]
    unsafe fn rx_desc_ptr(&self, index: usize) -> *mut u32 {
        self.base.add(RDMA_OFFSET + index * DESC_BYTES) as *mut u32
    }

    const NUM_DESC: usize = 16;
    const BUF_SIZE: usize = 2048;

    fn tx_buf_off(index: usize) -> usize {
        index * Self::BUF_SIZE
    }

    fn rx_buf_off(index: usize) -> usize {
        Self::NUM_DESC * Self::BUF_SIZE + index * Self::BUF_SIZE
    }

    /// Full device reset and EXT/UMAC prep for internal GPHY (RPi4).
    pub unsafe fn reset(&mut self) {
        self.write32(SYS_RBUF_FLUSH_CTRL, 0x1);
        self.write32(SYS_TBUF_FLUSH_CTRL, 0x1);
        for _ in 0..10_000 {
            core::hint::spin_loop();
        }
        self.write32(SYS_RBUF_FLUSH_CTRL, 0);
        self.write32(SYS_TBUF_FLUSH_CTRL, 0);

        // Internal GPHY on BCM2711 (not external RGMII PHY).
        self.write32(SYS_PORT_CTRL, PORT_MODE_INT_GPHY);

        let mut gphy = self.read32(GENET_EXT_OFF + EXT_GPHY_CTRL);
        gphy &= !(EXT_CFG_PWR_DOWN | EXT_CFG_IDDQ_BIAS);
        gphy |= EXT_GPHY_RESET;
        self.write32(GENET_EXT_OFF + EXT_GPHY_CTRL, gphy);
        for _ in 0..10_000 {
            core::hint::spin_loop();
        }
        gphy &= !EXT_GPHY_RESET;
        self.write32(GENET_EXT_OFF + EXT_GPHY_CTRL, gphy);
        for _ in 0..50_000 {
            core::hint::spin_loop();
        }

        let rev = self.read32(SYS_REV_CTRL);
        log::info!("genet: reset complete rev={rev:#010x}");
    }

    unsafe fn mdio_wait_idle(&self) {
        for _ in 0..10_000 {
            if self.umac_read(UMAC_MDIO_CMD) & MDIO_START_BUSY == 0 {
                return;
            }
            core::hint::spin_loop();
        }
    }

    /// MDIO write (clause 22).
    pub unsafe fn mdio_write(&mut self, phy: u8, reg: u8, val: u16) {
        self.mdio_wait_idle();
        let cmd = MDIO_START_BUSY
            | MDIO_WR
            | (u32::from(phy) << MDIO_PMD_SHIFT)
            | (u32::from(reg) << MDIO_REG_SHIFT)
            | u32::from(val);
        self.umac_write(UMAC_MDIO_CMD, cmd);
        self.mdio_wait_idle();
    }

    /// MDIO read.
    pub unsafe fn mdio_read(&mut self, phy: u8, reg: u8) -> u16 {
        self.mdio_wait_idle();
        let cmd = MDIO_START_BUSY
            | MDIO_RD
            | (u32::from(phy) << MDIO_PMD_SHIFT)
            | (u32::from(reg) << MDIO_REG_SHIFT);
        self.umac_write(UMAC_MDIO_CMD, cmd);
        self.mdio_wait_idle();
        (self.umac_read(UMAC_MDIO_CMD) & 0xffff) as u16
    }

    /// Bring up the internal GPHY (simplified auto-negotiation).
    pub unsafe fn phy_init(&mut self) {
        const PHY_ADDR: u8 = 0;

        let id1 = self.mdio_read(PHY_ADDR, 2);
        let id2 = self.mdio_read(PHY_ADDR, 3);
        log::info!("genet: PHY ID {:04x}:{:04x}", id1, id2);

        // BMCR reset + restart auto-negotiation.
        self.mdio_write(PHY_ADDR, 0, 1 << 15);
        for _ in 0..100_000 {
            core::hint::spin_loop();
        }
        self.mdio_write(PHY_ADDR, 0, 0x1200);

        for i in 0..200_000 {
            let bmsr = self.mdio_read(PHY_ADDR, 1);
            if (bmsr & 0x4) != 0 {
                log::info!("genet: PHY link up (attempt {i})");
                return;
            }
            if i % 10_000 == 0 {
                core::hint::spin_loop();
            }
        }
        log::info!("genet: PHY link timeout (continuing for smoke)");
    }

    /// Program the MAC address into UMAC.
    pub unsafe fn set_mac(&mut self, mac: &[u8; 6]) {
        let mac0 = u32::from(mac[0]) << 24
            | u32::from(mac[1]) << 16
            | u32::from(mac[2]) << 8
            | u32::from(mac[3]);
        let mac1 = u32::from(mac[4]) << 8 | u32::from(mac[5]);

        self.umac_write(UMAC_MAC0, mac0);
        self.umac_write(UMAC_MAC1, mac1);
        self.umac_write(UMAC_MAX_FRAME_LEN, 1536);

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

    /// Enable UMAC TX/RX.
    pub unsafe fn umac_enable(&mut self) {
        self.umac_write(UMAC_CMD, CMD_TX_EN | CMD_RX_EN | CMD_CRC_FWD | CMD_PAD_EN);
        self.umac_write(UMAC_MIB_CTRL, MIB_RESET_RX);
        for _ in 0..100 {
            core::hint::spin_loop();
        }
        self.umac_write(UMAC_MIB_CTRL, 0);
        log::info!("genet: UMAC enabled");
    }

    unsafe fn write_tx_desc(&mut self, index: usize, buf_phys: usize, len: usize, arm: bool) {
        let desc = self.tx_desc_ptr(index);
        let mut stat = ((len as u32) << DMA_BUFLENGTH_SHIFT as u32) | u32::from(DMA_SOP | DMA_EOP);
        if arm {
            stat |= u32::from(DMA_OWN | DMA_TX_APPEND_CRC);
        }
        ptr::write_volatile(desc, stat);
        ptr::write_volatile(desc.add(1), buf_phys as u32);
        ptr::write_volatile(desc.add(2), 0);
    }

    unsafe fn write_rx_desc(&mut self, index: usize, buf_phys: usize) {
        let desc = self.rx_desc_ptr(index);
        let stat =
            (u32::from(Self::BUF_SIZE as u16) << DMA_BUFLENGTH_SHIFT as u32) | u32::from(DMA_OWN);
        ptr::write_volatile(desc, stat);
        ptr::write_volatile(desc.add(1), buf_phys as u32);
        ptr::write_volatile(desc.add(2), 0);
    }

    unsafe fn read_desc_status(&self, desc: *mut u32) -> u16 {
        (ptr::read_volatile(desc) & 0xffff) as u16
    }

    /// Set up descriptor ring 16 and arm RX buffers.
    pub unsafe fn setup_rings(&mut self) {
        let ring = DESC_RING;
        let end_ptr = Self::NUM_DESC * DESC_BYTES;

        self.tdma_ring_write(ring, TDMA_PROD_INDEX, 0);
        self.tdma_ring_write(ring, TDMA_CONS_INDEX, 0);
        self.tdma_ring_write(
            ring,
            DMA_RING_BUF_SIZE,
            ((Self::NUM_DESC as u32) << 16) | Self::BUF_SIZE as u32,
        );
        self.tdma_ring_write(ring, DMA_START_ADDR, 0);
        self.tdma_ring_write(ring, DMA_END_ADDR, end_ptr as u32 - 1);

        self.rdma_ring_write(ring, RDMA_PROD_INDEX, 0);
        self.rdma_ring_write(ring, RDMA_CONS_INDEX, 0);
        self.rdma_ring_write(
            ring,
            DMA_RING_BUF_SIZE,
            ((Self::NUM_DESC as u32) << 16) | Self::BUF_SIZE as u32,
        );
        self.rdma_ring_write(ring, DMA_START_ADDR, 0);
        self.rdma_ring_write(ring, DMA_END_ADDR, end_ptr as u32 - 1);

        for i in 0..Self::NUM_DESC {
            self.write_tx_desc(i, 0, 0, false);
            let phys = self.dma_paddr + Self::rx_buf_off(i);
            self.write_rx_desc(i, phys);
        }

        self.tx_prod = 0;
        self.rx_cons = 0;

        self.rdma_global_write(DMA_CTRL, DMA_EN);
        self.tdma_global_write(DMA_CTRL, DMA_EN);

        log::info!(
            "genet: ring {ring} ready ({} TX / {} RX desc)",
            Self::NUM_DESC,
            Self::NUM_DESC
        );
    }

    /// Transmit a packet via ring 16.
    pub unsafe fn transmit(&mut self, buf: &[u8]) -> bool {
        let index = usize::from(self.tx_prod % Self::NUM_DESC as u16);
        let desc = self.tx_desc_ptr(index);
        let stat = self.read_desc_status(desc);
        if stat & DMA_OWN != 0 {
            return false;
        }

        let copy_len = buf.len().min(Self::BUF_SIZE);
        let buf_ptr = self.dma_vaddr.add(Self::tx_buf_off(index));
        ptr::copy_nonoverlapping(buf.as_ptr(), buf_ptr, copy_len);

        let phys = self.dma_paddr + Self::tx_buf_off(index);
        self.write_tx_desc(index, phys, copy_len, true);

        self.tx_prod = self.tx_prod.wrapping_add(1);
        self.tdma_ring_write(DESC_RING, TDMA_PROD_INDEX, u32::from(self.tx_prod));
        true
    }

    /// Reclaim completed TX descriptors (IRQ or poll).
    pub unsafe fn check_tx_completions(&mut self) -> usize {
        let mut completed = 0usize;
        for i in 0..Self::NUM_DESC {
            let desc = self.tx_desc_ptr(i);
            let stat = self.read_desc_status(desc);
            if stat != 0 && stat & DMA_OWN == 0 {
                ptr::write_volatile(desc, 0);
                ptr::write_volatile(desc.add(1), 0);
                ptr::write_volatile(desc.add(2), 0);
                completed += 1;
            }
        }
        completed
    }

    /// Poll one completed RX frame.
    pub unsafe fn receive(&mut self) -> Option<&[u8]> {
        let index = usize::from(self.rx_cons % Self::NUM_DESC as u16);
        let desc = self.rx_desc_ptr(index);
        let stat = self.read_desc_status(desc);
        if stat & DMA_OWN != 0 || stat == 0 {
            return None;
        }

        let len = ((stat & DMA_BUFLENGTH_MASK) as usize).min(Self::BUF_SIZE);
        if len == 0 {
            return None;
        }

        let buf_ptr = self.dma_vaddr.add(Self::rx_buf_off(index));
        let pkt = core::slice::from_raw_parts(buf_ptr, len);

        let phys = self.dma_paddr + Self::rx_buf_off(index);
        self.write_rx_desc(index, phys);
        self.rx_cons = self.rx_cons.wrapping_add(1);
        self.rdma_ring_write(DESC_RING, RDMA_CONS_INDEX, u32::from(self.rx_cons));

        Some(pkt)
    }

    /// Clear GENET interrupt status.
    pub unsafe fn ack_interrupts(&mut self) {
        self.write32(GENET_INTRL2_0_OFF + INTRL2_CPU_CLEAR, 0xffff_ffff);
        self.write32(GENET_INTRL2_1_OFF + INTRL2_CPU_CLEAR, 0xffff_ffff);
    }

    /// Unmask TX/RX DMA done interrupts on INTRL2 instance 0.
    pub unsafe fn enable_irqs(&mut self) {
        let mask = UMAC_IRQ_RXDMA_DONE | UMAC_IRQ_TXDMA_DONE;
        self.write32(GENET_INTRL2_0_OFF + INTRL2_CPU_MASK_CLEAR, mask);
        log::info!("genet: IRQs enabled (RX/TX DMA done)");
    }
}
