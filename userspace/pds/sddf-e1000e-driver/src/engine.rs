//! Owns PCI claim, BAR0, the link, both rings, and the private DMA slots.
//!
//! The handler is the only caller. One Microkit thread runs this domain.

use lerux_machine::e1000e::E1000eMap;

pub(crate) const BUF_SIZE: usize = 2048;

/// Owns the 82574. The handler cannot name a descriptor slot.
pub struct Engine;

#[derive(Debug)]
pub struct OpenError;

impl Engine {
    /// Claim `8086:10d3` at the compiled BDF, program BAR0, reset, bring the
    /// link up, install `mac` into the receive address, and publish both rings.
    ///
    /// Call once. A second call would reset the NIC under live rings.
    pub fn open(map: E1000eMap, mac: [u8; 6]) -> Result<Self, OpenError> {
        let _ = (map, mac);
        unimplemented!("e1000e engine open")
    }

    /// Read the cause register. Idempotent. Does not touch the sDDF queues.
    pub fn ack(&self) {
        unimplemented!("e1000e engine ack")
    }

    /// Copy `frame` into a private transmit buffer and ring the NIC.
    ///
    /// Returns nothing. The caller always returns the shared descriptor.
    /// A full ring or a frame that does not fit one buffer is dropped here.
    pub fn submit_tx(&mut self, frame: &[u8]) {
        let _ = frame;
        unimplemented!("e1000e engine submit_tx")
    }

    /// Copy one completed Ethernet frame into `dst` and retire that buffer.
    ///
    /// `Some(len)` means `len > 0` and the bytes start at the destination MAC.
    /// `None` means no descriptor is waiting.
    pub fn copy_rx(&mut self, dst: &mut [u8]) -> Option<u16> {
        let _ = dst;
        unimplemented!("e1000e engine copy_rx")
    }
}
