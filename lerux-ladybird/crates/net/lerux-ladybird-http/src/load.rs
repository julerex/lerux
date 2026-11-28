//! One GET built from an `https` URL whose host is [`TLS_NAME`].
//!
//! This milestone does not resolve a public name. A host other than
//! [`TLS_NAME`] is refused.

use alloc::string::{String, ToString};

use lerux_ladybird_url::Url;

/// Transport Layer Security name for the smoke helper.
pub const TLS_NAME: &str = "host";

const GET_PREFIX: &[u8] = b"GET ";
const GET_TAIL: &[u8] = b" HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n";

/// The URL cannot be loaded by this request server.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestError;

/// Path, query, and port of one GET.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Load {
    port: u16,
    target: String,
}

impl Load {
    /// Parse `input` as one GET for [`TLS_NAME`].
    ///
    /// # Errors
    ///
    /// Returns [`RequestError`] when `input` is not an `https` URL, the host
    /// is not [`TLS_NAME`], or the request target contains a space, a carriage
    /// return, or a line feed.
    pub fn parse(input: &str) -> Result<Self, RequestError> {
        let url = Url::parse(input, None).map_err(|_| RequestError)?;
        if url.host() != TLS_NAME {
            return Err(RequestError);
        }
        let target = url.request_target().to_string();
        if !target_ok(target.as_bytes()) {
            return Err(RequestError);
        }
        Ok(Self {
            port: url.port(),
            target,
        })
    }

    /// Port stored on the URL, including an explicit `:8443`.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Path and query. The `?` is included when a query is present.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Write `GET <target> HTTP/1.1` with `Host: host` and `Connection: close`.
    ///
    /// # Errors
    ///
    /// Returns [`RequestError`] when `out` is shorter than the request.
    pub fn write_get(&self, out: &mut [u8]) -> Result<usize, RequestError> {
        let target = self.target.as_bytes();
        let len = GET_PREFIX
            .len()
            .checked_add(target.len())
            .and_then(|sum| sum.checked_add(GET_TAIL.len()))
            .ok_or(RequestError)?;
        if len > out.len() {
            return Err(RequestError);
        }
        out[..GET_PREFIX.len()].copy_from_slice(GET_PREFIX);
        let target_end = GET_PREFIX.len() + target.len();
        out[GET_PREFIX.len()..target_end].copy_from_slice(target);
        out[target_end..len].copy_from_slice(GET_TAIL);
        Ok(len)
    }
}

fn target_ok(target: &[u8]) -> bool {
    !target.is_empty()
        && target
            .iter()
            .all(|byte| *byte != b' ' && *byte != b'\r' && *byte != b'\n')
}
