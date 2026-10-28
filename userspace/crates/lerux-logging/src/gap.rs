//! Five-second gap after a boot line on a real x86 CPU.
//!
//! The TSC rate is the frequency in the processor brand string (`CPU @ 4.00GHz`).
//! That matches the invariant TSC on the Haswell desk. A QEMU brand string has
//! no such rate; the ISO boot test uses `qemu64`, so those boots do not pause.

#[cfg(feature = "boot-line-gap")]
use core::cell::UnsafeCell;

#[cfg(all(feature = "boot-line-gap", target_arch = "x86_64"))]
const GAP_SECS: u64 = 5;
/// Used when the brand string has no `@ N.NN GHz` field. The desk CPU is 4 GHz.
#[cfg(all(feature = "boot-line-gap", target_arch = "x86_64"))]
const FALLBACK_HZ: u64 = 4_000_000_000;

#[cfg(feature = "boot-line-gap")]
struct HookCell(UnsafeCell<Option<fn(&str)>>);

// SAFETY: one Microkit protection domain thread installs the hook in init and
// runs it from that thread's log callback.
#[cfg(feature = "boot-line-gap")]
unsafe impl Sync for HookCell {}

#[cfg(feature = "boot-line-gap")]
static HOOK: HookCell = HookCell(UnsafeCell::new(None));

#[cfg(feature = "boot-line-gap")]
pub fn set_hook(hook: fn(&str)) {
    // SAFETY: exclusive to this PD thread; see `HookCell`.
    unsafe {
        *HOOK.0.get() = Some(hook);
    }
}

#[cfg(feature = "boot-line-gap")]
pub fn on_line(line: &str) {
    // SAFETY: same exclusive access as `set_hook`.
    let hook = unsafe { *HOOK.0.get() };
    if let Some(hook) = hook {
        hook(line);
    }
    pause();
}

#[cfg(all(feature = "boot-line-gap", not(target_arch = "x86_64")))]
pub fn pause() {}

#[cfg(all(feature = "boot-line-gap", target_arch = "x86_64"))]
pub fn pause() {
    let Some(hz) = tsc_hz() else {
        return;
    };
    let start = rdtsc();
    let ticks = hz.saturating_mul(GAP_SECS);
    while rdtsc().wrapping_sub(start) < ticks {
        core::hint::spin_loop();
    }
}

#[cfg(all(feature = "boot-line-gap", target_arch = "x86_64"))]
fn tsc_hz() -> Option<u64> {
    let brand = brand_string()?;
    if brand.windows(4).any(|window| window == b"QEMU") {
        return None;
    }
    Some(tsc_hz_from_brand(&brand).unwrap_or(FALLBACK_HZ))
}

#[cfg(all(feature = "boot-line-gap", target_arch = "x86_64"))]
fn brand_string() -> Option<[u8; 48]> {
    let max = core::arch::x86_64::__cpuid(0x8000_0000).eax;
    if max < 0x8000_0004 {
        return None;
    }
    let mut out = [0u8; 48];
    for (index, leaf) in [0x8000_0002u32, 0x8000_0003, 0x8000_0004]
        .into_iter()
        .enumerate()
    {
        let result = core::arch::x86_64::__cpuid(leaf);
        let words = [result.eax, result.ebx, result.ecx, result.edx];
        for (word_index, word) in words.into_iter().enumerate() {
            let base = index * 16 + word_index * 4;
            out[base..base + 4].copy_from_slice(&word.to_le_bytes());
        }
    }
    Some(out)
}

#[cfg(all(feature = "boot-line-gap", target_arch = "x86_64"))]
fn rdtsc() -> u64 {
    // SAFETY: RDTSC is unprivileged and does not touch memory.
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Hertz from a brand string such as `CPU @ 4.00GHz`. `None` when that field is absent.
fn tsc_hz_from_brand(bytes: &[u8]) -> Option<u64> {
    let at = bytes.windows(3).position(|window| window == b" @ ")?;
    let rest = trim_end(&bytes[at + 3..]);
    let split = rest
        .iter()
        .position(u8::is_ascii_alphabetic)
        .unwrap_or(rest.len());
    let milli = parse_milli(&rest[..split])?;
    let unit = &rest[split..];
    if unit.starts_with(b"GHz") {
        Some(milli.saturating_mul(1_000_000))
    } else if unit.starts_with(b"MHz") {
        Some(milli.saturating_mul(1_000))
    } else {
        None
    }
}

fn parse_milli(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() {
        return None;
    }
    let mut whole: u64 = 0;
    let mut index = 0;
    let mut saw_digit = false;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        whole = whole
            .checked_mul(10)?
            .checked_add(u64::from(bytes[index] - b'0'))?;
        saw_digit = true;
        index += 1;
    }
    if !saw_digit {
        return None;
    }
    let mut frac: u64 = 0;
    let mut digits = 0u32;
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() && digits < 3 {
            frac = frac
                .checked_mul(10)?
                .checked_add(u64::from(bytes[index] - b'0'))?;
            digits += 1;
            index += 1;
        }
    }
    if index != bytes.len() {
        return None;
    }
    while digits < 3 {
        frac = frac.checked_mul(10)?;
        digits += 1;
    }
    whole.checked_mul(1000)?.checked_add(frac)
}

fn trim_end(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|byte| *byte != 0 && *byte != b' ')
        .map_or(0, |index| index + 1);
    &bytes[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haswell_brand_is_four_gigahertz() {
        let brand = b"Intel(R) Core(TM) i7-4790K CPU @ 4.00GHz";
        assert_eq!(tsc_hz_from_brand(brand), Some(4_000_000_000));
    }

    #[test]
    fn megahertz_brand_scales() {
        assert_eq!(tsc_hz_from_brand(b"CPU @ 800MHz"), Some(800_000_000));
    }

    #[test]
    fn qemu_brand_has_no_rate() {
        assert_eq!(tsc_hz_from_brand(b"QEMU Virtual CPU version 2.5+"), None);
    }
}
