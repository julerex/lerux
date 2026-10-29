//! VGA text aperture and the i8042 keyboard controller.
//!
//! The system description maps `0xb8000` uncached and grants the CRTC ports
//! plus keyboard ports `0x60` and `0x64`. Scan-code translation stays in
//! [`crate::scancode`].

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{memory_region_symbol, var};

#[cfg(feature = "board-swift_sfx14_41g")]
use lerux_desk::{fill_background, paint_cell, panel_mode, Pen, CELL_PX, FRAMEBUFFER_BYTES};

use crate::screen::{Screen, CELLS};
#[cfg(feature = "board-swift_sfx14_41g")]
use crate::screen::{COLS, ROWS};

#[cfg(feature = "board-swift_sfx14_41g")]
struct Panel {
    frame: *mut u8,
    len: usize,
    pitch: usize,
    width: usize,
    height: usize,
    red: u8,
    green: u8,
    blue: u8,
    live: bool,
}

/// Microkit assigns IOPort caps from this CNode slot (same base as COM1).
const BASE_IOPORT_CAP: u64 = 394;

const STATUS_OBF: u8 = 0x01;
const STATUS_IBF: u8 = 0x02;

const CMD_DISABLE_KBD: u8 = 0xad;
const CMD_DISABLE_AUX: u8 = 0xa7;
const CMD_READ_CONFIG: u8 = 0x20;
const CMD_WRITE_CONFIG: u8 = 0x60;
const CMD_ENABLE_KBD: u8 = 0xae;
const DEV_ENABLE_SCAN: u8 = 0xf4;

/// IRQ1, system flag, translation, mouse clock held off.
const CONFIG_FALLBACK: u8 = 0x65;

const SPIN: u32 = 100_000;

/// CRTC index/data and the two i8042 ports. Each range has its own cap.
pub struct Device {
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    vga: *mut u16,
    #[cfg(feature = "board-swift_sfx14_41g")]
    panel: Panel,
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    crtc_cap: u64,
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    crtc_port: u16,
    data_cap: u64,
    data_port: u16,
    status_cap: u64,
    status_port: u16,
}

unsafe impl Send for Device {}

impl Device {
    pub fn from_system() -> Self {
        let data_id = *var!(kbd_data_ioport_id: usize = usize::MAX) as u64;
        let data_port = *var!(kbd_data_ioport_addr: usize = usize::MAX) as u16;
        let status_id = *var!(kbd_cmd_ioport_id: usize = usize::MAX) as u64;
        let status_port = *var!(kbd_cmd_ioport_addr: usize = usize::MAX) as u16;
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        let vga = memory_region_symbol!(vga_text_vaddr: *mut u16).as_ptr();
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        let crtc_id = *var!(crtc_ioport_id: usize = usize::MAX) as u64;
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        let crtc_port = *var!(crtc_ioport_addr: usize = usize::MAX) as u16;
        #[cfg(feature = "board-swift_sfx14_41g")]
        let panel = panel_from_system();
        Self {
            #[cfg(not(feature = "board-swift_sfx14_41g"))]
            vga,
            #[cfg(feature = "board-swift_sfx14_41g")]
            panel,
            #[cfg(not(feature = "board-swift_sfx14_41g"))]
            crtc_cap: BASE_IOPORT_CAP + crtc_id,
            #[cfg(not(feature = "board-swift_sfx14_41g"))]
            crtc_port,
            data_cap: BASE_IOPORT_CAP + data_id,
            data_port,
            status_cap: BASE_IOPORT_CAP + status_id,
            status_port,
        }
    }

    pub fn present_all(&self, screen: &Screen) {
        for (index, &value) in screen.cells().iter().enumerate() {
            self.write_cell(index, value);
        }
    }

    pub fn present_cells(&self, screen: &Screen, start: usize, end: usize) {
        for index in start..end.min(CELLS) {
            self.write_cell(index, screen.cells()[index]);
        }
    }

    pub fn enable_cursor(&self) {
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        {
            // Start line 0, end line 15, disable-bit clear: a block cursor.
            self.crtc(0x0a, 0x00);
            self.crtc(0x0b, 0x0f);
        }
    }

    pub fn sync_cursor(&self, screen: &Screen) {
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        {
            let pos = screen.cursor_index() as u16;
            self.crtc(0x0e, (pos >> 8) as u8);
            self.crtc(0x0f, (pos & 0xff) as u8);
        }
        #[cfg(feature = "board-swift_sfx14_41g")]
        {
            let _ = screen;
        }
    }

