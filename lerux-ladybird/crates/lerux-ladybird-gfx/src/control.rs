//! Shared control page for `browser_ui` and `web_content`.
//!
//! The bitmap is a different region. This page carries a state, a title
//! length, and a title buffer. Microkit zeroes shared pages, and state 0 is
//! idle. The state is a `u32` so a value other than the named constants is
//! still a defined bit pattern.

use core::mem::{align_of, size_of};

/// The page is idle. A zeroed region starts here.
pub const CONTROL_STATE_IDLE: u32 = 0;
/// `WebContent` has painted the bitmap and written the title.
pub const CONTROL_STATE_PAINTED: u32 = 1;

/// Title bytes stored on the page. A longer title is truncated.
pub const TITLE_CAP: usize = 64;

/// One Microkit page. The struct sits at the start.
pub const CONTROL_REGION_SIZE: usize = 0x1000;

/// Virtual address of this page on `qemu_virt_aarch64_ladybird`.
///
/// `browser_ui` and `web_content` both map it. Microkit zeroes it.
pub const CONTROL_VADDR: u64 = 0x9_000_000;

/// Virtual address of the shared frame bitmap.
///
/// The region holds [`crate::FRAME_BYTES`] bytes, starting on the next page.
pub const BITMAP_VADDR: u64 = 0x9_001_000;

/// Paint notification on `web_content`. That domain has no other channel.
pub const WEB_CONTENT_PAINT_CHANNEL: u8 = 0;

/// Graphics virtualiser channel on `browser_ui`.
///
/// Serial transmit is 0 and serial receive is 1, so this channel is 2.
pub const BROWSER_UI_GPU_CHANNEL: u8 = 2;

/// Paint notification on `browser_ui`.
pub const BROWSER_UI_PAINT_CHANNEL: u8 = 3;

const _: () = assert!(CONTROL_REGION_SIZE == 0x1000);
const _: () = assert!(CONTROL_VADDR + 0x1000 == BITMAP_VADDR);

/// Control region. Fill with [`control_page_zeroed`], then assign fields.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlPage {
    pub state: u32,
    pub title_len: u32,
    pub title: [u8; TITLE_CAP],
}

const _: () = assert!(size_of::<ControlPage>() <= CONTROL_REGION_SIZE);
const _: () = assert!(align_of::<ControlPage>() <= 8);
const _: () = assert!(size_of::<ControlPage>() == size_of::<u32>() + size_of::<u32>() + TITLE_CAP);

/// A zeroed control page. Padding stays zero.
#[must_use]
pub const fn control_page_zeroed() -> ControlPage {
    // SAFETY: every field is an integer or a byte array, so the all-zero
    // pattern is a valid value. Zeroing also clears padding.
    unsafe { core::mem::zeroed() }
}

impl ControlPage {
    /// Copy `bytes` into the title and set [`Self::title_len`].
    ///
    /// Bytes past the stored length are cleared. A title longer than
    /// [`TITLE_CAP`] is truncated.
    pub fn set_title(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(TITLE_CAP);
        self.title[..n].copy_from_slice(&bytes[..n]);
        self.title[n..].fill(0);
        self.title_len = u32::try_from(n).unwrap_or(0);
    }

    /// The title bytes `title_len` names, clamped to the buffer.
    #[must_use]
    pub fn title(&self) -> &[u8] {
        let len = usize::try_from(self.title_len).unwrap_or(TITLE_CAP);
        let len = len.min(TITLE_CAP);
        &self.title[..len]
    }
}
