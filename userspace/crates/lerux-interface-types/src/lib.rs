//! Shared postcard RPC message types for lerux protection domains.

#![no_std]

use serde::{Deserialize, Serialize};

/// Maximum payload length for [`EchoRequest::Echo`] / [`EchoResponse::Echo`].
pub const MAX_ECHO_LEN: usize = 32;

/// Echo service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoRequest {
    Ping,
    Echo {
        len: u8,
        text: [u8; MAX_ECHO_LEN],
    },
}

impl EchoRequest {
    pub fn echo(text: &[u8]) -> Self {
        let mut buf = [0u8; MAX_ECHO_LEN];
        let len = text.len().min(MAX_ECHO_LEN) as u8;
        buf[..len as usize].copy_from_slice(&text[..len as usize]);
        Self::Echo { len, text: buf }
    }
}

/// Echo service responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoResponse {
    Pong,
    Echo {
        len: u8,
        text: [u8; MAX_ECHO_LEN],
    },
}

impl EchoResponse {
    pub fn as_echo_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Pong => None,
            Self::Echo { len, text } => Some(&text[..*len as usize]),
        }
    }
}