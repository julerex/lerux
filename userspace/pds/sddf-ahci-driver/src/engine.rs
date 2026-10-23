use core::ptr;

use lerux_machine::{msi_data, AhciMap, Claimed, Unclaimed, AHCI_IOAPIC_VECTOR, MSI_ADDRESS};
use sel4::with_ipc_buffer_mut;
use sel4_microkit::var;

const BASE_IOPORT_CAP: u64 = 394;
const PCI_CONFIG_ADDR_OFFSET: u16 = 0;
const PCI_CONFIG_DATA_OFFSET: u16 = 4;

const GHC: u32 = 0x04;
const IS: u32 = 0x08;
const PI: u32 = 0x0c;

const PORT0: u32 = 0x100;
const PX_CLB: u32 = PORT0;
const PX_CLBU: u32 = PORT0 + 0x04;
const PX_FB: u32 = PORT0 + 0x08;
const PX_FBU: u32 = PORT0 + 0x0c;
const PX_IS: u32 = PORT0 + 0x10;
const PX_IE: u32 = PORT0 + 0x14;
const PX_CMD: u32 = PORT0 + 0x18;
const PX_TFD: u32 = PORT0 + 0x20;
const PX_SSTS: u32 = PORT0 + 0x28;
const PX_SERR: u32 = PORT0 + 0x30;
const PX_CI: u32 = PORT0 + 0x38;

const CMD_LIST_OFF: u32 = 0x0;
const RX_FIS_OFF: u32 = 0x400;
const CMD_TBL_OFF: u32 = 0x500;
const BOUNCE_OFF: u32 = 0x1000;

const BOUNCE_SIZE: usize = 0x1_0000;
const SECTORS_PER_TRANSFER_BLOCK: u64 = 8;
const PXCI_POLL: usize = 10_000_000;

const FIS_REG_H2D: u8 = 0x27;
const ATA_CMD_IDENTIFY: u8 = 0xec;
const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;

#[repr(C, packed)]
struct HbaCmdHeader {
    flags: u32,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
}

#[repr(C, packed)]
struct HbaPrdtEntry {
    dba: u32,
    dbau: u32,
    rsv: u32,
    dbc_i: u32,
}

pub struct Engine {
    abar: usize,
    dma_vaddr: usize,
    dma_paddr: u64,
    capacity_blocks: u64,
}

#[derive(Debug)]
pub struct OpenError;

impl Engine {
    pub fn open(map: AhciMap) -> Result<Self, OpenError> {
        let pci = Pci::new();
        let _claimed = claim_device(&pci)?;
        pci.program(&map)?;
        let abar = usize::try_from(map.abar_vaddr).expect("abar vaddr");
        let dma_vaddr = usize::try_from(map.dma_vaddr).expect("dma vaddr");
        let dma_paddr = map.dma_paddr.value();
        init_host(abar, dma_paddr)?;
        let capacity_sectors = identify_device(abar, dma_vaddr, dma_paddr)?;
        let capacity_blocks = capacity_sectors / SECTORS_PER_TRANSFER_BLOCK;
        Ok(Self {
            abar,
            dma_vaddr,
            dma_paddr,
            capacity_blocks,
        })
    }

    pub fn capacity_blocks(&self) -> u64 {
        self.capacity_blocks
    }

    pub fn ack(&self) {
        let port_is = read_u32(self.abar, PX_IS);
        write_u32(self.abar, PX_IS, port_is);
        let global = read_u32(self.abar, IS);
        write_u32(self.abar, IS, global);
    }

