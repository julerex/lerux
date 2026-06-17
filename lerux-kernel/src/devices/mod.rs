//! Architecture-independent device support the kernel needs for itself.
//!
//! In a microkernel most drivers live in userspace, but the kernel still needs a
//! few devices directly: a **serial port** for console/log output during boot
//! and debugging (the 16550 UART on PCs, the PL011 UART on ARM), and optional
//! **graphical debug** output to a bootloader framebuffer. These are the minimal
//! built-in drivers; everything else is delegated to userspace via schemes.
//!
//! See also: [`docs/kernel/architecture.md`] section 8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

pub mod graphical_debug;
pub mod serial;
pub mod uart_16550;
pub mod uart_pl011;
