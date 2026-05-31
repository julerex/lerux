use core::arch::asm;
use core::marker::PhantomData;

pub struct Pio<T> {
    port: u16,
    _t: PhantomData<T>,
}

impl Pio<u8> {
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            _t: PhantomData,
        }
    }
    pub fn read(&self) -> u8 {
        let value: u8;
        unsafe {
            asm!("in al, dx", out("al") value, in("dx") self.port, options(nomem, nostack));
        }
        value
    }
    pub fn write(&self, value: u8) {
        unsafe {
            asm!("out dx, al", in("dx") self.port, in("al") value, options(nomem, nostack));
        }
    }
}
