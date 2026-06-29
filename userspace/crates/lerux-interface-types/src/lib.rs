//! Shared postcard RPC message types for lerux protection domains.

#![no_std]

use serde::{Deserialize, Serialize};

/// Maximum payload length for [`EchoRequest::Echo`] / [`EchoResponse::Echo`].
pub const MAX_ECHO_LEN: usize = 32;

/// Echo service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoRequest {
    Ping,
    Echo { len: u8, text: [u8; MAX_ECHO_LEN] },
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
    Echo { len: u8, text: [u8; MAX_ECHO_LEN] },
}

impl EchoResponse {
    pub fn as_echo_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Pong => None,
            Self::Echo { len, text } => Some(&text[..*len as usize]),
        }
    }
}

/// Sector size for [`BlockResponse::Sector`].
pub const SECTOR_SIZE: usize = 512;

/// Block service requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockRequest {
    ReadSector { lba: u32 },
    Poll,
}

/// Block service responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "Sector payload must hold one disk sector inline for IPC"
)]
pub enum BlockResponse {
    Pending,
    Sector {
        #[serde(with = "sector_bytes")]
        data: [u8; SECTOR_SIZE],
    },
    Error,
}

mod sector_bytes {
    use core::fmt;

    use serde::{
        de::{self, SeqAccess, Visitor},
        Deserializer, Serializer,
    };

    use super::SECTOR_SIZE;

    pub fn serialize<S: Serializer>(
        data: &[u8; SECTOR_SIZE],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(data)
    }

    struct SectorVisitor;

    impl<'de> Visitor<'de> for SectorVisitor {
        type Value = [u8; SECTOR_SIZE];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a byte array of sector size")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            if v.len() != SECTOR_SIZE {
                return Err(E::invalid_length(v.len(), &self));
            }
            let mut data = [0u8; SECTOR_SIZE];
            data.copy_from_slice(v);
            Ok(data)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut data = [0u8; SECTOR_SIZE];
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(i, &self))?;
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(de::Error::invalid_length(SECTOR_SIZE + 1, &self));
            }
            Ok(data)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; SECTOR_SIZE], D::Error> {
        deserializer.deserialize_bytes(SectorVisitor)
    }
}
