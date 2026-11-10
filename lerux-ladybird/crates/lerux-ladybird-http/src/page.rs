//! Shared request header and the 64 KiB body region.
//!
//! `web_content` and `request_server` both map these bytes. The fields are
//! plain integers and arrays so the layout is `repr(C)`. [`request_page_zeroed`]
//! sets every field to zero, then the caller assigns the URL and the status.
//! The header's size is the sum of its fields, so that zero value has no
//! padding byte left unset.

use core::mem::{align_of, size_of};

use crate::absolute;

/// Header region. One Microkit page. The body is [`RequestBody`].
pub const REQUEST_REGION_SIZE: usize = 0x1000;

/// Body region. A longer response does not fit.
pub const REQUEST_BODY_SIZE: usize = 64 * 1024;

/// URL bytes stored on the header. A longer URL is rejected.
pub const REQUEST_URL_CAP: usize = 1024;

/// The page is idle. A zeroed header starts here.
pub const REQUEST_STATUS_IDLE: u32 = 0;
/// One GET is in flight. A second submit is rejected until it finishes.
pub const REQUEST_STATUS_PENDING: u32 = 1;
/// The body fits in [`REQUEST_BODY_SIZE`]. [`RequestPage::body_len`] is the length.
pub const REQUEST_STATUS_OK: u32 = 2;
/// The body does not fit, or the GET failed. [`RequestPage::body_len`] is 0.
pub const REQUEST_STATUS_ERR: u32 = 3;

/// Header shared by `web_content` and `request_server`.
///
/// The body lives in [`RequestBody`], not in this struct. This header is small
/// enough to return by value. The body is 64 KiB, which is the whole
/// protection-domain stack, so that region is cleared in place.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestPage {
    pub status: u32,
    pub url_len: u32,
    pub body_len: u32,
    pub url: [u8; REQUEST_URL_CAP],
}

/// Shared response body. 64 KiB, `repr(C)`, separate from the header page.
#[repr(C)]
pub struct RequestBody {
    pub bytes: [u8; REQUEST_BODY_SIZE],
}

const _: () = assert!(size_of::<RequestPage>() <= REQUEST_REGION_SIZE);
const _: () = assert!(align_of::<RequestPage>() <= 8);
const _: () = assert!(size_of::<RequestPage>() == 4 + 4 + 4 + REQUEST_URL_CAP);
const _: () = assert!(size_of::<RequestBody>() == REQUEST_BODY_SIZE);
const _: () = assert!(align_of::<RequestBody>() <= 8);
const _: () = assert!(REQUEST_BODY_SIZE == 0x1_0000);
const _: () = assert!(REQUEST_BODY_SIZE.is_multiple_of(0x1000));
const _: () = assert!(REQUEST_URL_CAP.is_multiple_of(4));
const _: () = assert!(REQUEST_BODY_SIZE <= 0xFFFF_FFFF);

/// Why [`RequestPage::submit`] refused the URL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitError {
    /// A request is already in flight.
    Busy,
    /// `url` is not one absolute `https` URL that fits on the page.
    Url,
}

impl core::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::Busy => "request in flight",
            Self::Url => "invalid url",
        };
        f.write_str(text)
    }
}

impl core::error::Error for SubmitError {}

/// A header with every field set to zero.
///
/// The size of [`RequestPage`] is the sum of its fields, so this value has
/// no padding. Callers assign the URL and the status after this returns.
#[must_use]
pub const fn request_page_zeroed() -> RequestPage {
    RequestPage {
        status: REQUEST_STATUS_IDLE,
        url_len: 0,
        body_len: 0,
        url: [0; REQUEST_URL_CAP],
    }
}

impl RequestBody {
    /// Set every byte to zero. The mapped region is already zero after boot.
    pub fn clear(&mut self) {
        self.bytes.fill(0);
    }
}

