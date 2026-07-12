//! Phase 46: parent protection domain that receives child faults.
//!
//! Stock Microkit delivers a child's seL4 fault IPC to the parent's
//! [`Handler::fault`] instead of only the system monitor. This PD logs a
//! structured summary (enough for smoke tests and as a hook for a future
//! GDB RSP stub). Full libgdb requires forked seL4/Microkit — see ADR-005.

#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4::Fault;
use sel4_microkit::{protection_domain, Child, Handler, Infallible, MessageInfo};

/// Child PD index from the system description (`id="1"` on crash_demo).
const CRASH_DEMO: Child = Child::new(1);

struct HandlerImpl {
    fault_count: u32,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("lerux-debug: ready (parent fault handler)");
    HandlerImpl { fault_count: 0 }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn fault(
        &mut self,
        child: Child,
        msg_info: MessageInfo,
    ) -> Result<Option<MessageInfo>, Self::Error> {
        self.fault_count = self.fault_count.saturating_add(1);
        let fault = msg_info.fault();
        log::info!(
            "lerux-debug: fault child={} count={}",
            child.index(),
            self.fault_count
        );
        match fault {
            Fault::VmFault(vm) => {
                log::info!(
                    "lerux-debug: VmFault ip={:#x} addr={:#x} prefetch={} fsr={:#x}",
                    vm.ip(),
                    vm.addr(),
                    vm.is_prefetch(),
                    vm.fsr()
                );
            }
            Fault::CapFault(_) => {
                log::info!("lerux-debug: CapFault");
            }
            Fault::UnknownSyscall(u) => {
                log::info!(
                    "lerux-debug: UnknownSyscall ip={:#x} syscall={}",
                    u.fault_ip(),
                    u.syscall()
                );
            }
            Fault::UserException(_) => {
                log::info!("lerux-debug: UserException");
            }
            other => {
                log::info!("lerux-debug: other fault: {other:?}");
            }
        }
        if child == CRASH_DEMO {
            // Suspend so the fault reply slot is not left dangling for the next recv.
            let _ = child.tcb().tcb_suspend();
            log::info!("lerux-debug: crash-demo stopped (no restart)");
        }
        // Phase 57: machine-parseable one-liner for `lerux diagnose`.
        log::info!(
            "lerux-debug: crash dump child={} count={} (use host gdbstub for backtrace)",
            child.index(),
            self.fault_count
        );
        // Do not reply to the fault (child stays suspended).
        Ok(None)
    }
}
