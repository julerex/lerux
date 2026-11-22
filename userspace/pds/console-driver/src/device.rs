//! VGA text aperture and the i8042 keyboard controller.
//!
//! The system description maps `0xb8000` uncached and grants the CRTC ports
//! plus keyboard ports `0x60` and `0x64`. On the Swift image the same driver
//! paints the GOP panel when the mode record names that buffer, and the VGA
//! text page otherwise. Scan-code translation stays in [`crate::scancode`].

#[cfg(feature = "board-swift_sfx14_41g")]
use core::cell::Cell;

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{memory_region_symbol, var};

#[cfg(feature = "board-swift_sfx14_41g")]
use lerux_desk::{
    fill_background, paint_cell, paint_underline, panel_mode, Pen, BROWSER_HOLD, FRAMEBUFFER_BYTES,
};

use crate::screen::Screen;
#[cfg(feature = "board-swift_sfx14_41g")]
use crate::screen::CELLS;
#[cfg(feature = "board-swift_sfx14_41g")]
use crate::screen::COLS;

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
    cursor: Cell<Option<usize>>,
}

/// Microkit assigns `IOPort` caps from this `CNode` slot (same base as COM1).
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
    surface: Surface,
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

fn low_u16(value: usize) -> u16 {
    u16::try_from(value & 0xffff).expect("low half")
}

pub(crate) fn vga_text() -> *mut u16 {
    memory_region_symbol!(vga_text_vaddr: *mut u16).as_ptr()
}

/// The Swift image paints the panel or the VGA text page. Other boards are VGA.
#[cfg(feature = "board-swift_sfx14_41g")]
enum Surface {
    Panel(Panel),
    Vga {
        ptr: *mut u16,
        crtc_cap: u64,
        crtc_port: u16,
    },
}

impl Device {
    pub fn from_system() -> Self {
        let data_id = *var!(kbd_data_ioport_id: usize = usize::MAX) as u64;
        let data_port = low_u16(*var!(kbd_data_ioport_addr: usize = usize::MAX));
        let status_id = *var!(kbd_cmd_ioport_id: usize = usize::MAX) as u64;
        let status_port = low_u16(*var!(kbd_cmd_ioport_addr: usize = usize::MAX));
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        let vga = vga_text();
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        let crtc_id = *var!(crtc_ioport_id: usize = usize::MAX) as u64;
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        let crtc_port = low_u16(*var!(crtc_ioport_addr: usize = usize::MAX));
        #[cfg(feature = "board-swift_sfx14_41g")]
        let surface = surface_from_system();
        Self {
            #[cfg(not(feature = "board-swift_sfx14_41g"))]
            vga,
            #[cfg(feature = "board-swift_sfx14_41g")]
            surface,
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

    /// The firmware buffer is the Swift panel, so this boot paints pixels.
    #[cfg(feature = "board-swift_sfx14_41g")]
    pub fn uses_panel(&self) -> bool {
        matches!(self.surface, Surface::Panel(_))
    }

    pub fn present_all(&self, screen: &Screen) {
        for (index, &value) in screen.page().iter().enumerate() {
            self.write_cell(index, value);
        }
    }

    pub fn present_cells(&self, screen: &Screen, start: usize, end: usize) {
        let page = screen.page();
        let end = end.min(page.len());
        for (index, &value) in page.iter().enumerate().take(end).skip(start) {
            self.write_cell(index, value);
        }
    }

    pub(crate) fn enable_cursor(&self) {
        // Scanlines 13 through 15, disable-bit clear: an underline.
        // The Swift panel draws that underline in pixels instead.
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        {
            self.crtc(0x0a, 0x0d);
            self.crtc(0x0b, 0x0f);
        }
        #[cfg(feature = "board-swift_sfx14_41g")]
        if let Surface::Vga {
            crtc_cap,
            crtc_port,
            ..
        } = self.surface
        {
            Self::crtc_at(crtc_cap, crtc_port, 0x0a, 0x0d);
            Self::crtc_at(crtc_cap, crtc_port, 0x0b, 0x0f);
        }
    }

    pub(crate) fn sync_cursor(&self, screen: &Screen) {
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        {
            let pos = u16::try_from(screen.cursor_index()).expect("cursor fits");
            self.crtc(0x0e, (pos >> 8) as u8);
            self.crtc(0x0f, (pos & 0xff) as u8);
        }
        #[cfg(feature = "board-swift_sfx14_41g")]
        match &self.surface {
            Surface::Vga {
                crtc_cap,
                crtc_port,
                ..
            } => {
                let pos = u16::try_from(screen.cursor_index()).expect("cursor fits");
                Self::crtc_at(*crtc_cap, *crtc_port, 0x0e, (pos >> 8) as u8);
                Self::crtc_at(*crtc_cap, *crtc_port, 0x0f, (pos & 0xff) as u8);
            }
            Surface::Panel(panel) => {
                let next = screen.cursor_index();
                let page = screen.page();
                if let Some(prev) = panel.cursor.replace(Some(next))
                    && prev != next
                    && prev < page.len()
                {
                    self.write_cell(prev, page[prev]);
                }
                if next < page.len() {
                    self.write_cell(next, page[next]);
                    Self::underline(panel, next, page[next]);
                }
            }
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
        match &self.surface {
            Surface::Vga { ptr, .. } => unsafe {
                ptr.add(index).write_volatile(value);
            },
            Surface::Panel(panel) => write_panel_cell(panel, index, value),
        }
    }

    #[cfg(feature = "board-swift_sfx14_41g")]
    fn underline(panel: &Panel, index: usize, value: u16) {
        if !panel.live || held() || index >= CELLS {
            return;
        }
        // SAFETY: the same mapped framebuffer `write_cell` paints. The underline
        // covers the bottom two pixels of one cell.
        let frame = unsafe { core::slice::from_raw_parts_mut(panel.frame, panel.len) };
        paint_underline(
            frame,
            pen(panel),
            index % COLS,
            index / COLS,
            (value >> 8) as u8,
        );
    }

    fn crtc_at(cap: u64, port: u16, index: u8, value: u8) {
        Self::out8(cap, port, index);
        Self::out8(cap, port.wrapping_add(1), value);
    }

    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    fn crtc(&self, index: u8, value: u8) {
        Self::crtc_at(self.crtc_cap, self.crtc_port, index, value);
    }

    fn command(&self, byte: u8) -> bool {
        if !self.wait_ibf_clear() {
            return false;
        }
        Self::out8(self.status_cap, self.status_port, byte);
        true
    }

    fn write_data(&self, byte: u8) -> bool {
        if !self.wait_ibf_clear() {
            return false;
        }
        Self::out8(self.data_cap, self.data_port, byte);
        true
    }

    fn read_data(&self) -> Option<u8> {
        if self.status() & STATUS_OBF == 0 {
            return None;
        }
        Some(Self::in8(self.data_cap, self.data_port))
    }

    fn status(&self) -> u8 {
        Self::in8(self.status_cap, self.status_port)
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

    fn out8(cap: u64, port: u16, value: u8) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out8(cap, u64::from(port), u64::from(value));
        });
    }

    fn in8(cap: u64, port: u16) -> u8 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In8(cap, port);
            ret.result
        })
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
const _: () = assert!(crate::screen::ROWS == lerux_desk::CONSOLE_ROWS);

