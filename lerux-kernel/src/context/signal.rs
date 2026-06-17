//! Signal delivery to the currently running context.
//!
//! A **signal** is a Unix-style asynchronous notification — `SIGKILL`,
//! `SIGSEGV`, etc. — sent to a process to interrupt it or ask it to handle an
//! event. The kernel cannot deliver a signal at an arbitrary instruction; it
//! waits for a safe point (typically when returning to userspace) and then runs
//! this code.
//!
//! [`signal_handler`] checks the current context for pending signals: if it is
//! being force-killed it exits immediately, otherwise it arranges for the
//! process's registered signal handler to run via the `sigcontrol` shared
//! structure that the kernel and userspace runtime cooperate through.
//!
//! See also: [`docs/kernel/architecture.md`] section 5.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use core::sync::atomic::Ordering;

use crate::{context, sync::CleanLockToken, syscall::SigcontrolFlags};

/// Deliver any pending signal to the current context, at a safe point.
///
/// Called when the kernel is about to resume userspace. Force-kill takes effect
/// here (the context exits); otherwise a pending, unblocked signal is dispatched
/// to the process's handler.
pub fn signal_handler(token: &mut CleanLockToken) {
    let context_lock = context::current();
    let context = context_lock.upgradeable_read(token.token());

    let being_sigkilled = context.being_sigkilled;

    if being_sigkilled {
        drop(context);
        drop(context_lock);
        crate::syscall::process::exit_this_context(None, token);
    }

    /*let thumbs_down = ptrace::breakpoint_callback(
        PTRACE_STOP_SIGNAL,
        Some(ptrace_event!(PTRACE_STOP_SIGNAL)),
    )
    .and_then(|_| ptrace::next_breakpoint().map(|f| f.contains(PTRACE_FLAG_IGNORE)));*/

    // TODO: thumbs_down
    let Some((thread_ctl, proc_ctl, st)) = context.sigcontrol() else {
        // Discard signal if sigcontrol is unset.
        trace!("no sigcontrol, returning");
        return;
    };
    if thread_ctl.currently_pending_unblocked(proc_ctl) == 0 {
        // The context is currently Runnable. When transitioning into Blocked, it will check for
        // signals (with the context lock held, which is required when sending signals). After
        // that, any detection of pending unblocked signals by the sender, will result in the
        // context being unblocked, and signals sent.

        // TODO: prioritize signals over regular program execution
        return;
    }
    let control_flags =
        SigcontrolFlags::from_bits_retain(thread_ctl.control_flags.load(Ordering::Acquire));

    if control_flags.contains(SigcontrolFlags::INHIBIT_DELIVERY) {
        // Signals are inhibited to protect critical sections inside libc, but this code will run
        // every time the context is switched to.
        trace!("Inhibiting delivery, returning");
        return;
    }

    let sigh_instr_ptr = st.user_handler.get();

    let mut context = context.upgrade();
    let Some(regs) = context.regs_mut() else {
        // TODO: is this even reachable?
        trace!("No registers, returning");
        return;
    };

    let ip = regs.instr_pointer();
    let archdep_reg = regs.sig_archdep_reg();

    regs.set_instr_pointer(sigh_instr_ptr);

    let context = context.downgrade();
    let (thread_ctl, _, _) = context
        .sigcontrol()
        .expect("cannot have been unset while holding the lock");

    thread_ctl.saved_ip.set(ip);
    thread_ctl.saved_archdep_reg.set(archdep_reg);

    thread_ctl.control_flags.store(
        (control_flags | SigcontrolFlags::INHIBIT_DELIVERY).bits(),
        Ordering::Release,
    );
}
pub fn excp_handler(excp: crate::syscall::Exception) {
    let mut token = unsafe { CleanLockToken::new() };

    let current = context::current();

    let context = current.write(token.token());

    let Some(eh) = context.sig.as_ref().and_then(|s| s.excp_handler) else {
        // TODO: Let procmgr print this?
        info!(
            "UNHANDLED EXCEPTION, CPU {}, PID {}, NAME {}, CONTEXT {current:p}",
            crate::cpu_id(),
            context.pid,
            context.name
        );
        drop(context);
        // TODO: Allow exceptions to be caught by tracer etc, without necessarily exiting the
        // context (closing files, dropping AddrSpace, etc)
        crate::syscall::process::exit_this_context(Some(excp), &mut token);
    };
    // TODO
    /*
    let Some(regs) = context.regs_mut() else {
        // TODO: unhandled exception in this case too?
        return;
    };
    let old_ip = regs.instr_pointer();
    let old_archdep_reg = regs.ar
    let (tctl, pctl, sigst) = context.sigcontrol().expect("already checked");
    tctl.saved_ip.set(excp.rsp);
    tctl.saved_archdep_reg*/
}