    /// Enable scan set 1 and IRQ1. A missing controller still leaves the banner up.
    pub fn init_keyboard(&self) -> bool {
        let _ = self.command(CMD_DISABLE_KBD);
        let _ = self.command(CMD_DISABLE_AUX);
        for _ in 0..16 {
            if self.read_data().is_none() {
                break;
            }
        }

        let config = if self.command(CMD_READ_CONFIG) && self.wait_obf() {
            let mut config = self.read_data().unwrap_or(CONFIG_FALLBACK);
            config |= 0x01 | 0x40;
            config &= !0x12;
            config |= 0x20;
            config
        } else {
            CONFIG_FALLBACK
        };
        let wrote = self.command(CMD_WRITE_CONFIG) && self.write_data(config);
        let enabled = self.command(CMD_ENABLE_KBD) && self.write_data(DEV_ENABLE_SCAN);
        if self.wait_obf() {
            let _ = self.read_data();
        }
        wrote && enabled
    }

    /// Next set-1 byte, if the output buffer is full.
    pub fn read_scancode(&self) -> Option<u8> {
        self.read_data()
    }

    fn write_cell(&self, index: usize, value: u16) {
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        unsafe {
            self.vga.add(index).write_volatile(value);
        }
        #[cfg(feature = "board-swift_sfx14_41g")]
        {
            if !self.panel.live || index >= CELLS {
                return;
            }
            let frame =
                unsafe { core::slice::from_raw_parts_mut(self.panel.frame, self.panel.len) };
            paint_cell(
                frame,
                Pen {
                    pitch: self.panel.pitch,
                    width: self.panel.width,
                    height: self.panel.height,
                    red: self.panel.red,
                    green: self.panel.green,
                    blue: self.panel.blue,
                },
                index % COLS,
                index / COLS,
                (value & 0xff) as u8,
            );
        }
    }

    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    fn crtc(&self, index: u8, value: u8) {
        self.out8(self.crtc_cap, self.crtc_port, index);
        self.out8(self.crtc_cap, self.crtc_port.wrapping_add(1), value);
    }

    fn command(&self, byte: u8) -> bool {
        if !self.wait_ibf_clear() {
            return false;
        }
        self.out8(self.status_cap, self.status_port, byte);
        true
    }

    fn write_data(&self, byte: u8) -> bool {
        if !self.wait_ibf_clear() {
            return false;
        }
        self.out8(self.data_cap, self.data_port, byte);
        true
    }

    fn read_data(&self) -> Option<u8> {
        if self.status() & STATUS_OBF == 0 {
            return None;
        }
        Some(self.in8(self.data_cap, self.data_port))
    }

    fn status(&self) -> u8 {
        self.in8(self.status_cap, self.status_port)
    }

    fn wait_ibf_clear(&self) -> bool {
        for _ in 0..SPIN {
            if self.status() & STATUS_IBF == 0 {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn wait_obf(&self) -> bool {
        for _ in 0..SPIN {
            if self.status() & STATUS_OBF != 0 {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn out8(&self, cap: u64, port: u16, value: u8) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out8(cap, u64::from(port), u64::from(value));
        });
    }

    fn in8(&self, cap: u64, port: u16) -> u8 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In8(cap, port);
            ret.result
        })
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
const _: () = assert!(crate::screen::ROWS == lerux_desk::CONSOLE_ROWS);

#[cfg(feature = "board-swift_sfx14_41g")]
fn panel_from_system() -> Panel {
    let mode_ptr = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode_ptr, 40) };
    let mode = panel_mode(record);
    let live = mode.is_some();
    let (pitch, width, height, red, green, blue) = if let Some(mode) = mode {
        (
            mode.pitch() as usize,
            mode.width() as usize,
            mode.height() as usize,
            mode.red_shift(),
            mode.green_shift(),
            mode.blue_shift(),
        )
    } else {
        (0, 0, 0, 0, 0, 0)
    };
    if live {
        // SAFETY: Microkit mapped `framebuffer` read-write for FRAMEBUFFER_BYTES
        // bytes. This runs once during init, before the handler borrows the panel.
        let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
        let pen = Pen {
            pitch,
            width,
            height,
            red,
            green,
            blue,
        };
        let mut console = pen;
        console.height = ROWS * CELL_PX;
        fill_background(frame, console);
    }
    Panel {
        frame,
        len: FRAMEBUFFER_BYTES as usize,
        pitch,
        width,
        height,
        red,
        green,
        blue,
        live,
    }
}
