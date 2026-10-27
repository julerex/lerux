//! Owns PCI claim, BAR0, the link, both rings, and the private DMA slots.
//!
//! The handler is the only caller. One Microkit thread runs this domain.

use core::{
    mem::{offset_of, size_of},
    ptr,
    sync::atomic::{compiler_fence, Ordering},
};

use lerux_machine::e1000e::{
    e1000e_map, ClaimedNic, E1000eMap, UnclaimedNic, REGISTER_BAR, REGISTER_BAR_SIZE,
};
use sel4::with_ipc_buffer_mut;
use sel4_microkit::var;

pub(crate) const BUF_SIZE: usize = 2048;

const RING_LEN: usize = 32;
const RING_MASK: u16 = (RING_LEN as u16) - 1;

const CTRL: u32 = 0x0000;
const STATUS: u32 = 0x0008;
const EECD: u32 = 0x0010;
const ICR: u32 = 0x00c0;
const ITR: u32 = 0x00c4;
const IMS: u32 = 0x00d0;
const IMC: u32 = 0x00d8;
const RCTL: u32 = 0x0100;
const TCTL: u32 = 0x0400;
const TIPG: u32 = 0x0410;
const RDBAL: u32 = 0x2800;
const RDBAH: u32 = 0x2804;
const RDLEN: u32 = 0x2808;
const RDH: u32 = 0x2810;
const RDT: u32 = 0x2818;
const RDTR: u32 = 0x2820;
const RXDCTL: u32 = 0x2828;
const TDBAL: u32 = 0x3800;
const TDBAH: u32 = 0x3804;
const TDLEN: u32 = 0x3808;
const TDH: u32 = 0x3810;
const TDT: u32 = 0x3818;
const TXDCTL: u32 = 0x3828;
const RFCTL: u32 = 0x5008;
const MTA: u32 = 0x5200;
const RAL0: u32 = 0x5400;
const RAH0: u32 = 0x5404;

const CTRL_SLU: u32 = 1 << 6;
const CTRL_RST: u32 = 1 << 26;
const STATUS_LU: u32 = 1 << 1;
const EECD_AUTO_RD: u32 = 1 << 9;
const RCTL_EN: u32 = 1 << 1;
const RCTL_BAM: u32 = 1 << 15;
const RCTL_SECRC: u32 = 1 << 26;
const TCTL_EN: u32 = 1 << 1;
const TCTL_PSP: u32 = 1 << 3;
const TCTL_CT: u32 = 0x10 << 4;
const TCTL_COLD: u32 = 0x3f << 12;
const DCTL_ENABLE: u32 = 1 << 25;
const RFCTL_EXTEN: u32 = 1 << 15;
const RAH_AV: u32 = 1 << 31;
const IMS_TXDW: u32 = 1 << 0;
const IMS_LSC: u32 = 1 << 2;
const IMS_RXO: u32 = 1 << 6;
const IMS_RXT0: u32 = 1 << 7;

/// IPGT 8, IPGR1 8, IPGR2 6. The 82574 copper gap.
const TIPG_COPPER: u32 = 8 | (8 << 10) | (6 << 20);

const TX_CMD_EOP: u8 = 0x01;
const TX_CMD_IFCS: u8 = 0x02;
const TX_CMD_RS: u8 = 0x08;
const TX_DD: u8 = 0x01;
const RX_DD: u8 = 0x01;
const RX_EOP: u8 = 0x02;

const MTA_DWORDS: u32 = 128;
const DEVICE_SPIN: usize = 10_000_000;

const PCI_VENDOR: u8 = 0x00;
const PCI_COMMAND: u8 = 0x04;
const PCI_CAP_PTR: u8 = 0x34;
const CMD_MEMORY: u32 = 1 << 1;
const CMD_BUS_MASTER: u32 = 1 << 2;
const CMD_INTX_DISABLE: u32 = 1 << 10;
const CAP_MSI: u8 = 0x05;
const CAP_MSIX: u8 = 0x11;
const MSI_ENABLE: u32 = 1 << 16;
const MSIX_ENABLE: u32 = 1 << 31;

const BASE_IOPORT_CAP: u64 = 394;
const PCI_CONFIG_ADDR_OFFSET: u16 = 0;
const PCI_CONFIG_DATA_OFFSET: u16 = 4;