impl RequestPage {
    /// Copy an absolute `https` URL onto the page and mark one request in flight.
    ///
    /// The stored bytes are the URL as given. [`Self::request_target`] is the
    /// path and the query. There is no cookie field and no second slot.
    ///
    /// # Errors
    ///
    /// [`SubmitError::Busy`] when a request is already in flight, or the status
    /// is not idle, ok, or err.
    /// [`SubmitError::Url`] when `url` is empty, not absolute `https`, contains
    /// whitespace, or does not fit in [`REQUEST_URL_CAP`].
    pub fn submit(&mut self, url: &str) -> Result<(), SubmitError> {
        if !reusable(self.status) {
            return Err(SubmitError::Busy);
        }
        if url.len() > REQUEST_URL_CAP || absolute::parse(url).is_none() {
            return Err(SubmitError::Url);
        }
        self.url[..url.len()].copy_from_slice(url.as_bytes());
        self.url[url.len()..].fill(0);
        self.url_len = u32_len(url.len());
        self.body_len = 0;
        self.status = REQUEST_STATUS_PENDING;
        Ok(())
    }

    /// URL bytes `url_len` names, when they are UTF-8 inside the buffer.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        stored_url(self)
    }

    /// Host as stored. This crate does not look the name up.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        Some(self.view()?.host)
    }

    /// TCP port. An omitted port is [`crate::HTTPS_PORT`].
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        Some(self.view()?.port)
    }

    /// Path and query. The fragment is omitted. An empty path is `/`.
    #[must_use]
    pub fn request_target(&self) -> Option<&str> {
        Some(self.view()?.target)
    }

    /// Body bytes when the status is [`REQUEST_STATUS_OK`].
    ///
    /// `region` is the shared body. A short slice yields `None`.
    #[must_use]
    pub fn body<'region>(&self, region: &'region [u8]) -> Option<&'region [u8]> {
        if self.status != REQUEST_STATUS_OK {
            return None;
        }
        let len = usize::try_from(self.body_len).ok()?;
        region.get(..len)
    }

    /// Record the GET body the way `request_server` would.
    ///
    /// `region` is the shared body and must be [`REQUEST_BODY_SIZE`] bytes.
    /// Returns false when no request is in flight, or `region` is the wrong
    /// length, and leaves both unchanged.
    /// A body that fits sets [`REQUEST_STATUS_OK`] and [`Self::body_len`].
    /// A longer body sets [`REQUEST_STATUS_ERR`], a length of 0, and clears
    /// `region`. It does not copy a prefix of that body.
    #[must_use]
    pub fn write_body(&mut self, region: &mut [u8], body: &[u8]) -> bool {
        if self.status != REQUEST_STATUS_PENDING || region.len() != REQUEST_BODY_SIZE {
            return false;
        }
        if body.len() > REQUEST_BODY_SIZE {
            self.reject_body(region);
            return true;
        }
        self.store_fitted(region, body);
        true
    }

    fn view(&self) -> Option<absolute::Https<'_>> {
        absolute::parse(stored_url(self)?)
    }

    fn reject_body(&mut self, region: &mut [u8]) {
        // Clear before publishing the error. A prefix of `body` is never copied.
        region.fill(0);
        self.body_len = 0;
        self.status = REQUEST_STATUS_ERR;
    }

    fn store_fitted(&mut self, region: &mut [u8], body: &[u8]) {
        let len = body.len();
        region[..len].copy_from_slice(body);
        region[len..].fill(0);
        self.body_len = u32_len(len);
        self.status = REQUEST_STATUS_OK;
    }
}

fn reusable(status: u32) -> bool {
    matches!(
        status,
        REQUEST_STATUS_IDLE | REQUEST_STATUS_OK | REQUEST_STATUS_ERR
    )
}

fn stored_url(page: &RequestPage) -> Option<&str> {
    let len = usize::try_from(page.url_len).ok()?;
    let bytes = page.url.get(..len)?;
    core::str::from_utf8(bytes).ok()
}

fn u32_len(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(0)
}
