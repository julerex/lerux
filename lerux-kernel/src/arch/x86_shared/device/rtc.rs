//! Real-time clock (RTC) support placeholder for x86.
//!
//! Intentionally empty: on x86 the kernel currently reads wall-clock time via
//! other paths, and RTC access is left to userspace. This module exists so the
//! arch layout matches other architectures that do drive an RTC here.
