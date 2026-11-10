//! Stand-in for Ladybird `Libraries/LibHTTP` and `Libraries/LibRequests`.
//!
//! `web_content` writes one absolute `https` URL into the shared request page
//! and waits. `request_server` performs one GET of that URL's path and query,
//! writes the body into a shared region of 64 KiB, sets the length and a
//! status, and notifies back. A body that does not fit sets an error status
//! and a length of 0. This crate does not resolve a name, follow a redirect,
//! store a cookie, or start a second request while one is in flight.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod absolute;
mod page;

pub use absolute::HTTPS_PORT;
pub use page::{
    request_page_zeroed, RequestBody, RequestPage, SubmitError, REQUEST_BODY_SIZE,
    REQUEST_REGION_SIZE, REQUEST_STATUS_ERR, REQUEST_STATUS_IDLE, REQUEST_STATUS_OK,
    REQUEST_STATUS_PENDING, REQUEST_URL_CAP,
};
