#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

// Channel 0: cothread_client (<end pd="cothread_peer" id="0" />).
const CLIENT: Channel = Channel::new(0);

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(CLIENT) {
            CLIENT.notify();
        }
        Ok(())
    }
}
