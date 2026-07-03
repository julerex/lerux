//! BCM2711 EMMC2 (SDHCI) block device for RPi4 SD/MMC slot.
//!
//! Uses programmed I/O (no ADMA) for sector transfers. Card init follows the
//! standard SD memory card identification flow (CMD0 → ACMD41 → CMD2/3/7/16).

#![allow(unsafe_op_in_unsafe_fn)]

use core::ptr;

use lerux_logging::log;
use sel4_driver_interfaces::block::GetBlockDeviceLayout;

const SECTOR_SIZE: usize = 512;

/// Standard SDHCI register offsets (16-bit unless noted).
mod regs {
    pub const BLOCK_SIZE: usize = 0x04;
    pub const BLOCK_COUNT: usize = 0x06;
    pub const ARGUMENT: usize = 0x08;
    pub const TRANSFER_MODE: usize = 0x0c;
    pub const COMMAND: usize = 0x0e;
    pub const RESPONSE: usize = 0x10;
    pub const BUFFER_DATA: usize = 0x20;
    pub const PRESENT_STATE: usize = 0x24;
    #[expect(dead_code, reason = "SDHCI host control; reserved for bus-width setup")]
    pub const HOST_CONTROL: usize = 0x28;
    pub const POWER_CONTROL: usize = 0x29;
    pub const CLOCK_CONTROL: usize = 0x2c;
    pub const TIMEOUT_CONTROL: usize = 0x2e;
    pub const SOFTWARE_RESET: usize = 0x2f;
    pub const NORMAL_INT_STATUS: usize = 0x30;
    pub const ERROR_INT_STATUS: usize = 0x32;
    pub const NORMAL_INT_ENABLE: usize = 0x34;
    pub const NORMAL_INT_SIGNAL_ENABLE: usize = 0x38;
    pub const HOST_CONTROL2: usize = 0x3e;
}

use regs::*;

const CMD_INHIBIT_CMD: u32 = 1 << 0;
const CMD_INHIBIT_DAT: u32 = 1 << 1;
const PRESENT_CARD_INSERTED: u32 = 1 << 16;
const PRESENT_CARD_STABLE: u32 = 1 << 17;

const INT_CMD_COMPLETE: u16 = 1 << 0;
const INT_TRANSFER_COMPLETE: u16 = 1 << 1;
const INT_BUFFER_READ_READY: u16 = 1 << 5;
const INT_BUFFER_WRITE_READY: u16 = 1 << 4;
const INT_ERROR: u16 = 1 << 15;

const RESP_NONE: u16 = 0x00;
const RESP_136: u16 = 0x01;
const RESP_48: u16 = 0x02;
const RESP_48_BUSY: u16 = 0x03;

const CMD_CRC_EN: u16 = 0x08;
const CMD_INDEX_EN: u16 = 0x10;
const CMD_DATA_PRESENT: u16 = 0x20;

const TRANSFER_READ: u16 = 1 << 4;
const TRANSFER_SINGLE: u16 = 1 << 5;

const RESET_ALL: u8 = 0x01;
const POWER_3_3V: u8 = 0x0e;
const POWER_ON: u8 = 0x01;

const HC2_1V8_SIG: u16 = 1 << 3;

#[derive(Debug, Clone, Copy)]
pub enum Emmc2Error {
    Timeout,
    #[expect(dead_code, reason = "reserved for card-detect gate")]
    CardNotPresent,
    CommandFailed,
    IoError,
}

pub struct Emmc2 {
    base: *mut u8,
    rca: u16,
    num_blocks: u64,
    initialized: bool,
}

impl Emmc2 {
    /// # Safety
    /// `base` must map a valid SDHCI register window for the RPi4 EMMC2 controller.
    pub unsafe fn new(base: *mut ()) -> Self {
        Self {
            base: base as *mut u8,
            rca: 0,
            num_blocks: 0,
            initialized: false,
        }
    }

    #[inline]
    unsafe fn read8(&self, off: usize) -> u8 {
        unsafe { ptr::read_volatile(self.base.add(off)) }
    }

    #[inline]
    unsafe fn write8(&mut self, off: usize, val: u8) {
        unsafe {
            ptr::write_volatile(self.base.add(off), val);
        }
    }

    #[inline]
    unsafe fn read16(&self, off: usize) -> u16 {
        unsafe { ptr::read_volatile(self.base.add(off) as *const u16) }
    }

    #[inline]
    unsafe fn write16(&mut self, off: usize, val: u16) {
        unsafe {
            ptr::write_volatile(self.base.add(off) as *mut u16, val);
        }
    }

    #[inline]
    unsafe fn read32(&self, off: usize) -> u32 {
        unsafe { ptr::read_volatile(self.base.add(off) as *const u32) }
    }

