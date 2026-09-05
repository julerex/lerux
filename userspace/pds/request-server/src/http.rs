//! HTTP/1.1 request builder. Response parsing lives in `lerux-interface-types`.

use alloc::vec::Vec;

use lerux_interface_types::{HttpMethod, HttpUrl, MAX_HTTP_HEADER_NAME, MAX_HTTP_HEADER_VALUE};

pub const MAX_EXTRA_HEADERS: usize = 4;

pub struct ExtraHeader {
    name: [u8; MAX_HTTP_HEADER_NAME],
    name_len: u8,
    value: [u8; MAX_HTTP_HEADER_VALUE],
    value_len: u8,
}

impl ExtraHeader {
    pub fn from_parts(name: &[u8], value: &[u8]) -> Self {
        let mut n = [0u8; MAX_HTTP_HEADER_NAME];
        let mut v = [0u8; MAX_HTTP_HEADER_VALUE];
        let name_len = name.len().min(MAX_HTTP_HEADER_NAME) as u8;
        let value_len = value.len().min(MAX_HTTP_HEADER_VALUE) as u8;
        n[..name_len as usize].copy_from_slice(&name[..name_len as usize]);
        v[..value_len as usize].copy_from_slice(&value[..value_len as usize]);
        Self {
            name: n,
            name_len,
            value: v,
            value_len,
        }
    }

    fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    fn value(&self) -> &[u8] {
        &self.value[..self.value_len as usize]
    }
}

pub fn build_request(method: HttpMethod, url: &HttpUrl, extra: &[Option<ExtraHeader>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(method.as_token());
    out.push(b' ');
    out.extend_from_slice(url.path());
    out.extend_from_slice(b" HTTP/1.1\r\nHost: ");
    out.extend_from_slice(url.host());
    out.extend_from_slice(b"\r\nConnection: close\r\n");
    for h in extra.iter().flatten() {
        out.extend_from_slice(h.name());
        out.extend_from_slice(b": ");
        out.extend_from_slice(h.value());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out
}
