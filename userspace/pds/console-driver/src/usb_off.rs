//! Swift X has no USB keyboard in this image. The built-in keyboard is PS/2.

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