/// Owns the 82574. The handler cannot name a descriptor slot.
pub struct Engine {
    regs: usize,
    dma_vaddr: usize,
    dma_paddr: u64,
    /// Next transmit descriptor this side will write. Shadow of TDT.
    tx_tail: u16,
    /// Oldest transmit descriptor whose DD has not been observed.
    tx_clean: u16,
    /// Next receive descriptor to read. The gap sits at the previous index.
    rx_next: u16,
    /// Set from the RCTL.SECRC readback. [`frame_len`](Self::frame_len) is the only subtract of 4.
    crc_in_length: bool,
}

#[derive(Debug)]
pub struct OpenError;

#[repr(C)]
struct TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

#[repr(C)]
struct RxDesc {
    addr: u64,
    length: u16,
    csum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

const RING_PAD: usize = 4096 - RING_LEN * size_of::<TxDesc>();

#[repr(C, align(4096))]
struct DmaLayout {
    tx_ring: [TxDesc; RING_LEN],
    _tx_pad: [u8; RING_PAD],
    rx_ring: [RxDesc; RING_LEN],
    _rx_pad: [u8; RING_PAD],
    tx_bufs: [[u8; BUF_SIZE]; RING_LEN],
    rx_bufs: [[u8; BUF_SIZE]; RING_LEN],
}

const _: () = {
    assert!(offset_of!(DmaLayout, tx_ring) == 0);
    assert!(offset_of!(DmaLayout, rx_ring) == 0x1000);
    assert!(offset_of!(DmaLayout, tx_bufs) == 0x2000);
    assert!(offset_of!(DmaLayout, rx_bufs) == 0x1_2000);
    assert!(size_of::<DmaLayout>() == 0x2_2000);
    assert!(size_of::<TxDesc>() == 16);
    assert!(size_of::<RxDesc>() == 16);
    assert!(RING_LEN.is_power_of_two());
    assert!(size_of::<DmaLayout>() as u64 <= e1000e_map().dma_size);
};

impl Engine {
    /// Claim `8086:10d3` at the compiled BDF, program BAR0, reset, bring the
    /// link up, install `mac` into the receive address, and publish both rings.
    ///
    /// Call once. A second call would reset the NIC under live rings.
    pub fn open(map: E1000eMap, mac: [u8; 6]) -> Result<Self, OpenError> {
        let pci = Pci::new();
        let claimed = claim_device(&pci)?;
        pci.program(claimed, &map)?;
        let regs = usize::try_from(map.regs_vaddr).expect("regs vaddr");
        reset_mac(regs)?;
        // CTRL.RST reloads the EEPROM image into BAR0 and the command word,
        // and it can set the MSI and MSI-X enables. Either enable masks the pin.
        pci.write_window(&map)?;
        pci.force_pin()?;
        write_u32(regs, IMC, 0xffff_ffff);
        bring_link(regs)?;
        install_mac(regs, mac)?;
        let mut engine = Self {
            regs,
            dma_vaddr: usize::try_from(map.dma_vaddr).expect("dma vaddr"),
            dma_paddr: map.dma_paddr.value(),
            tx_tail: 0,
            tx_clean: 0,
            rx_next: 0,
            crc_in_length: false,
        };
        engine.publish_rings()?;
        engine.enable_paths();
        Ok(engine)
    }

    /// Read the cause register. Idempotent. Does not touch the sDDF queues.
    pub fn ack(&self) {
        let _ = read_u32(self.regs, ICR);
    }

    /// Copy `frame` into a private transmit buffer and ring the NIC.
    ///
    /// Returns nothing. The caller always returns the shared descriptor.
    /// A full ring or a frame that does not fit one buffer is dropped here.
    pub fn submit_tx(&mut self, frame: &[u8]) {
        self.reclaim_tx();
        let Ok(length) = u16::try_from(frame.len()) else {
            return;
        };
        if length == 0 || usize::from(length) > BUF_SIZE {
            return;
        }
        if next_index(self.tx_tail) == self.tx_clean {
            return;
        }
        let index = self.tx_tail;
        let phys = self.buf_phys(offset_of!(DmaLayout, tx_bufs), index);
        // SAFETY: the transmit buffer is inside the mapped DMA window and
        // `length` fits in one slot. The shared region and the DMA window
        // do not overlap.
        unsafe {
            ptr::copy_nonoverlapping(frame.as_ptr(), self.tx_buf(index), usize::from(length));
            ptr::write_volatile(
                self.tx_desc(index),
                TxDesc {
                    addr: phys,
                    length,
                    cso: 0,
                    cmd: TX_CMD_EOP | TX_CMD_IFCS | TX_CMD_RS,
                    status: 0,
                    css: 0,
                    special: 0,
                },
            );
        }
        compiler_fence(Ordering::Release);
        self.tx_tail = next_index(index);
        write_u32(self.regs, TDT, u32::from(self.tx_tail));
    }

