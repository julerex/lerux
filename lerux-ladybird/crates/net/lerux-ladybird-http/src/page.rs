//! One request slot shared by `web_content` and `request_server`.
//!
//! The body is a different region. This page carries the state, the URL, and
//! the body length. Microkit zeroes shared pages, and state 0 is idle.

use core::mem::{align_of, size_of};

/// The page is idle. A zeroed region starts here.
pub const REQUEST_STATE_IDLE: u32 = 0;
/// `web_content` has written the URL and is waiting.
pub const REQUEST_STATE_ASK: u32 = 1;
/// The body region holds `body_len` bytes of the response.
pub const REQUEST_STATE_OK: u32 = 2;
/// The load failed. `body_len` is 0.
pub const REQUEST_STATE_ERR: u32 = 3;

/// URL bytes stored on the page. A longer URL is refused.
pub const URL_CAP: usize = 128;

/// Body region. A longer body is an error and `body_len` stays 0.
pub const BODY_CAP: usize = 64 * 1024;

/// One Microkit page. The struct sits at the start.
pub const REQUEST_REGION_SIZE: usize = 0x1000;

/// System-description name of the request page.
pub const REQUEST_REGION: &str = "ladybird_request";

/// System-description name of the body region.
pub const BODY_REGION: &str = "ladybird_body";

/// Virtual address of the request page on `qemu_virt_aarch64_ladybird`.
pub const REQUEST_VADDR: u64 = 0x9_020_000;

/// Virtual address of the shared body region. It follows the request page.
pub const BODY_VADDR: u64 = 0x9_021_000;

/// Request notification on `web_content`. Paint stays channel 0, so this is 1.
pub const WEB_CONTENT_REQUEST_CHANNEL: u8 = 1;

/// Request notification on `request_server`.
pub const REQUEST_SERVER_CHANNEL: u8 = 0;

/// Network receive channel on `request_server`.
pub const REQUEST_NET_RX_CHANNEL: u8 = 1;

/// Network transmit channel on `request_server`.
pub const REQUEST_NET_TX_CHANNEL: u8 = 2;

const _: () = assert!(REQUEST_REGION_SIZE == 0x1000);
const _: () = assert!(BODY_CAP == 0x1_0000);
const _: () = assert!(BODY_VADDR == REQUEST_VADDR + REQUEST_REGION_SIZE as u64);
const _: () = assert!(size_of::<RequestPage>() <= REQUEST_REGION_SIZE);
const _: () = assert!(align_of::<RequestPage>() <= 8);

/// Request region. Fill with [`request_page_zeroed`], then assign fields.
///
/// `state` is stored by the caller after a release fence, so the URL or the
/// body length is visible before the state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestPage {
    pub state: u32,
    pub url_len: u32,
    pub body_len: u32,
    pub url: [u8; URL_CAP],
}

const _: () = assert!(
    size_of::<RequestPage>() == size_of::<u32>() + size_of::<u32>() + size_of::<u32>() + URL_CAP
);

/// A zeroed request page. Padding stays zero.
#[must_use]
pub const fn request_page_zeroed() -> RequestPage {
    // SAFETY: every field is an integer or a byte array, so the all-zero
    // pattern is a valid value. Zeroing also clears padding.
    unsafe { core::mem::zeroed() }
}

impl RequestPage {
    /// Copy `url` into the page and clear `body_len`.
    ///
    /// An empty URL or a URL longer than [`URL_CAP`] is refused and the page
    /// is left unchanged. This does not store [`REQUEST_STATE_ASK`].
    #[must_use]
    pub fn set_url(&mut self, url: &[u8]) -> bool {
        if url.is_empty() || url.len() > URL_CAP {
            return false;
        }
        let n = url.len();
        self.url[..n].copy_from_slice(url);
        self.url[n..].fill(0);
        self.url_len = u32::try_from(n).unwrap_or(0);
        self.body_len = 0;
        true
    }

    /// The URL bytes `url_len` names, clamped to the buffer.
    #[must_use]
    pub fn url(&self) -> &[u8] {
        let len = usize::try_from(self.url_len).unwrap_or(URL_CAP);
        &self.url[..len.min(URL_CAP)]
    }

    /// Record a body that fits in [`BODY_CAP`].
    ///
    /// A longer body sets `body_len` to 0 and returns false. This does not
    /// store [`REQUEST_STATE_OK`].
    #[must_use]
    pub fn accept_body(&mut self, len: usize) -> bool {
        let Ok(stored) = u32::try_from(len) else {
            self.body_len = 0;
            return false;
        };
        if len > BODY_CAP {
            self.body_len = 0;
            return false;
        }
        self.body_len = stored;
        true
    }

    /// Clear `body_len` after a failed load.
    pub fn clear_body(&mut self) {
        self.body_len = 0;
    }

    /// Body length clamped to [`BODY_CAP`].
    #[must_use]
    pub fn body_len(&self) -> usize {
        usize::try_from(self.body_len).unwrap_or(0).min(BODY_CAP)
    }
}
