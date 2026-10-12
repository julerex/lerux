#![no_std]
#![no_main]

use core::arch::asm;

use sel4_microkit::{
    protection_domain, set_mr, Channel, ChannelSet, Handler, Infallible, MessageInfo,
};

use lerux_sddf::{ticks_to_ns, SDDF_TIMER_GET_TIME, SDDF_TIMER_SET_TIMEOUT};

const CLIENT: Channel = Channel::new(0);

struct HandlerImpl {
    freq: u64,
}

#[protection_domain]
fn init() -> HandlerImpl {
    HandlerImpl { freq: cntfrq() }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT {
            unreachable!("the timer client is channel 0");
        }
        if msg_info.label() == SDDF_TIMER_GET_TIME {
            set_mr(0, ticks_to_ns(cntpct(), self.freq));
            return Ok(MessageInfo::new(0, 1));
        }
        // A set-timeout has no deadline on this image, and an unknown label
        // gets the same empty reply.
        let _ = SDDF_TIMER_SET_TIMEOUT;
        Ok(MessageInfo::new(0, 0))
    }
}

fn cntpct() -> u64 {
    let value: u64;
    // SAFETY: seL4 exports the physical counter to userspace on this board.
    unsafe {
        asm!(
            "mrs {value}, cntpct_el0",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

fn cntfrq() -> u64 {
    let value: u64;
    // SAFETY: seL4 exports the counter frequency to userspace on this board.
    unsafe {
        asm!(
            "mrs {value}, cntfrq_el0",
            value = out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}
