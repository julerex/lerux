//! Free-running timer from the x86 TSC (`rdtsc`).
//!
//! The kernel owns the PIT for boot calibration / APIC timing, so userspace
//! does not reprogram channel 0. QEMU TCG exposes a stable TSC; we assume
//! 1 GHz for duration conversion (good enough for uptime / smoke). Wire
//! protocol and dispatch live in `lerux_driver_protocols::timer`.

#![no_std]
#![no_main]

use core::{convert::Infallible, time::Duration};

use sel4_driver_interfaces::timer::{Clock, ErrorType, Timer as TimerTrait};
use sel4_microkit::{protection_domain, Channel};

use lerux_driver_protocols::timer::TimerHandler;

const CLIENT: Channel = Channel::new(1);

/// Assumed TSC frequency on QEMU q35 (TCG). Documented for Phase 56 smokes.
const TSC_HZ: u64 = 1_000_000_000;

struct Driver {
    start_tsc: u64,
}

impl Driver {
    fn new() -> Self {
        Self { start_tsc: rdtsc() }
    }
}

fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: RDTSC is unprivileged on QEMU PC; no memory access.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

impl ErrorType for Driver {
    type Error = Infallible;
}

impl Clock for Driver {
    fn get_time(&mut self) -> Result<Duration, Self::Error> {
        let now = rdtsc();
        let delta = now.saturating_sub(self.start_tsc);
        let nanos = (u128::from(delta) * 1_000_000_000) / u128::from(TSC_HZ);
        Ok(Duration::from_nanos(nanos as u64))
    }
}

impl TimerTrait for Driver {
    fn set_timeout(&mut self, _relative: Duration) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear_timeout(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[protection_domain]
fn init() -> TimerHandler<Driver> {
    TimerHandler::new(Driver::new(), CLIENT)
}
