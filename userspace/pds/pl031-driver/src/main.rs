#![no_std]
#![no_main]

use sel4_microkit::{memory_region_symbol, protection_domain, Channel, Handler};
use sel4_microkit_driver_adapters::rtc::driver::HandlerImpl;
use sel4_pl031_driver::Driver;

const CLIENT: Channel = Channel::new(1);

#[protection_domain]
fn init() -> impl Handler {
    let driver = unsafe { Driver::new(memory_region_symbol!(pl031_mmio_vaddr: *mut ()).as_ptr()) };
    HandlerImpl::new(driver, CLIENT)
}