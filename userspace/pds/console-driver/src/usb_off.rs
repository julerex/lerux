//! Swift X has no USB keyboard in this image. The built-in keyboard is PS/2.

pub struct UsbKbd;

impl UsbKbd {
    pub fn off() -> Self {
        Self
    }

    pub fn ready(&self) -> bool {
        false
    }

    pub fn poll(&mut self, _buf: &mut [u8]) -> usize {
        0
    }

    pub fn ack_irq(&mut self) {}
}
