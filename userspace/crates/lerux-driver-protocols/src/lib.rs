//! Shared postcard wire protocols for lerux driver PDs.
//!
//! Each protocol here is the contract between two PDs; declaring it once makes
//! drift a compile error instead of a runtime decode failure in QEMU.
//!
//! - [`serial`]: byte-at-a-time serial passthrough spoken between
//!   `serial-driver`, `serial-virt`, and the rust-sel4 `SerialClient` adapter.
//! - [`timer`]: get-time / timeout protocol spoken by the arch timer PDs
//!   (`rdtime-timer-driver`, `tsc-timer-driver`); mirrors the **private**
//!   `sel4_microkit_driver_adapters::timer::message_types` so the upstream
//!   `TimerClient` keeps working.

#![no_std]

pub mod serial {
    //! Serial passthrough protocol (one byte per RPC).
    //!
    //! Wire format matches the rust-sel4 `SerialClient` adapter and must not
    //! change without updating every serial PD together.

    use embedded_hal_nb::nb;
    use serde::{Deserialize, Serialize};

    /// Poll result carried over the wire (`nb`-style readiness).
    #[derive(Debug, Serialize, Deserialize)]
    pub enum NonBlocking<T> {
        Ready(T),
        WouldBlock,
    }

    impl<T> NonBlocking<T> {
        /// Map an `nb` poll result into the wire type, keeping hard errors.
        pub fn from_nb_result<E>(r: nb::Result<T, E>) -> Result<Self, E> {
            match r {
                Ok(v) => Ok(Self::Ready(v)),
                Err(nb::Error::WouldBlock) => Ok(Self::WouldBlock),
                Err(nb::Error::Other(err)) => Err(err),
            }
        }
    }

    impl<T> From<Option<T>> for NonBlocking<T> {
        fn from(v: Option<T>) -> Self {
            match v {
                Some(v) => NonBlocking::Ready(v),
                None => NonBlocking::WouldBlock,
            }
        }
    }

    impl<T, E> From<NonBlocking<T>> for nb::Result<T, E> {
        fn from(v: NonBlocking<T>) -> Self {
            match v {
                NonBlocking::Ready(v) => Ok(v),
                NonBlocking::WouldBlock => Err(nb::Error::WouldBlock),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Request {
        Read,
        Write(u8),
        Flush,
    }

    pub type Response = Result<SuccessResponse, ErrorResponse>;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum SuccessResponse {
        Read(NonBlocking<u8>),
        Write(NonBlocking<()>),
        Flush(NonBlocking<()>),
    }

    #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
    pub enum ErrorResponse {
        WriteError,
        FlushError,
    }
}

pub mod timer {
    //! Timer protocol and a generic handler for free-running clock PDs.
    //!
    //! The message enums mirror `sel4_microkit_driver_adapters::timer::message_types`
    //! (private upstream), so the rust-sel4 `TimerClient` can call these PDs.

    use core::{convert::Infallible, time::Duration};

    use sel4_driver_interfaces::timer::{Clock, Timer};
    use sel4_microkit::{Channel, Handler, MessageInfo};
    use sel4_microkit_simple_ipc as simple_ipc;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Request {
        GetTime,
        NumTimers,
        SetTimeout { timer: usize, relative: Duration },
        ClearTimeout { timer: usize },
    }

    pub type Response = Result<SuccessResponse, ErrorResponse>;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum SuccessResponse {
        GetTime(Duration),
        NumTimers(usize),
        SetTimeout,
        ClearTimeout,
    }

    #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
    pub enum ErrorResponse {
        TimerOutOfBounds,
        Unspecified,
    }

    /// Single-client, single-timer RPC handler over any [`Clock`] + [`Timer`]
    /// driver. Arch timer PDs only implement the driver; dispatch lives here.
    pub struct TimerHandler<D> {
        driver: D,
        client: Channel,
    }

    impl<D: Clock + Timer> TimerHandler<D> {
        pub fn new(driver: D, client: Channel) -> Self {
            Self { driver, client }
        }

        fn handle_request(&mut self, req: Request) -> Response {
            match req {
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
            }
        }
    }

    impl<D: Clock + Timer> Handler for TimerHandler<D> {
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
                Ok(req) => simple_ipc::send(self.handle_request(req)),
                Err(_) => simple_ipc::send_unspecified_error(),
            })
        }
    }
}