    pub fn transfer(
        &mut self,
        read: bool,
        guest: *mut u8,
        len: usize,
        start_lba: u64,
    ) -> Result<(), ()> {
        if len == 0 {
            return Ok(());
        }
        let mut offset = 0usize;
        let mut lba = start_lba;
        let mut remaining = len;
        while remaining > 0 {
            let chunk = remaining.min(BOUNCE_SIZE);
            let sectors = u16::try_from(chunk / 512).expect("chunk is sector-aligned");
            if read {
                run_dma(
                    self.abar,
                    self.dma_vaddr,
                    self.dma_paddr,
                    false,
                    lba,
                    sectors,
                    ATA_CMD_READ_DMA_EXT,
                )?;
                // SAFETY: bounce lies inside the mapped DMA window and `chunk` fits.
                unsafe {
                    ptr::copy_nonoverlapping(
                        (self.dma_vaddr + BOUNCE_OFF as usize) as *const u8,
                        guest.add(offset),
                        chunk,
                    );
                }
            } else {
                // SAFETY: guest buffer is mapped for `len` bytes and `chunk` fits in the remainder.
                unsafe {
                    ptr::copy_nonoverlapping(
                        guest.add(offset),
                        (self.dma_vaddr + BOUNCE_OFF as usize) as *mut u8,
                        chunk,
                    );
                }
                run_dma(
                    self.abar,
                    self.dma_vaddr,
                    self.dma_paddr,
                    true,
                    lba,
                    sectors,
                    ATA_CMD_WRITE_DMA_EXT,
                )?;
            }
            offset += chunk;
            remaining -= chunk;
            lba += u64::from(sectors);
        }
        Ok(())
    }
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
        let ioport_id = *var!(pci_config_ioport_id: usize = 0) as u32;
        let base = *var!(pci_config_ioport_addr: usize = 0) as u16;
        let addr = Unclaimed::compiled().address();
        Self {
            cap: BASE_IOPORT_CAP + u64::from(ioport_id),
            base,
            bus: addr.bus(),
            dev: addr.device(),
            func: addr.function(),
        }
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

    fn program(&self, map: &AhciMap) -> Result<(), OpenError> {
        let bar = u32::try_from(map.abar_paddr.value()).expect("abar paddr fits u32");
        self.write_config(0x24, bar);
        let cmd = self.read_config(0x04);
        self.write_config(0x04, cmd | 0x06);
        enable_msi(self)?;
        Ok(())
    }
}

fn claim_device(pci: &Pci) -> Result<Claimed, OpenError> {
    let raw = pci.read_config(0x08);
    let class = (raw >> 8) & 0x00ff_ffff;
    Unclaimed::compiled().claim(class).map_err(|_| OpenError)
}

fn enable_msi(pci: &Pci) -> Result<(), OpenError> {
    let mut cap_ptr = (pci.read_config(0x34) & 0xff) as u8;
    while cap_ptr != 0 {
        // A config cycle addresses a dword. The capability id and the next pointer
        // are the low half. The message control, including the enable bit, is the high half.
        let header = pci.read_config(cap_ptr);
        let cap_id = (header & 0xff) as u8;
        let next = ((header >> 8) & 0xfc) as u8;
        if cap_id == 0x05 {
            let msg_ctl = (header >> 16) as u16;
            let data = msi_data(AHCI_IOAPIC_VECTOR);
            pci.write_config(cap_ptr + 4, MSI_ADDRESS);
            if msg_ctl & 0x0080 != 0 {
                pci.write_config(cap_ptr + 8, 0);
                pci.write_config(cap_ptr + 0xc, data);
            } else {
                pci.write_config(cap_ptr + 8, data);
            }
            let enabled = (header & 0x0000_ffff) | (u32::from(msg_ctl | 0x0001) << 16);
            pci.write_config(cap_ptr, enabled);
            return Ok(());
        }
        cap_ptr = next;
    }
    Err(OpenError)
}

fn read_u32(abar: usize, off: u32) -> u32 {
    // SAFETY: the protection domain maps the Advanced Host Controller Interface register block.
    unsafe { ptr::read_volatile((abar + off as usize) as *const u32) }
}

fn write_u32(abar: usize, off: u32, value: u32) {
    // SAFETY: the protection domain maps the Advanced Host Controller Interface register block.
    unsafe { ptr::write_volatile((abar + off as usize) as *mut u32, value) }
}