    /// Copy one completed Ethernet frame into `dst` and retire that buffer.
    ///
    /// `Some(len)` means `len > 0` and the bytes start at the destination MAC.
    /// `None` means no descriptor is waiting.
    pub fn copy_rx(&mut self, dst: &mut [u8]) -> Option<u16> {
        loop {
            let index = self.rx_next;
            // SAFETY: the receive ring is inside the mapped DMA window.
            let header = unsafe { ptr::read_volatile(self.rx_desc(index)) };
            if header.status & RX_DD == 0 {
                return None;
            }
            compiler_fence(Ordering::Acquire);
            if let Some(len) = self.publishable(&header, dst.len()) {
                // SAFETY: `len` fits this buffer and `dst`. The private
                // window and the shared receive region do not overlap.
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.rx_buf(index),
                        dst.as_mut_ptr(),
                        usize::from(len),
                    );
                }
                self.retire_rx(index);
                return Some(len);
            }
            self.retire_rx(index);
        }
    }

    fn publishable(&self, header: &RxDesc, dst_len: usize) -> Option<u16> {
        if header.status & RX_EOP == 0 || header.errors != 0 {
            return None;
        }
        let len = self.frame_len(header.length)?;
        let n = usize::from(len);
        if n > dst_len || n > BUF_SIZE {
            None
        } else {
            Some(len)
        }
    }

    fn frame_len(&self, raw: u16) -> Option<u16> {
        let len = if self.crc_in_length {
            raw.checked_sub(4)?
        } else {
            raw
        };
        if len == 0 {
            None
        } else {
            Some(len)
        }
    }

    fn reclaim_tx(&mut self) {
        while self.tx_clean != self.tx_tail {
            // SAFETY: the transmit ring is inside the mapped DMA window.
            let header = unsafe { ptr::read_volatile(self.tx_desc(self.tx_clean)) };
            if header.status & TX_DD == 0 {
                return;
            }
            self.tx_clean = next_index(self.tx_clean);
        }
    }

    fn retire_rx(&mut self, index: u16) {
        let phys = self.buf_phys(offset_of!(DmaLayout, rx_bufs), index);
        // SAFETY: the receive ring is inside the mapped DMA window.
        unsafe {
            ptr::write_volatile(
                self.rx_desc(index),
                RxDesc {
                    addr: phys,
                    length: 0,
                    csum: 0,
                    status: 0,
                    errors: 0,
                    special: 0,
                },
            );
        }
        compiler_fence(Ordering::Release);
        write_u32(self.regs, RDT, u32::from(index));
        self.rx_next = next_index(index);
    }

    fn publish_rings(&mut self) -> Result<(), OpenError> {
        // SAFETY: e1000e_dma covers `DmaLayout` and this runs before the rings are armed.
        unsafe {
            ptr::write_bytes(self.dma_vaddr as *mut u8, 0, size_of::<DmaLayout>());
        }
        for slot in 0..RING_LEN {
            let index = u16::try_from(slot).expect("slot");
            let phys = self.buf_phys(offset_of!(DmaLayout, rx_bufs), index);
            // SAFETY: each index names one receive descriptor in the zeroed window.
            unsafe {
                ptr::write_volatile(
                    self.rx_desc(index),
                    RxDesc {
                        addr: phys,
                        length: 0,
                        csum: 0,
                        status: 0,
                        errors: 0,
                        special: 0,
                    },
                );
            }
        }
        compiler_fence(Ordering::Release);
        write_base(
            self.regs,
            RDBAL,
            RDBAH,
            self.dma_paddr + u64::try_from(offset_of!(DmaLayout, rx_ring)).expect("rx ring"),
        );
        write_base(
            self.regs,
            TDBAL,
            TDBAH,
            self.dma_paddr + u64::try_from(offset_of!(DmaLayout, tx_ring)).expect("tx ring"),
        );
        let len = u32::try_from(RING_LEN * size_of::<RxDesc>()).expect("ring bytes");
        write_u32(self.regs, RDLEN, len);
        write_u32(self.regs, TDLEN, len);
        write_u32(self.regs, RDH, 0);
        write_u32(self.regs, TDH, 0);
        write_u32(self.regs, TDT, 0);
        write_u32(self.regs, RDT, u32::from(RING_MASK));
        write_u32(self.regs, RXDCTL, read_u32(self.regs, RXDCTL) | DCTL_ENABLE);
        write_u32(self.regs, TXDCTL, read_u32(self.regs, TXDCTL) | DCTL_ENABLE);
        spin_until(|| {
            read_u32(self.regs, RXDCTL) & DCTL_ENABLE != 0
                && read_u32(self.regs, TXDCTL) & DCTL_ENABLE != 0
        })
    }

    fn enable_paths(&mut self) {
        // EXTEN selects the extended receive descriptor. DD is not in the
        // legacy status byte while that bit is set.
        write_u32(self.regs, RFCTL, read_u32(self.regs, RFCTL) & !RFCTL_EXTEN);
        // BSIZE 00 and BSEX clear select a 2048-byte receive buffer.
        let rctl = RCTL_EN | RCTL_BAM | RCTL_SECRC;
        write_u32(self.regs, RCTL, rctl);
        self.crc_in_length = read_u32(self.regs, RCTL) & RCTL_SECRC == 0;
        write_u32(self.regs, TCTL, TCTL_EN | TCTL_PSP | TCTL_CT | TCTL_COLD);
        write_u32(self.regs, TIPG, TIPG_COPPER);
        write_u32(self.regs, RDTR, 0);
        write_u32(self.regs, ITR, 0);
        write_u32(self.regs, IMS, IMS_TXDW | IMS_LSC | IMS_RXO | IMS_RXT0);
        let _ = read_u32(self.regs, ICR);
    }

    fn tx_desc(&self, index: u16) -> *mut TxDesc {
        let base = self.dma_vaddr + offset_of!(DmaLayout, tx_ring);
        (base as *mut TxDesc).wrapping_add(usize::from(index))
    }

    fn rx_desc(&self, index: u16) -> *mut RxDesc {
        let base = self.dma_vaddr + offset_of!(DmaLayout, rx_ring);
        (base as *mut RxDesc).wrapping_add(usize::from(index))
    }

    fn tx_buf(&self, index: u16) -> *mut u8 {
        self.buf_ptr(offset_of!(DmaLayout, tx_bufs), index)
    }

    fn rx_buf(&self, index: u16) -> *mut u8 {
        self.buf_ptr(offset_of!(DmaLayout, rx_bufs), index)
    }

    fn buf_ptr(&self, base_off: usize, index: u16) -> *mut u8 {
        (self.dma_vaddr + base_off + usize::from(index) * BUF_SIZE) as *mut u8
    }

    fn buf_phys(&self, base_off: usize, index: u16) -> u64 {
        self.dma_paddr
            + u64::try_from(base_off).expect("buffer offset")
            + u64::from(index) * u64::try_from(BUF_SIZE).expect("buffer size")
    }
}

