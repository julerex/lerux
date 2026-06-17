use crate::rmm::PhysicalAddress;
use alloc::boxed::Box;

pub trait Rxsdt {
    fn iter(&self) -> Box<dyn Iterator<Item = PhysicalAddress>>;
}
