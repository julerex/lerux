//! Free-running timer from the x86 TSC (`rdtsc`).
//!
//! The kernel owns the PIT for boot calibration / APIC timing, so userspace
//! does not reprogram channel 0. QEMU TCG exposes a stable TSC; we assume
//! 1 GHz for duration conversion (good enough for uptime / smoke). Wire
//! format matches `sel4-microkit-driver-adapters` timer messages.

#![no_std]
#![no_main]

use core::{convert::Infallible, time::Duration};

use sel4_driver_interfaces::timer::{Clock, ErrorType, Timer as TimerTrait};
use sel4_microkit::{protection_domain, Channel, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;
use serde::{Deserialize, Serialize};

const CLIENT: Channel = Channel::new(1);

/// Assumed TSC frequency on QEMU q35 (TCG). Documented for Phase 56 smokes.
const TSC_HZ: u64 = 1_000_000_000;

// Mirror sel4_microkit_driver_adapters::timer::message_types (private there).
#[derive(Debug, Serialize, Deserialize)]
enum Request {
    GetTime,
    NumTimers,
    SetTimeout { timer: usize, relative: Duration },
    ClearTimeout { timer: usize },
}

type Response = Result<SuccessResponse, ErrorResponse>;

#[derive(Debug, Serialize, Deserialize)]
enum SuccessResponse {
    GetTime(Duration),
    NumTimers(usize),
    SetTimeout,
    ClearTimeout,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
enum ErrorResponse {
    TimerOutOfBounds,
    Unspecified,
}

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

struct HandlerImpl {
    driver: Driver,
    client: Channel,
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != self.client {
            return Ok(simple_ipc::send_unspecified_error());
        }
        Ok(match simple_ipc::recv::<Request>(msg_info) {
            Ok(req) => {
                let resp: Response = match req {
                    Request::GetTime => self
                        .driver
                        .get_time()
                        .map(SuccessResponse::GetTime)
                        .map_err(|_| ErrorResponse::Unspecified),
                    Request::NumTimers => Ok(SuccessResponse::NumTimers(1)),
                    Request::SetTimeout { timer, relative } => {
                        if timer != 0 {
                            Err(ErrorResponse::TimerOutOfBounds)
                        } else {
                            self.driver
                                .set_timeout(relative)
                                .map(|()| SuccessResponse::SetTimeout)
                                .map_err(|_| ErrorResponse::Unspecified)
                        }
                    }
                    Request::ClearTimeout { timer } => {
                        if timer != 0 {
                            Err(ErrorResponse::TimerOutOfBounds)
                        } else {
                            self.driver
                                .clear_timeout()
                                .map(|()| SuccessResponse::ClearTimeout)
                                .map_err(|_| ErrorResponse::Unspecified)
                        }
                    }
                };
                simple_ipc::send(resp)
            }
            Err(_) => simple_ipc::send_unspecified_error(),
        })
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    HandlerImpl {
        driver: Driver::new(),
        client: CLIENT,
    }
}