#[cfg(feature = "board-swift_sfx14_41g")]
fn framebuffer_bytes() -> usize {
    usize::try_from(FRAMEBUFFER_BYTES).expect("framebuffer fits")
}

/// True while the shell's browser owns the panel.
#[cfg(feature = "board-swift_sfx14_41g")]
fn held() -> bool {
    let hold = memory_region_symbol!(browser_hold_vaddr: *mut u8).as_ptr();
    unsafe { hold.read_volatile() == BROWSER_HOLD }
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn pen(panel: &Panel) -> Pen {
    Pen {
        pitch: panel.pitch,
        width: panel.width,
        height: panel.height,
        red: panel.red,
        green: panel.green,
        blue: panel.blue,
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn write_panel_cell(panel: &Panel, index: usize, value: u16) {
    if !panel.live || held() || index >= CELLS {
        return;
    }
    let frame = unsafe { core::slice::from_raw_parts_mut(panel.frame, panel.len) };
    paint_cell(
        frame,
        pen(panel),
        index % COLS,
        index / COLS,
        (value & 0xff) as u8,
        (value >> 8) as u8,
    );
}

/// Panel when the mode record names the Swift buffer. VGA text otherwise.
/// A VGA boot does not read or write the high framebuffer.
#[cfg(feature = "board-swift_sfx14_41g")]
fn surface_from_system() -> Surface {
    let mode_ptr = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode_ptr, 40) };
    if panel_mode(record).is_some() {
        return Surface::Panel(panel_from_record(record));
    }
    let crtc_id = *var!(crtc_ioport_id: usize = usize::MAX) as u64;
    let crtc_port = low_u16(*var!(crtc_ioport_addr: usize = usize::MAX));
    Surface::Vga {
        ptr: vga_text(),
        crtc_cap: BASE_IOPORT_CAP + crtc_id,
        crtc_port,
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn panel_from_record(record: &[u8]) -> Panel {
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
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
    if live && !held() {
        // SAFETY: Microkit mapped `framebuffer` read-write for FRAMEBUFFER_BYTES
        // bytes. This runs once during init, before the handler borrows the panel.
        let frame = unsafe { core::slice::from_raw_parts_mut(frame, framebuffer_bytes()) };
        let pen = Pen {
            pitch,
            width,
            height,
            red,
            green,
            blue,
        };
        fill_background(frame, pen);
    }
    Panel {
        frame,
        len: framebuffer_bytes(),
        pitch,
        width,
        height,
        red,
        green,
        blue,
        live,
        cursor: Cell::new(None),
    }
}
