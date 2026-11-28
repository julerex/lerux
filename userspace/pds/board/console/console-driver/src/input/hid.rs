//! USB HID boot-keyboard reports, as 8-byte interrupt packets.
//!
//! The shell wants ASCII, the same bytes [`crate::scancode`] produces from
//! PS/2 scan set 1. A report is the set of keys held down, so a press is a
//! usage that was absent from the previous report.

/// Left shift is bit 1, right shift is bit 5.
const SHIFT_BITS: u8 = 0x22;

/// Phantom / rollover. The whole report is noise.
const ROLLOVER: u8 = 0x01;

const USAGE_CAPS: u8 = 0x39;

/// Shift, caps, and the last report's key slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HidState {
    shift: bool,
    caps: bool,
    down: [u8; 6],
}

/// Newly pressed keys in `report`, written into `out`.
///
/// `report` is the boot-protocol packet: modifiers, reserved, then six usages.
/// Enter is `\r`. Backspace is `0x7f`. A rollover report changes nothing.
pub fn decode(state: &mut HidState, report: &[u8], out: &mut [u8]) -> usize {
    if report.len() < 8 || report[2..8].contains(&ROLLOVER) {
        return 0;
    }
    state.shift = report[0] & SHIFT_BITS != 0;
    let mut n = 0;
    for &usage in &report[2..8] {
        if usage == 0 || state.down.contains(&usage) {
            continue;
        }
        if usage == USAGE_CAPS {
            state.caps = !state.caps;
            continue;
        }
        let Some(byte) = usage_byte(usage, state.shift, state.caps) else {
            continue;
        };
        if n < out.len() {
            out[n] = byte;
            n += 1;
        }
    }
    state.down.copy_from_slice(&report[2..8]);
    n
}

fn usage_byte(usage: u8, shift: bool, caps: bool) -> Option<u8> {
    if (0x04..0x1e).contains(&usage) {
        let letter = b'a' + (usage - 0x04);
        let upper = shift ^ caps;
        return Some(if upper {
            letter.to_ascii_uppercase()
        } else {
            letter
        });
    }
    if usage == 0x28 {
        return Some(b'\r');
    }
    if usage == 0x2a {
        return Some(0x7f);
    }
    let (plain, shifted) = match usage {
        0x1e => (b'1', b'!'),
        0x1f => (b'2', b'@'),
        0x20 => (b'3', b'#'),
        0x21 => (b'4', b'$'),
        0x22 => (b'5', b'%'),
        0x23 => (b'6', b'^'),
        0x24 => (b'7', b'&'),
        0x25 => (b'8', b'*'),
        0x26 => (b'9', b'('),
        0x27 => (b'0', b')'),
        0x2c => (b' ', b' '),
        0x2d => (b'-', b'_'),
        0x2e => (b'=', b'+'),
        0x2f => (b'[', b'{'),
        0x30 => (b']', b'}'),
        0x31 => (b'\\', b'|'),
        0x33 => (b';', b':'),
        0x34 => (b'\'', b'"'),
        0x35 => (b'`', b'~'),
        0x36 => (b',', b'<'),
        0x37 => (b'.', b'>'),
        0x38 => (b'/', b'?'),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(state: &mut HidState, usage: u8) -> Option<u8> {
        let mut report = [0u8; 8];
        report[2] = usage;
        let mut out = [0u8; 6];
        let n = decode(state, &report, &mut out);
        (n == 1).then_some(out[0])
    }

    #[test]
    fn echo_hi_usages_become_the_shell_line() {
        let mut state = HidState::default();
        // Boot-protocol usages for `echo hi` and Enter.
        let keys = [
            (0x08, b'e'),
            (0x06, b'c'),
            (0x0b, b'h'),
            (0x12, b'o'),
            (0x2c, b' '),
            (0x0b, b'h'),
            (0x0c, b'i'),
            (0x28, b'\r'),
        ];
        for (usage, expect) in keys {
            assert_eq!(press(&mut state, usage), Some(expect));
            // Release, so the next report is a new press.
            let mut out = [0u8; 6];
            assert_eq!(decode(&mut state, &[0; 8], &mut out), 0);
        }
    }

    #[test]
    fn shift_is_level_and_a_held_key_does_not_repeat() {
        let mut state = HidState::default();
        let mut out = [0u8; 6];
        let mut held = [0u8; 8];
        held[0] = 0x02;
        held[2] = 0x08;
        assert_eq!(decode(&mut state, &held, &mut out), 1);
        assert_eq!(out[0], b'E');
        assert_eq!(decode(&mut state, &held, &mut out), 0);
        held[0] = 0;
        assert_eq!(decode(&mut state, &held, &mut out), 0);
        assert_eq!(decode(&mut state, &[0; 8], &mut out), 0);
        assert_eq!(press(&mut state, 0x08), Some(b'e'));
    }

    #[test]
    fn caps_lock_toggles_letters_only() {
        let mut state = HidState::default();
        assert_eq!(press(&mut state, USAGE_CAPS), None);
        let mut out = [0u8; 6];
        assert_eq!(decode(&mut state, &[0; 8], &mut out), 0);
        assert_eq!(press(&mut state, 0x08), Some(b'E'));
        assert_eq!(decode(&mut state, &[0; 8], &mut out), 0);
        assert_eq!(press(&mut state, 0x1e), Some(b'1'));
    }

    #[test]
    fn rollover_is_ignored() {
        let mut state = HidState::default();
        let mut out = [0u8; 6];
        let report = [0, 0, ROLLOVER, 0x08, 0, 0, 0, 0];
        assert_eq!(decode(&mut state, &report, &mut out), 0);
        assert_eq!(press(&mut state, 0x08), Some(b'e'));
    }
}
