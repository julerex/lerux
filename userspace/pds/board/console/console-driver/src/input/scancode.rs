//! PS/2 scan set 1 (what QEMU and PC firmware present when translation is on).

/// Shift, caps, and the E0/E1 prefix, across IRQs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyState {
    shift: bool,
    caps: bool,
    extended: bool,
}

/// Map one set-1 byte to an ASCII byte the shell already understands.
///
/// Enter is `\r`. Backspace is `0x7f`. Break codes, shifts, and extended
/// keys (arrows) produce nothing.
pub fn decode(state: &mut KeyState, code: u8) -> Option<u8> {
    if state.extended {
        state.extended = false;
        return None;
    }
    if code == 0xe0 || code == 0xe1 {
        state.extended = true;
        return None;
    }

    let released = code & 0x80 != 0;
    let make = code & 0x7f;
    if make == 0x2a || make == 0x36 {
        state.shift = !released;
        return None;
    }
    if released {
        return None;
    }
    if make == 0x3a {
        state.caps = !state.caps;
        return None;
    }
    if make == 0x1c {
        return Some(b'\r');
    }
    if make == 0x0e {
        return Some(0x7f);
    }
    if let Some(letter) = letter(make) {
        let upper = state.shift ^ state.caps;
        return Some(if upper {
            letter.to_ascii_uppercase()
        } else {
            letter
        });
    }
    symbol(make, state.shift)
}

fn letter(make: u8) -> Option<u8> {
    Some(match make {
        0x10 => b'q',
        0x11 => b'w',
        0x12 => b'e',
        0x13 => b'r',
        0x14 => b't',
        0x15 => b'y',
        0x16 => b'u',
        0x17 => b'i',
        0x18 => b'o',
        0x19 => b'p',
        0x1e => b'a',
        0x1f => b's',
        0x20 => b'd',
        0x21 => b'f',
        0x22 => b'g',
        0x23 => b'h',
        0x24 => b'j',
        0x25 => b'k',
        0x26 => b'l',
        0x2c => b'z',
        0x2d => b'x',
        0x2e => b'c',
        0x2f => b'v',
        0x30 => b'b',
        0x31 => b'n',
        0x32 => b'm',
        _ => return None,
    })
}

fn symbol(make: u8, shift: bool) -> Option<u8> {
    let (plain, shifted) = match make {
        0x02 => (b'1', b'!'),
        0x03 => (b'2', b'@'),
        0x04 => (b'3', b'#'),
        0x05 => (b'4', b'$'),
        0x06 => (b'5', b'%'),
        0x07 => (b'6', b'^'),
        0x08 => (b'7', b'&'),
        0x09 => (b'8', b'*'),
        0x0a => (b'9', b'('),
        0x0b => (b'0', b')'),
        0x0c => (b'-', b'_'),
        0x0d => (b'=', b'+'),
        0x1a => (b'[', b'{'),
        0x1b => (b']', b'}'),
        0x27 => (b';', b':'),
        0x28 => (b'\'', b'"'),
        0x29 => (b'`', b'~'),
        0x2b => (b'\\', b'|'),
        0x33 => (b',', b'<'),
        0x34 => (b'.', b'>'),
        0x35 => (b'/', b'?'),
        0x39 => (b' ', b' '),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_hi_scancodes_become_the_shell_line() {
        let mut state = KeyState::default();
        // Set 1 make codes for `echo hi` and Enter.
        let keys = [
            (0x12, b'e'),
            (0x2e, b'c'),
            (0x23, b'h'),
            (0x18, b'o'),
            (0x39, b' '),
            (0x23, b'h'),
            (0x17, b'i'),
            (0x1c, b'\r'),
        ];
        for (code, expect) in keys {
            assert_eq!(decode(&mut state, code), Some(expect));
        }
    }

    #[test]
    fn shift_and_break_do_not_stick() {
        let mut state = KeyState::default();
        assert_eq!(decode(&mut state, 0x2a), None);
        assert_eq!(decode(&mut state, 0x12), Some(b'E'));
        assert_eq!(decode(&mut state, 0xaa), None);
        assert_eq!(decode(&mut state, 0x12), Some(b'e'));
        assert_eq!(decode(&mut state, 0x92), None);
    }

    #[test]
    fn backspace_and_extended_keys() {
        let mut state = KeyState::default();
        assert_eq!(decode(&mut state, 0x0e), Some(0x7f));
        assert_eq!(decode(&mut state, 0xe0), None);
        assert_eq!(decode(&mut state, 0x48), None);
        assert_eq!(decode(&mut state, 0x12), Some(b'e'));
    }

    #[test]
    fn caps_lock_toggles_letters_only() {
        let mut state = KeyState::default();
        assert_eq!(decode(&mut state, 0x3a), None);
        assert_eq!(decode(&mut state, 0x12), Some(b'E'));
        assert_eq!(decode(&mut state, 0x02), Some(b'1'));
    }
}
