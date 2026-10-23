//! Key-downs for the desk keyboard.
//!
//! A bare Ctrl transition is not an event. Shift and Caps Lock stay in the
//! decoder and change [`Key::Char`].

#![no_std]

#[cfg(test)]
extern crate std;

/// Events queued for one protection domain. A full ring drops the new push.
pub const KEY_CAP: usize = 64;

/// Named key. Letters and digits are [`Key::Char`] after Shift and Caps Lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(u8),
    Enter,
    Backspace,
    Escape,
    Left,
    Right,
    Tab,
    Up,
    Down,
}

/// One key-down and whether Ctrl is held.
///
/// `ctrl` is the modifier of `key`. A Ctrl key alone never builds this value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub ctrl: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Set1State {
    shift_l: bool,
    shift_r: bool,
    ctrl_l: bool,
    ctrl_r: bool,
    caps: bool,
    extended: bool,
    skip: u8,
}

impl Set1State {
    pub const fn new() -> Self {
        Self {
            shift_l: false,
            shift_r: false,
            ctrl_l: false,
            ctrl_r: false,
            caps: false,
            extended: false,
            skip: 0,
        }
    }

    fn ctrl(self) -> bool {
        self.ctrl_l || self.ctrl_r
    }

    fn shift(self) -> bool {
        self.shift_l || self.shift_r
    }
}

/// One set-1 byte. A break, a bare Ctrl transition, or an unknown make is `None`.
pub fn from_set1(state: &mut Set1State, byte: u8) -> Option<KeyEvent> {
    if state.skip > 0 {
        state.skip -= 1;
        return None;
    }
    // 0xE0 has the high bit set, and it is the extended prefix, not a break.
    if byte == 0xE0 {
        state.extended = true;
        return None;
    }
    // Pause is E1 1D 45 E1 9D C5. The 1D would otherwise become Ctrl.
    if byte == 0xE1 {
        state.skip = 5;
        state.extended = false;
        return None;
    }
    let extended = state.extended;
    state.extended = false;
    if byte & 0x80 != 0 {
        apply_modifier(state, extended, byte & 0x7F, false);
        return None;
    }
    if apply_modifier(state, extended, byte, true) {
        return None;
    }
    if let Some(key) = named(extended, byte) {
        return Some(KeyEvent {
            key,
            ctrl: state.ctrl(),
        });
    }
    if extended {
        return None;
    }
    let ch = base_char(byte)?;
    Some(KeyEvent {
        key: Key::Char(apply_shift(*state, ch)),
        ctrl: state.ctrl(),
    })
}

fn apply_modifier(state: &mut Set1State, extended: bool, code: u8, down: bool) -> bool {
    match (extended, code) {
        (false, 0x2A) => state.shift_l = down,
        (false, 0x36) => state.shift_r = down,
        (false, 0x1D) => state.ctrl_l = down,
        (true, 0x1D) => state.ctrl_r = down,
        (false, 0x3A) if down => state.caps = !state.caps,
        _ => return false,
    }
    true
}

fn named(extended: bool, code: u8) -> Option<Key> {
    if extended {
        return match code {
            0x48 => Some(Key::Up),
            0x4B => Some(Key::Left),
            0x4D => Some(Key::Right),
            0x50 => Some(Key::Down),
            0x1C => Some(Key::Enter),
            _ => None,
        };
    }
    match code {
        0x01 => Some(Key::Escape),
        0x0E => Some(Key::Backspace),
        0x0F => Some(Key::Tab),
        0x1C => Some(Key::Enter),
        _ => None,
    }
}

fn base_char(code: u8) -> Option<u8> {
    Some(match code {
        0x02 => b'1',
        0x03 => b'2',
        0x04 => b'3',
        0x05 => b'4',
        0x06 => b'5',
        0x07 => b'6',
        0x08 => b'7',
        0x09 => b'8',
        0x0A => b'9',
        0x0B => b'0',
        0x0C => b'-',
        0x0D => b'=',
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
        0x1A => b'[',
        0x1B => b']',
        0x1E => b'a',
        0x1F => b's',
        0x20 => b'd',
        0x21 => b'f',
        0x22 => b'g',
        0x23 => b'h',
        0x24 => b'j',
        0x25 => b'k',
        0x26 => b'l',
        0x27 => b';',
        0x28 => b'\'',
        0x29 => b'`',
        0x2B => b'\\',
        0x2C => b'z',
        0x2D => b'x',
        0x2E => b'c',
        0x2F => b'v',
        0x30 => b'b',
        0x31 => b'n',
        0x32 => b'm',
        0x33 => b',',
        0x34 => b'.',
        0x35 => b'/',
        0x39 => b' ',
        _ => return None,
    })
}

fn apply_shift(state: Set1State, ch: u8) -> u8 {
    if ch.is_ascii_alphabetic() {
        if state.shift() ^ state.caps {
            ch.to_ascii_uppercase()
        } else {
            ch
        }
    } else if state.shift() {
        match ch {
            b'1' => b'!',
            b'2' => b'@',
            b'3' => b'#',
            b'4' => b'$',
            b'5' => b'%',
            b'6' => b'^',
            b'7' => b'&',
            b'8' => b'*',
            b'9' => b'(',
            b'0' => b')',
            b'-' => b'_',
            b'=' => b'+',
            b'[' => b'{',
            b']' => b'}',
            b';' => b':',
            b'\'' => b'"',
            b'`' => b'~',
            b'\\' => b'|',
            b',' => b'<',
            b'.' => b'>',
            b'/' => b'?',
            other => other,
        }
    } else {
        ch
    }
}