fn next_index(index: u16) -> u16 {
    (index + 1) & RING_MASK
}

fn reset_mac(regs: usize) -> Result<(), OpenError> {
    write_u32(regs, IMC, 0xffff_ffff);
    write_u32(regs, CTRL, read_u32(regs, CTRL) | CTRL_RST);
    spin_until(|| read_u32(regs, CTRL) & CTRL_RST == 0 && read_u32(regs, EECD) & EECD_AUTO_RD != 0)
}

fn bring_link(regs: usize) -> Result<(), OpenError> {
    write_u32(regs, CTRL, read_u32(regs, CTRL) | CTRL_SLU);
    spin_until(|| read_u32(regs, STATUS) & STATUS_LU != 0)
}

fn install_mac(regs: usize, mac: [u8; 6]) -> Result<(), OpenError> {
    let ral = u32::from_le_bytes([mac[0], mac[1], mac[2], mac[3]]);
    let rah = u32::from_le_bytes([mac[4], mac[5], 0, 0]) | RAH_AV;
    write_u32(regs, RAL0, ral);
    write_u32(regs, RAH0, rah);
    if read_u32(regs, RAL0) != ral || read_u32(regs, RAH0) != rah {
        return Err(OpenError);
    }
    let mut entry = 0;
    while entry < MTA_DWORDS {
        write_u32(regs, MTA + entry * 4, 0);
        entry += 1;
    }
    Ok(())
}

fn write_base(regs: usize, low: u32, high: u32, phys: u64) {
    let low_bits = u32::try_from(phys & 0xffff_ffff).expect("ring low");
    let high_bits = u32::try_from(phys >> 32).expect("ring high");
    write_u32(regs, low, low_bits);
    write_u32(regs, high, high_bits);
}

