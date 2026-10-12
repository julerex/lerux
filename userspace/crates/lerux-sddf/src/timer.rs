//! Timer client page and the time conversion from `include/sddf/timer/`.
//!
//! `timer_client_config_t` matches `include/sddf/timer/config.h`. The get-time
//! and set-timeout labels match `include/sddf/timer/protocol.h`.

/// `SDDF_TIMER_GET_TIME`.
pub const SDDF_TIMER_GET_TIME: u64 = 0;
/// `SDDF_TIMER_SET_TIMEOUT`.
pub const SDDF_TIMER_SET_TIMEOUT: u64 = 1;
/// `SDDF_TIMER_MAX_CLIENTS`.
pub const SDDF_TIMER_MAX_CLIENTS: usize = 64;
/// `SDDF_TIMER_MAGIC_LEN`.
pub const SDDF_TIMER_MAGIC_LEN: usize = 5;

/// Header magic `sDDF` followed by `0x06`.
pub const SDDF_TIMER_MAGIC: [u8; SDDF_TIMER_MAGIC_LEN] = [b's', b'D', b'D', b'F', 0x06];

/// `timer_client_config_t`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timer_client_config_t {
    pub magic: [u8; SDDF_TIMER_MAGIC_LEN],
    pub driver_id: u8,
}

/// Channel the shell uses to call the timer driver.
///
/// Serial transmit, serial receive, and the filesystem server already use 0, 1, and 2.
pub const SHELL_TIMER_CHANNEL: u8 = 3;
/// Channel id on the timer driver. The shell's protected call arrives here.
pub const TIMER_CLIENT_CHANNEL: u8 = 0;

/// One nanosecond, and the frequency `ticks_to_ns` treats as one tick per nanosecond.
const NS_PER_S: u64 = 1_000_000_000;

/// `timer_client_config_t` for the shell.
pub fn shell_client_config() -> timer_client_config_t {
    let mut config: timer_client_config_t = unsafe { core::mem::zeroed() };
    config.magic = SDDF_TIMER_MAGIC;
    config.driver_id = TIMER_CLIENT_CHANNEL;
    config
}

/// Copy a configuration struct into `dst`.
pub fn timer_config_to_bytes<T>(value: &T, dst: &mut [u8]) {
    assert_eq!(dst.len(), core::mem::size_of::<T>());
    // SAFETY: `dst` is exactly one `T`, and the source is an initialised `T`.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(value).cast::<u8>(),
            dst.as_mut_ptr(),
            dst.len(),
        );
    }
}

/// Read a configuration struct out of bytes that came from [`timer_config_to_bytes`].
///
/// # Safety
///
/// `bytes` must be a valid representation of `T`.
pub unsafe fn timer_config_from_bytes<T>(bytes: &[u8]) -> T {
    assert_eq!(bytes.len(), core::mem::size_of::<T>());
    let mut value = core::mem::MaybeUninit::<T>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            value.as_mut_ptr().cast::<u8>(),
            bytes.len(),
        );
        value.assume_init()
    }
}

/// Convert counter ticks at `freq` hertz into nanoseconds.
///
/// This is `ticks_to_ns` from `include/sddf/timer/timer_common.h`. A zero
/// frequency returns 0.
pub fn ticks_to_ns(ticks: u64, freq: u64) -> u64 {
    if freq == 0 {
        return 0;
    }
    let whole = ticks / freq * NS_PER_S;
    let rest = (ticks % freq).saturating_mul(NS_PER_S) / freq;
    whole.saturating_add(rest)
}

/// Calendar date for `ns` nanoseconds after 1970-01-01.
///
/// The timer client returns time since startup, so a guest that has been up
/// for less than a day is still on the first day.
pub fn date_from_ns(ns: u64) -> (u16, u8, u8) {
    civil_from_days(ns / 86_400 / NS_PER_S)
}

/// Howard Hinnant's `civil_from_days`, for a non-negative day count since 1970-01-01.
fn civil_from_days(days: u64) -> (u16, u8, u8) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year as u16, month as u8, day as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qemu_virt_ten_seconds_is_still_the_epoch_day() {
        let ns = ticks_to_ns(625_000_000, 62_500_000);
        assert_eq!(ns, 10_000_000_000);
        assert_eq!(date_from_ns(ns), (1970, 1, 1));
        assert_eq!(date_from_ns(86_400 * NS_PER_S), (1970, 1, 2));
    }

    #[test]
    fn shell_timer_page_names_the_driver_channel() {
        let config = shell_client_config();
        assert_eq!(config.magic, SDDF_TIMER_MAGIC);
        assert_eq!(config.driver_id, TIMER_CLIENT_CHANNEL);
        assert_eq!(SHELL_TIMER_CHANNEL, 3);
    }
}
