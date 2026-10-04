//! Network buffer descriptors and queues.
//!
//! Field order matches `include/sddf/network/queue.h`. `oid` is the low 6 bits
//! of its byte, which is the `uint8_t oid : 6` bit-field. Clients store 0.

use core::sync::atomic::AtomicU32;

pub const NET_BUFFER_SIZE: u32 = 2048;

/// One buffer descriptor. The trailing pad keeps the C size and alignment.
#[repr(C)]
pub struct net_buff_desc_t {
    pub io_or_offset: u64,
    pub len: u16,
    oid: u8,
    _pad: [u8; 5],
}

impl net_buff_desc_t {
    pub const fn new(io_or_offset: u64, len: u16, oid: u8) -> Self {
        Self {
            io_or_offset,
            len,
            oid: oid & 0x3f,
            _pad: [0; 5],
        }
    }

    /// Ownership identifier. Only the low 6 bits are defined.
    pub const fn oid(self) -> u8 {
        self.oid & 0x3f
    }
}

/// Fixed prefix of `net_queue_t`. Descriptors follow this prefix in memory.
///
/// The C type ends in a flexible array of `net_buff_desc_t`. That array is not
/// part of the prefix, but it raises the alignment to 8. `tail` and `head` are
/// adjacent `uint16_t` fields.
#[repr(C, align(8))]
pub struct net_queue_t {
    pub tail: u16,
    pub head: u16,
    pub consumer_signalled: AtomicU32,
}

#[repr(C)]
pub struct net_queue_handle_t {
    pub free: *mut net_queue_t,
    pub active: *mut net_queue_t,
    pub capacity: u32,
}