    unsafe fn wait_cmd_ready(&mut self) -> Result<(), Emmc2Error> {
        for _ in 0..500_000 {
            let state = self.read32(PRESENT_STATE);
            if state & CMD_INHIBIT_CMD == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(Emmc2Error::Timeout)
    }

    unsafe fn wait_dat_ready(&mut self) -> Result<(), Emmc2Error> {
        for _ in 0..500_000 {
            let state = self.read32(PRESENT_STATE);
            if state & CMD_INHIBIT_DAT == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(Emmc2Error::Timeout)
    }

    unsafe fn wait_int(&mut self, mask: u16) -> Result<(), Emmc2Error> {
        for _ in 0..1_000_000 {
            let status = self.read16(NORMAL_INT_STATUS);
            if status & INT_ERROR != 0 {
                let err = self.read16(ERROR_INT_STATUS);
                log::info!("emmc2: error int status {err:#06x}");
                self.write16(NORMAL_INT_STATUS, status);
                self.write16(ERROR_INT_STATUS, err);
                return Err(Emmc2Error::CommandFailed);
            }
            if status & mask != 0 {
                self.write16(NORMAL_INT_STATUS, status);
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(Emmc2Error::Timeout)
    }

    unsafe fn send_cmd(
        &mut self,
        cmd_index: u8,
        arg: u32,
        flags: u16,
        transfer_mode: u16,
    ) -> Result<(), Emmc2Error> {
        self.wait_cmd_ready()?;
        if transfer_mode != 0 {
            self.wait_dat_ready()?;
        }

        self.write16(NORMAL_INT_STATUS, 0xffff);
        self.write16(ERROR_INT_STATUS, 0xffff);
        self.write16(ARGUMENT, (arg >> 16) as u16);
        self.write16(ARGUMENT + 2, arg as u16);
        self.write16(TRANSFER_MODE, transfer_mode);
        let cmd_reg = (u16::from(cmd_index) << 8) | flags | CMD_CRC_EN | CMD_INDEX_EN;
        self.write16(COMMAND, cmd_reg);

        self.wait_int(INT_CMD_COMPLETE)?;
        if flags & (RESP_48 | RESP_48_BUSY | RESP_136) != 0 {
            // Response captured; nothing to return for now.
        }
        Ok(())
    }

    unsafe fn set_clock(&mut self, hz: u32) -> Result<(), Emmc2Error> {
        // SDHCI base clock on RPi4 EMMC is typically 50 MHz.
        const BASE_HZ: u32 = 50_000_000;
        let divisor = (BASE_HZ / hz).max(1);
        let sdclk = (divisor as u16) << 8;

        self.write16(CLOCK_CONTROL, 0);
        for _ in 0..10_000 {
            core::hint::spin_loop();
        }
        self.write16(CLOCK_CONTROL, sdclk);
        for _ in 0..10_000 {
            core::hint::spin_loop();
        }
        self.write16(CLOCK_CONTROL, sdclk | 0x0004); // internal clock enable
        for _ in 0..100_000 {
            if self.read16(CLOCK_CONTROL) & 0x0002 != 0 {
                break;
            }
            core::hint::spin_loop();
        }
        self.write16(CLOCK_CONTROL, sdclk | 0x0004 | 0x0001); // SD clock enable
        Ok(())
    }

    unsafe fn card_present(&self) -> bool {
        let state = self.read32(PRESENT_STATE);
        state & PRESENT_CARD_INSERTED != 0 && state & PRESENT_CARD_STABLE != 0
    }

    unsafe fn init_card(&mut self) -> Result<(), Emmc2Error> {
        if !self.card_present() {
            log::info!("emmc2: no card detected (proceeding with init anyway)");
        }

        self.write8(SOFTWARE_RESET, RESET_ALL);
        for _ in 0..100_000 {
            if self.read8(SOFTWARE_RESET) == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        self.write8(POWER_CONTROL, 0);
        self.write8(POWER_CONTROL, POWER_3_3V | POWER_ON);
        for _ in 0..10_000 {
            core::hint::spin_loop();
        }

        self.write16(HOST_CONTROL2, HC2_1V8_SIG);
        self.write16(NORMAL_INT_ENABLE, 0xffff);
        self.write16(NORMAL_INT_SIGNAL_ENABLE, 0xffff);
        self.write8(TIMEOUT_CONTROL, 0x0e);

        self.set_clock(400_000)?;

        // CMD0 — go idle
        self.send_cmd(0, 0, RESP_NONE, 0)?;
        for _ in 0..10_000 {
            core::hint::spin_loop();
        }

        // CMD8 — interface condition (SD v2+)
        let _ = self.send_cmd(8, 0x1aa, RESP_48, 0);

        // ACMD41 — wait until card ready
        for _ in 0..1000 {
            self.send_cmd(55, 0, RESP_48, 0)?;
            if self.send_cmd(41, 0x40ff8000, RESP_48, 0).is_ok() {
                let resp = self.read32(RESPONSE);
                if resp & (1 << 31) != 0 {
                    break;
                }
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }

        self.send_cmd(2, 0, RESP_136, 0)?;
        self.send_cmd(3, 0, RESP_48, 0)?;
        self.rca = (self.read32(RESPONSE) >> 16) as u16;
        let select_arg = u32::from(self.rca) << 16;
        self.send_cmd(7, select_arg, RESP_48_BUSY, 0)?;
        self.send_cmd(16, SECTOR_SIZE as u32, RESP_48, 0)?;

        self.set_clock(25_000_000)?;

        // CMD9 — CSD for capacity (SDHC uses sector addressing).
        self.send_cmd(9, select_arg, RESP_136, 0)?;
        let csd0 = self.read32(RESPONSE);
        let csd1 = self.read32(RESPONSE + 4);
        let csd2 = self.read32(RESPONSE + 8);
        let csd3 = self.read32(RESPONSE + 12);
        let csd = [csd0, csd1, csd2, csd3];

        let csd_structure = (csd[0] >> 30) & 0x3;
        self.num_blocks = if csd_structure == 1 {
            // CSD v2 (SDHC/SDXC): capacity = (C_SIZE+1) * 512KB / 512
            let c_size = ((csd[1] & 0x3f) << 16) | ((csd[2] >> 16) & 0xffff);
            (u64::from(c_size) + 1) * 1024
        } else {
            // Fallback for older cards or failed CSD parse.
            16 * 1024 * 2 // 16 MiB smoke default
        };

        self.initialized = true;
        log::info!(
            "emmc2: card ready rca={:#06x} blocks={}",
            self.rca,
            self.num_blocks
        );
        Ok(())
    }

    pub unsafe fn init(&mut self) -> Result<(), Emmc2Error> {
        self.init_card()
    }

    pub unsafe fn read_sector(
        &mut self,
        lba: u64,
        buf: &mut [u8; SECTOR_SIZE],
    ) -> Result<(), Emmc2Error> {
        if !self.initialized {
            return Err(Emmc2Error::IoError);
        }
        let arg = (u32::from(self.rca) << 16) | (lba as u32);
        self.write16(BLOCK_SIZE, SECTOR_SIZE as u16 | 0x7000);
        self.write16(BLOCK_COUNT, 1);
        self.send_cmd(
            17,
            arg,
            RESP_48 | CMD_DATA_PRESENT,
            TRANSFER_READ | TRANSFER_SINGLE,
        )?;
        self.wait_int(INT_BUFFER_READ_READY | INT_TRANSFER_COMPLETE)?;
        for i in 0..SECTOR_SIZE / 4 {
            let word = self.read32(BUFFER_DATA);
            let off = i * 4;
            buf[off] = (word >> 24) as u8;
            buf[off + 1] = (word >> 16) as u8;
            buf[off + 2] = (word >> 8) as u8;
            buf[off + 3] = word as u8;
        }
        Ok(())
    }

    pub unsafe fn write_sector(
        &mut self,
        lba: u64,
        buf: &[u8; SECTOR_SIZE],
    ) -> Result<(), Emmc2Error> {
        if !self.initialized {
            return Err(Emmc2Error::IoError);
        }
        let arg = (u32::from(self.rca) << 16) | (lba as u32);
        self.write16(BLOCK_SIZE, SECTOR_SIZE as u16 | 0x7000);
        self.write16(BLOCK_COUNT, 1);
        self.send_cmd(24, arg, RESP_48 | CMD_DATA_PRESENT, TRANSFER_SINGLE)?;
        self.wait_int(INT_BUFFER_WRITE_READY)?;
        for i in 0..SECTOR_SIZE / 4 {
            let off = i * 4;
            let word = u32::from(buf[off]) << 24
                | u32::from(buf[off + 1]) << 16
                | u32::from(buf[off + 2]) << 8
                | u32::from(buf[off + 3]);
            self.write32(BUFFER_DATA, word);
        }
        self.wait_int(INT_TRANSFER_COMPLETE)?;
        Ok(())
    }

    #[inline]
    unsafe fn write32(&mut self, off: usize, val: u32) {
        unsafe {
            ptr::write_volatile(self.base.add(off) as *mut u32, val);
        }
    }

    pub fn ack_interrupt(&mut self) {
        // Clear any pending SDHCI status on IRQ notify.
        unsafe {
            let status = self.read16(NORMAL_INT_STATUS);
            if status != 0 {
                self.write16(NORMAL_INT_STATUS, status);
            }
            let err = self.read16(ERROR_INT_STATUS);
            if err != 0 {
                self.write16(ERROR_INT_STATUS, err);
            }
        }
    }
}

impl GetBlockDeviceLayout for Emmc2 {
    type Error = Emmc2Error;

    fn get_block_size(&mut self) -> Result<usize, Self::Error> {
        Ok(SECTOR_SIZE)
    }

    fn get_num_blocks(&mut self) -> Result<u64, Self::Error> {
        if self.num_blocks == 0 {
            return Err(Emmc2Error::IoError);
        }
        Ok(self.num_blocks)
    }
}
