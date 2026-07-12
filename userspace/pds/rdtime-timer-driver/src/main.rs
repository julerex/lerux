//! Free-running timer from the RISC-V `rdtime` CSR (mtime shadow).
//!
//! QEMU `virt` timebase is 10 MHz (`timebase-frequency = 0x989680`). No MMIO
//! and no IRQ: CLINT is owned by the kernel. `set_timeout` is a no-op (Phase
//! 56 smoke only needs `get_time` / uptime). Wire format matches
//! `sel4-microkit-driver-adapters` timer messages so `TimerClient` works.

#![no_std]
#![no_main]

use core::{convert::Infallible, time::Duration};

use sel4_driver_interfaces::timer::{Clock, ErrorType, Timer as TimerTrait};
use sel4_microkit::{protection_domain, Channel, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;
use serde::{Deserialize, Serialize};

const CLIENT: Channel = Channel::new(1);

/// QEMU virt / seL4 `TIMER_FREQUENCY` for qemu-riscv-virt.
const TIMEBASE_HZ: u64 = 10_000_000;

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
    start_ticks: u64,
}

impl Driver {
    fn new() -> Self {
        Self {
            start_ticks: read_time(),
        }
    }

    fn elapsed_ticks(&self) -> u64 {
        read_time().saturating_sub(self.start_ticks)
    }
}

fn read_time() -> u64 {
    let mut t: u64;
    // SAFETY: `rdtime` is a valid S/U-mode CSR read on QEMU virt (mtime shadow).
    unsafe {
        core::arch::asm!("rdtime {}", out(reg) t, options(nostack, preserves_flags));
    }
    t
}

impl ErrorType for Driver {
    type Error = Infallible;
}

impl Clock for Driver {
    fn get_time(&mut self) -> Result<Duration, Self::Error> {
        let ticks = self.elapsed_ticks();
        let nanos = (u128::from(ticks) * 1_000_000_000) / u128::from(TIMEBASE_HZ);
        Ok(Duration::from_nanos(nanos as u64))
    }
}

impl TimerTrait for Driver {
    fn set_timeout(&mut self, _relative: Duration) -> Result<(), Self::Error> {
        // No userspace IRQ path without CLINT mtimecmp (kernel-owned).
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