fn init_host(abar: usize, dma_paddr: u64) -> Result<(), OpenError> {
    write_u32(abar, GHC, read_u32(abar, GHC) | 1);
    for _ in 0..PXCI_POLL {
        if read_u32(abar, GHC) & 1 == 0 {
            break;
        }
        core::hint::spin_loop();
    }
    if read_u32(abar, GHC) & 1 != 0 {
        return Err(OpenError);
    }
    write_u32(abar, GHC, read_u32(abar, GHC) | (1 << 31) | (1 << 1));
    if read_u32(abar, PI) & 1 == 0 {
        return Err(OpenError);
    }
    stop_port(abar);
    let clb = u32::try_from(dma_paddr + CMD_LIST_OFF as u64).expect("clb paddr");
    let fb = u32::try_from(dma_paddr + RX_FIS_OFF as u64).expect("fb paddr");
    write_u32(abar, PX_CLB, clb);
    write_u32(abar, PX_CLBU, 0);
    write_u32(abar, PX_FB, fb);
    write_u32(abar, PX_FBU, 0);
    write_u32(abar, PX_SERR, read_u32(abar, PX_SERR));
    write_u32(abar, PX_IS, read_u32(abar, PX_IS));
    // Staggered spin-up keeps the link down until software requests it.
    if read_u32(abar, 0) & (1 << 27) != 0 {
        write_u32(abar, PX_CMD, read_u32(abar, PX_CMD) | (1 << 1));
    }
    for _ in 0..PXCI_POLL {
        if read_u32(abar, PX_SSTS) & 0xf == 0x3 {
            break;
        }
        core::hint::spin_loop();
    }
    if !start_port(abar) {
        return Err(OpenError);
    }
    // Device-to-host register FIS, PIO setup, and DMA setup. The global enable is useless alone.
    write_u32(abar, PX_IE, 0x3f);
    Ok(())
}

