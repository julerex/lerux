//! This image does not bring up a USB keyboard. The built-in keyboard is the i8042.

pub struct UsbKbd;

impl UsbKbd {
    pub fn off() -> Self {
        Self
    }

    pub fn ready() -> bool {
        false
    }

    pub fn poll(_buf: &mut [u8]) -> usize {
        0
    }

    pub fn ack_irq() {}
}