fn spin_until(mut ready: impl FnMut() -> bool) -> Result<(), OpenError> {
    for _ in 0..DEVICE_SPIN {
        if ready() {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    if ready() {
        Ok(())
    } else {
        Err(OpenError)
    }
}

fn read_u32(regs: usize, off: u32) -> u32 {
    // SAFETY: the protection domain maps the 82574 register window.
    unsafe { ptr::read_volatile((regs + off as usize) as *const u32) }
}

fn write_u32(regs: usize, off: u32, value: u32) {
    // SAFETY: the protection domain maps the 82574 register window.
    unsafe { ptr::write_volatile((regs + off as usize) as *mut u32, value) }
}

struct Pci {
    cap: u64,
    base: u16,
    bus: u8,
    dev: u8,
    func: u8,
}

impl Pci {
    fn new() -> Self {
        let ioport_id = *var!(pci_net_config_ioport_id: usize = 0) as u32;
        let base = *var!(pci_net_config_ioport_addr: usize = 0) as u16;
        let addr = UnclaimedNic::compiled().address();
        Self {
            cap: BASE_IOPORT_CAP + u64::from(ioport_id),
            base,
            bus: addr.bus(),
            dev: addr.device(),
            func: addr.function(),
        }
    }

    fn program(&self, claimed: ClaimedNic, map: &E1000eMap) -> Result<(), OpenError> {
        let addr = claimed.address();
        if addr.bus() != self.bus || addr.device() != self.dev || addr.function() != self.func {
            return Err(OpenError);
        }
        self.write_window(map)?;
        self.force_pin()
    }

    fn write_window(&self, map: &E1000eMap) -> Result<(), OpenError> {
        let bar = u32::try_from(map.regs_paddr.value()).expect("regs paddr");
        if bar & 0xf != 0 {
            return Err(OpenError);
        }
        let offset = 0x10 + REGISTER_BAR * 4;
        self.write_config(offset, bar);
        let got = self.read_config(offset);
        let mask = !u32::try_from(REGISTER_BAR_SIZE - 1).expect("bar mask");
        if got & mask != bar & mask {
            return Err(OpenError);
        }
        let cmd = self.read_config(PCI_COMMAND);
        self.write_config(
            PCI_COMMAND,
            (cmd | CMD_MEMORY | CMD_BUS_MASTER) & !CMD_INTX_DISABLE,
        );
        Ok(())
    }

    fn force_pin(&self) -> Result<(), OpenError> {
        let mut cap_ptr = (self.read_config(PCI_CAP_PTR) & 0xff) as u8;
        for _ in 0..48 {
            if cap_ptr == 0 {
                return Ok(());
            }
            let header = self.read_config(cap_ptr);
            let cap_id = (header & 0xff) as u8;
            let next = ((header >> 8) & 0xfc) as u8;
            if cap_id == CAP_MSI {
                self.write_config(cap_ptr, header & !MSI_ENABLE);
            } else if cap_id == CAP_MSIX {
                self.write_config(cap_ptr, header & !MSIX_ENABLE);
            }
            cap_ptr = next;
        }
        Err(OpenError)
    }

    fn config_address(&self, reg: u8) -> u32 {
        0x8000_0000
            | (u32::from(self.bus) << 16)
            | (u32::from(self.dev) << 11)
            | (u32::from(self.func) << 8)
            | (u32::from(reg) & 0xfc)
    }

    fn out32(&self, port: u16, value: u32) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out32(self.cap, u64::from(port), u64::from(value));
        });
    }

    fn in32(&self, port: u16) -> u32 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In32(self.cap, port);
            ret.result
        })
    }

    fn read_config(&self, reg: u8) -> u32 {
        self.out32(self.base + PCI_CONFIG_ADDR_OFFSET, self.config_address(reg));
        self.in32(self.base + PCI_CONFIG_DATA_OFFSET)
    }

    fn write_config(&self, reg: u8, value: u32) {
        self.out32(self.base + PCI_CONFIG_ADDR_OFFSET, self.config_address(reg));
        self.out32(self.base + PCI_CONFIG_DATA_OFFSET, value);
    }
}

fn claim_device(pci: &Pci) -> Result<ClaimedNic, OpenError> {
    let id = pci.read_config(PCI_VENDOR);
    let vendor = (id & 0xffff) as u16;
    let device = (id >> 16) as u16;
    UnclaimedNic::compiled()
        .claim(vendor, device)
        .map_err(|_| OpenError)
}
