#![no_std]
#![no_main]

#[cfg(target_arch = "aarch64")]
use core::arch::asm;

use sel4_microkit::{
    protection_domain, set_mr, Channel, ChannelSet, Handler, Infallible, MessageInfo,
};

use lerux_sddf::{ticks_to_ns, SDDF_TIMER_GET_TIME, SDDF_TIMER_SET_TIMEOUT};

const CLIENT: Channel = Channel::new(0);

/// QEMU virt / seL4 timer frequency for qemu-riscv-virt.
#[cfg(target_arch = "riscv64")]
const TIMEBASE_HZ: u64 = 10_000_000;

#[cfg(target_arch = "aarch64")]
struct HandlerImpl {
    freq: u64,
}

#[cfg(target_arch = "riscv64")]
struct HandlerImpl {
    start_ticks: u64,
}

#[cfg(target_arch = "x86_64")]
struct HandlerImpl {
    start_tsc: u64,
}

/// Assumed TSC frequency on QEMU q35 (TCG).
#[cfg(target_arch = "x86_64")]
const TSC_HZ: u64 = 1_000_000_000;

#[protection_domain]
fn init() -> HandlerImpl {
    #[cfg(target_arch = "aarch64")]
    {
        HandlerImpl { freq: cntfrq() }
    }
    #[cfg(target_arch = "riscv64")]
    {
        HandlerImpl {
            start_ticks: read_time(),
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        HandlerImpl { start_tsc: rdtsc() }
    }
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
            set_mr(0, time_ns(self));
            return Ok(MessageInfo::new(0, 1));
        }
        // A set-timeout has no deadline on this image, and an unknown label
        // gets the same empty reply.
        let _ = SDDF_TIMER_SET_TIMEOUT;
        Ok(MessageInfo::new(0, 0))
    }
}

fn time_ns(handler: &HandlerImpl) -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        ticks_to_ns(cntpct(), handler.freq)
    }
    #[cfg(target_arch = "riscv64")]
    {
        ticks_to_ns(read_time().saturating_sub(handler.start_ticks), TIMEBASE_HZ)
    }
    #[cfg(target_arch = "x86_64")]
    {
        let delta = rdtsc().saturating_sub(handler.start_tsc);
        ticks_to_ns(delta, TSC_HZ)
    }
}

#[cfg(target_arch = "aarch64")]
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

#[cfg(target_arch = "aarch64")]
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

#[cfg(target_arch = "x86_64")]
fn rdtsc() -> u64 {
    // SAFETY: RDTSC is unprivileged on QEMU PC; no memory access.
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[cfg(target_arch = "riscv64")]
fn read_time() -> u64 {
    let mut ticks: u64;
    // SAFETY: `rdtime` is a valid S/U-mode control and status register read on QEMU virt.
    unsafe {
        core::arch::asm!("rdtime {}", out(reg) ticks, options(nostack, preserves_flags));
    }
    ticks
}