/// In-memory key ring. `push` returns false when [`KEY_CAP`] events are waiting.
#[derive(Clone, Copy, Debug)]
pub struct KeyRing {
    slots: [KeyEvent; KEY_CAP],
    head: usize,
    len: usize,
}

impl KeyRing {
    pub const fn new() -> Self {
        Self {
            slots: [KeyEvent {
                key: Key::Escape,
                ctrl: false,
            }; KEY_CAP],
            head: 0,
            len: 0,
        }
    }

    /// `false` drops `event`. The events already queued stay in order.
    pub fn push(&mut self, event: KeyEvent) -> bool {
        if self.len == KEY_CAP {
            return false;
        }
        let tail = (self.head + self.len) % KEY_CAP;
        self.slots[tail] = event;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<KeyEvent> {
        if self.len == 0 {
            return None;
        }
        let event = self.slots[self.head];
        self.head = (self.head + 1) % KEY_CAP;
        self.len -= 1;
        Some(event)
    }
}

impl Default for KeyRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_bytes(bytes: &[u8]) -> Option<KeyEvent> {
        let mut state = Set1State::new();
        let mut ring = KeyRing::new();
        for &byte in bytes {
            if let Some(event) = from_set1(&mut state, byte) {
                assert!(ring.push(event), "one decoded event fits in an empty ring");
            }
        }
        ring.pop()
    }

    #[test]
    fn set1_tab_enter_and_backspace_pop() {
        let tab = push_bytes(&[0x0F]).expect("make 0x0F pops as Tab");
        assert_eq!(tab.key, Key::Tab);
        assert!(!tab.ctrl);
        let enter = push_bytes(&[0x1C]).expect("make 0x1C pops as Enter");
        assert_eq!(enter.key, Key::Enter);
        let backspace = push_bytes(&[0x0E]).expect("make 0x0E pops as Backspace");
        assert_eq!(backspace.key, Key::Backspace);
    }

    #[test]
    fn ctrl_then_l_pops_l_with_ctrl() {
        let event = push_bytes(&[0x1D, 0x26]).expect("ctrl held across l pops as the character");
        assert_eq!(event.key, Key::Char(b'l'));
        assert!(event.ctrl);
    }

    #[test]
    fn bare_ctrl_and_breaks_pop_nothing() {
        assert!(
            push_bytes(&[0x1D]).is_none(),
            "a bare Ctrl make pops nothing"
        );
        assert!(
            push_bytes(&[0x9D]).is_none(),
            "a bare Ctrl break pops nothing"
        );
        assert!(push_bytes(&[0x9C]).is_none(), "a break code pops nothing");
        assert!(
            push_bytes(&[0xE0, 0xCD]).is_none(),
            "an extended break pops nothing"
        );
        let event = push_bytes(&[0x1D, 0x9D, 0x26]).expect("a letter after Ctrl is released");
        assert_eq!(event.key, Key::Char(b'l'));
        assert!(!event.ctrl);
    }

    #[test]
    fn extended_right_pops_as_right() {
        let event = push_bytes(&[0xE0, 0x4D]).expect("0xE0 then 0x4D pops as Right");
        assert_eq!(event.key, Key::Right);
        assert!(!event.ctrl);
    }

    #[test]
    fn shift_and_caps_stay_inside_the_decoder() {
        assert!(push_bytes(&[0x2A]).is_none(), "Shift is not an event");
        assert!(push_bytes(&[0x3A]).is_none(), "Caps Lock is not an event");
        let shifted = push_bytes(&[0x2A, 0x26]).expect("Shift changes the letter");
        assert_eq!(shifted.key, Key::Char(b'L'));
        assert!(!shifted.ctrl);
        let caps = push_bytes(&[0x3A, 0x26]).expect("Caps Lock changes the letter");
        assert_eq!(caps.key, Key::Char(b'L'));
    }

    #[test]
    fn extended_prefix_does_not_stick() {
        let mut state = Set1State::new();
        let mut ring = KeyRing::new();
        for byte in [0xE0, 0x26, 0x26] {
            if let Some(event) = from_set1(&mut state, byte) {
                assert!(ring.push(event));
            }
        }
        let event = ring
            .pop()
            .expect("a letter after a dropped extended prefix pops");
        assert_eq!(event.key, Key::Char(b'l'));
        assert!(ring.pop().is_none());
    }

    #[test]
    fn full_ring_drops_the_push_until_a_pop() {
        let mut ring = KeyRing::new();
        for i in 0..KEY_CAP {
            let event = KeyEvent {
                key: Key::Char(b'a' + u8::try_from(i).expect("index fits in a byte")),
                ctrl: false,
            };
            assert!(ring.push(event), "the ring accepts events until it is full");
        }
        let dropped = KeyEvent {
            key: Key::Char(b'Z'),
            ctrl: false,
        };
        assert!(!ring.push(dropped), "a full ring drops the push");
        let first = ring.pop().expect("a full ring still pops the oldest event");
        assert_eq!(first.key, Key::Char(b'a'));
        assert!(ring.push(dropped), "a pop then accepts one more");
    }
}