fn start_port(abar: usize) -> bool {
    write_u32(abar, PX_CMD, read_u32(abar, PX_CMD) | (1 << 4));
    for _ in 0..PXCI_POLL {
        if read_u32(abar, PX_CMD) & (1 << 14) != 0 {
            break;
        }
        core::hint::spin_loop();
    }
    write_u32(abar, PX_CMD, read_u32(abar, PX_CMD) | 1);
    for _ in 0..PXCI_POLL {
        if read_u32(abar, PX_CMD) & (1 << 15) != 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn stop_port(abar: usize) {
    write_u32(abar, PX_CMD, read_u32(abar, PX_CMD) & !(1 | (1 << 4)));
    for _ in 0..PXCI_POLL {
        let cmd = read_u32(abar, PX_CMD);
        if cmd & ((1 << 14) | (1 << 15)) == 0 {
            return;
        }
        core::hint::spin_loop();
    }
}

fn identify_device(abar: usize, dma_vaddr: usize, dma_paddr: u64) -> Result<u64, OpenError> {
    if run_dma(abar, dma_vaddr, dma_paddr, false, 0, 1, ATA_CMD_IDENTIFY).is_err() {
        return Err(OpenError);
    }
    let words = dma_vaddr + BOUNCE_OFF as usize;
    // SAFETY: identify returns 512 bytes into the bounce buffer.
    let w83 = unsafe { ptr::read_volatile((words + 83 * 2) as *const u16) };
    let lba48 = w83 & 0x0400 != 0;
    let sectors = if lba48 {
        let w100 = unsafe { ptr::read_volatile((words + 100 * 2) as *const u16) };
        let w101 = unsafe { ptr::read_volatile((words + 101 * 2) as *const u16) };
        let w102 = unsafe { ptr::read_volatile((words + 102 * 2) as *const u16) };
        let w103 = unsafe { ptr::read_volatile((words + 103 * 2) as *const u16) };
        u64::from(w100)
            | (u64::from(w101) << 16)
            | (u64::from(w102) << 32)
            | (u64::from(w103) << 48)
    } else {
        let w60 = unsafe { ptr::read_volatile((words + 60 * 2) as *const u16) };
        let w61 = unsafe { ptr::read_volatile((words + 61 * 2) as *const u16) };
        u64::from(w60) | (u64::from(w61) << 16)
    };
    Ok(sectors)
}

fn run_dma(
    abar: usize,
    dma_vaddr: usize,
    dma_paddr: u64,
    write: bool,
    lba: u64,
    sectors: u16,
    command: u8,
) -> Result<(), ()> {
    if read_u32(abar, PX_CMD) & (1 << 15) == 0 && !start_port(abar) {
        return Err(());
    }
    let byte_count = usize::from(sectors) * 512;
    build_command(
        dma_vaddr, dma_paddr, write, lba, sectors, command, byte_count,
    );
    write_u32(abar, PX_CI, 1);
    if !poll_pxci(abar) {
        stop_port(abar);
        return Err(());
    }
    let tfd = read_u32(abar, PX_TFD);
    if tfd & 0x1 != 0 {
        return Err(());
    }
    Ok(())
}

fn build_command(
    dma_vaddr: usize,
    dma_paddr: u64,
    write: bool,
    lba: u64,
    sectors: u16,
    command: u8,
    byte_count: usize,
) {
    let hdr_addr = dma_vaddr + CMD_LIST_OFF as usize;
    let tbl_addr = dma_vaddr + CMD_TBL_OFF as usize;
    let bounce_paddr = dma_paddr + BOUNCE_OFF as u64;
    let tbl_paddr = dma_paddr + CMD_TBL_OFF as u64;

    // SAFETY: command list and table live in the mapped DMA window.
    unsafe {
        // A command-list slot is 32 bytes. The header fields occupy the first 16.
        ptr::write_bytes(hdr_addr as *mut u8, 0, 32);
        let hdr = hdr_addr as *mut HbaCmdHeader;
        let cfl = 5u32;
        let w = u32::from(write);
        let prdtl = 1u32;
        (*hdr).flags = cfl | (w << 6) | (prdtl << 16);
        (*hdr).ctba = u32::try_from(tbl_paddr).expect("table paddr");
        (*hdr).ctbau = 0;

        ptr::write_bytes(
            tbl_addr as *mut u8,
            0,
            0x80 + core::mem::size_of::<HbaPrdtEntry>(),
        );
        let fis = tbl_addr as *mut u8;
        fis.write_volatile(FIS_REG_H2D);
        // Bit 7 tells the device this frame is a command, not a control update.
        fis.add(1).write_volatile(0x80);
        fis.add(2).write_volatile(command);
        fis.add(4).write_volatile((lba & 0xff) as u8);
        fis.add(5).write_volatile(((lba >> 8) & 0xff) as u8);
        fis.add(6).write_volatile(((lba >> 16) & 0xff) as u8);
        fis.add(7).write_volatile(0x40);
        fis.add(8).write_volatile(((lba >> 24) & 0xff) as u8);
        fis.add(9).write_volatile(((lba >> 32) & 0xff) as u8);
        fis.add(10).write_volatile(((lba >> 40) & 0xff) as u8);
        fis.add(12).write_volatile((sectors & 0xff) as u8);
        fis.add(13).write_volatile((sectors >> 8) as u8);

        let prdt = (tbl_addr + 0x80) as *mut HbaPrdtEntry;
        (*prdt).dba = u32::try_from(bounce_paddr).expect("bounce paddr");
        (*prdt).dbau = 0;
        let dbc = u32::try_from(byte_count - 1).expect("byte count");
        (*prdt).dbc_i = dbc;
    }
}

fn poll_pxci(abar: usize) -> bool {
    for _ in 0..PXCI_POLL {
        if read_u32(abar, PX_CI) & 1 == 0 {
            return read_u32(abar, PX_TFD) & 0x80 == 0;
        }
        core::hint::spin_loop();
    }
    false
}
