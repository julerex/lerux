//! Shared control page for `browser_ui` and `web_content`.
//!
//! The bitmap is a different region. This page carries a state, a title
//! length, and a title buffer. Microkit zeroes shared pages, and state 0 is
//! idle. The state is a `u32` so a value other than the named constants is
//! still a defined bit pattern.

use core::mem::{align_of, size_of};

/// The page is idle. A zeroed region starts here.
pub const CONTROL_STATE_IDLE: u32 = 0;
/// WebContent has painted the bitmap and written the title.
pub const CONTROL_STATE_PAINTED: u32 = 1;

/// Title bytes stored on the page. A longer title is truncated.
pub const TITLE_CAP: usize = 64;

/// One Microkit page. The struct sits at the start.
pub const CONTROL_REGION_SIZE: usize = 0x1000;

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
    pub fn title(&self) -> &[u8] {
        let len = usize::try_from(self.title_len).unwrap_or(TITLE_CAP);
        let len = len.min(TITLE_CAP);
        &self.title[..len]
    }
}
