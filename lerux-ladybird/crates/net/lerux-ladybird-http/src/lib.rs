//! Shared request page for `web_content` and `request_server`.
//!
//! `web_content` writes one URL and waits. `request_server` performs one GET
//! and writes the body into a separate region. A body longer than [`BODY_CAP`]
//! is an error with length 0. This image does not resolve names, follow a
//! redirect, or keep a second request in flight.

#![no_std]
extern crate alloc;

mod load;
mod page;
mod response;

pub use load::{Load, RequestError, TLS_NAME};
pub use page::{
    request_page_zeroed, RequestPage, BODY_CAP, BODY_REGION, BODY_VADDR, REQUEST_NET_RX_CHANNEL,
    REQUEST_NET_TX_CHANNEL, REQUEST_REGION, REQUEST_REGION_SIZE, REQUEST_SERVER_CHANNEL,
    REQUEST_STATE_ASK, REQUEST_STATE_ERR, REQUEST_STATE_IDLE, REQUEST_STATE_OK, REQUEST_VADDR,
    URL_CAP, WEB_CONTENT_REQUEST_CHANNEL,
};
pub use response::{response_body, ResponseBody, RESPONSE_SLACK};
