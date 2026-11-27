//! USB configuration walking for one HID boot keyboard.
//!
//! The Z97 receiver (`045e:07b2`) advertises interface 0 as boot protocol
//! keyboard, interrupt IN, 8-byte packets. Hubs and report-only interfaces
//! are skipped.

/// Full speed and low speed, the speeds a boot keyboard uses on this board.
pub const SPEED_FULL: u8 = 1;
pub const SPEED_LOW: u8 = 2;

/// One interrupt-IN endpoint on a boot-keyboard or boot-mouse interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootKeyboard {
    pub config_value: u8,
    pub interface: u8,
    /// Endpoint address, including the IN direction bit.
    pub ep_addr: u8,
    pub max_packet: u16,
    /// `bInterval` from the endpoint descriptor, not the xHCI field.
    pub interval: u8,
}

/// seL4 x86 MSI data vector for a Microkit SDF `vector`.
///
/// `sel4/arch/constants.h` says an allocated vector X is delivered as
/// `X + IRQ_OFFSET`, and `IRQ_OFFSET` is `0x20 + 16`. The kernel does not
/// rewrite the MSI message.
pub fn msi_data(sdf_vector: u32) -> u32 {
    sdf_vector + 0x30
}

/// HCSPARAMS2 scratchpad count. Low five bits are 31:27, high five are 25:21.
pub fn scratchpad_count(hcsparams2: u32) -> u32 {
    ((hcsparams2 >> 16) & 0x3e0) | ((hcsparams2 >> 27) & 0x1f)
}

/// Device context index of an endpoint address (`0x81` is IN endpoint 1 → 3).
pub fn endpoint_dci(ep_addr: u8) -> u8 {
    let number = ep_addr & 0x0f;
    if ep_addr & 0x80 != 0 {
        number * 2 + 1
    } else {
        number * 2
    }
}

/// xHCI endpoint-context Interval for an interrupt endpoint.
///
/// Full- and low-speed `bInterval` counts 1 ms frames. The controller wants
/// that period as a microframe exponent: `fls(bInterval * 8) - 1`.
pub fn xhci_interval(speed: u8, b_interval: u8) -> u8 {
    if speed == SPEED_FULL || speed == SPEED_LOW {
        let frames = u32::from(b_interval.max(1));
        let exponent = 32 - (frames.saturating_mul(8)).leading_zeros() - 1;
        return u8::try_from(exponent.clamp(3, 10)).unwrap_or(5);
    }
    b_interval.clamp(1, 16) - 1
}

/// Last HID boot keyboard (class 3, subclass 1, protocol 1) with an IN interrupt.
///
/// An earlier boot interface must not hide a later one. The Z97 typing dongle
/// is not the first HID device on the bus. A mouse after the only keyboard
/// does not replace it.
pub fn find_boot_keyboard(config: &[u8]) -> Option<BootKeyboard> {
    find_boot(config, 1)
}

/// The boot keyboard the shell types from, or `None` for a mouse.
///
/// A device whose boot mouse sits on an earlier interface than its boot
/// keyboard is a mouse. The Holtek gaming mouse is that shape. The Microsoft
/// receiver puts its keyboard on interface 0 and its mouse after it.
pub fn find_shell_keyboard(config: &[u8]) -> Option<BootKeyboard> {
    let keyboard = find_boot_keyboard(config)?;
    match find_boot_mouse(config) {
        Some(mouse) if mouse.interface < keyboard.interface => None,
        _ => Some(keyboard),
    }
}

/// First HID boot mouse (class 3, subclass 1, protocol 2) with an IN interrupt.
///
/// Protocol 1 is a boot keyboard. The Holtek gaming mouse advertises that
/// interface as well, and it is not the shell keyboard.
pub fn find_boot_mouse(config: &[u8]) -> Option<BootKeyboard> {
    find_boot(config, 2)
}

fn find_boot(config: &[u8], protocol: u8) -> Option<BootKeyboard> {
    let mut config_value = 0u8;
    let mut boot_interface = None;
    let mut found = None;
    let mut index = 0usize;
    while index + 1 < config.len() {
        let len = usize::from(config[index]);
        if len < 2 || index + len > config.len() {
            break;
        }
        let kind = config[index + 1];
        if kind == 2 && len >= 6 {
            config_value = config[index + 5];
        } else if kind == 4 && len >= 8 {
            let boot =
                config[index + 5] == 3 && config[index + 6] == 1 && config[index + 7] == protocol;
            boot_interface = boot.then_some(config[index + 2]);
        } else if kind == 5
            && len >= 7
            && let Some(interface) = boot_interface
        {
            let addr = config[index + 2];
            let attr = config[index + 3];
            if addr & 0x80 != 0 && attr & 0x03 == 3 {
                let max_packet = u16::from(config[index + 4]) | (u16::from(config[index + 5]) << 8);
                found = Some(BootKeyboard {
                    config_value,
                    interface,
                    ep_addr: addr,
                    max_packet,
                    interval: config[index + 6],
                });
                boot_interface = None;
            }
        }
        index += len;
    }
    found
}

/// Root ports the driver walks. `HCSPARAMS1` is clamped to this.
pub const MAX_PORTS: usize = 30;

/// Port mark for the device that armed as the keyboard.
pub const MARK_KEYBOARD: u8 = b'k';

/// Longest [`status_line`]: the note, 30 marked ports, and the code.
pub const STATUS_MAX: usize = 192;

/// `usb: NOTE, ports 4a 10n, cc 4` for a bring-up that armed no keyboard.
///
/// Bit `n - 1` of `connected` is root port `n`. `marks[n - 1]` is where
/// that port stopped as a keyboard: `r` port reset, `s` enable slot, `a`
/// address device, `d` device descriptor, `g` configuration descriptor,
/// `n` no shell keyboard in it, `p` set configuration or boot protocol,
/// `i` configure the interrupt endpoint. 0 is not tried. `cc` is the last
/// xHCI completion code that was not success, and 0 means no event came.
/// `ep` is endpoint 0's context state after the last control transfer that
/// got no event: 0 disabled, 1 running, 2 halted, 3 stopped, 4 error.
pub fn status_line(
    note: &str,
    connected: u32,
    marks: &[u8; MAX_PORTS],
    cc: Option<u8>,
    ep: Option<u8>,
    out: &mut [u8; STATUS_MAX],
) -> usize {
    let mut n = 0;
    let mut put = |bytes: &[u8]| {
        for &byte in bytes {
            if n < out.len() {
                out[n] = byte;
                n += 1;
            }
        }
    };
    put(b"usb: ");
    put(note.as_bytes());
    if connected != 0 {
        put(b", ports");
        for (index, &mark) in marks.iter().enumerate() {
            if connected & (1 << index) == 0 {
                continue;
            }
            put(b" ");
            put_decimal(&mut put, index + 1);
            if mark != 0 {
                put(&[mark]);
            }
        }
    }
    if let Some(code) = cc {
        put(b", cc ");
        put_decimal(&mut put, usize::from(code));
    }
    if let Some(state) = ep {
        put(b" ep ");
        put_decimal(&mut put, usize::from(state));
    }
    put(b"\n");
    n
}

fn put_decimal(put: &mut impl FnMut(&[u8]), value: usize) {
    let mut digits = [0u8; 3];
    let mut len = 0;
    let mut rest = value.min(999);
    loop {
        digits[len] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        len += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    digits[..len].reverse();
    put(&digits[..len]);
}

/// VGA text page the I217 shell paints.
#[cfg(any(test, feature = "board-pc_z97_d3h_i217"))]
const POINTER_COLS: i32 = 80;
#[cfg(any(test, feature = "board-pc_z97_d3h_i217"))]
const POINTER_ROWS: i32 = 25;

/// Add a boot-mouse report to a cell. Bytes are buttons, X, Y. X and Y are
/// signed displacements. Later bytes in an 8-byte packet are ignored.
#[cfg(any(test, feature = "board-pc_z97_d3h_i217"))]
pub fn step_pointer(col: i32, row: i32, report: &[u8]) -> (i32, i32, u8) {
    if report.len() < 3 {
        return (col, row, 0);
    }
    let dx = i32::from(report[1].cast_signed());
    let dy = i32::from(report[2].cast_signed());
    let col = (col + dx).clamp(0, POINTER_COLS - 1);
    let row = (row + dy).clamp(0, POINTER_ROWS - 1);
    (col, row, report[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interface 0 boot keyboard, then a boot mouse. Same shape as the Z97 receiver.
    fn receiver_config() -> [u8; 34] {
        [
            9, 2, 34, 0, 1, 1, 0, 0xa0, 50, // configuration, value 1
            9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0, boot keyboard
            9, 0x21, 0x11, 0x01, 0, 1, 0x22, 75, 0, // HID
            7, 5, 0x81, 3, 8, 0, 4, // interrupt IN, 8 bytes, interval 4
        ]
    }

    #[test]
    fn finds_the_boot_keyboard_ahead_of_other_interfaces() {
        let mut config = [0u8; 50];
        let head = receiver_config();
        config[..head.len()].copy_from_slice(&head);
        // A second interface (boot mouse) must not replace the keyboard.
        let mouse = [9u8, 4, 1, 0, 1, 3, 1, 2, 0, 7, 5, 0x82, 3, 4, 0, 1];
        // Fix the configuration's interface count and length loosely: the walker
        // does not require wTotalLength to cover the extra interface.
        config[head.len()..head.len() + mouse.len()].copy_from_slice(&mouse);
        let found = find_boot_keyboard(&config[..head.len() + mouse.len()]).unwrap();
        assert_eq!(
            found,
            BootKeyboard {
                config_value: 1,
                interface: 0,
                ep_addr: 0x81,
                max_packet: 8,
                interval: 4,
            }
        );
    }

    #[test]
    fn a_later_boot_keyboard_wins_over_an_earlier_interface() {
        // Boot keyboard, then a vendor interface, then the target boot keyboard.
        // Returning the first interface arms the wrong device.
        let config = [
            9u8, 2, 57, 0, 3, 1, 0, 0xa0, 50, // configuration, three interfaces
            9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0, boot keyboard
            7, 5, 0x81, 3, 8, 0, 4, // interrupt IN
            9, 4, 1, 0, 1, 0xff, 0, 0, 0, // interface 1, vendor, not a keyboard
            7, 5, 0x82, 3, 8, 0, 1, //
            9, 4, 2, 0, 1, 3, 1, 1, 0, // interface 2, target boot keyboard
            7, 5, 0x83, 3, 8, 0, 8, // interrupt IN, interval 8
        ];
        let found = find_boot_keyboard(&config).unwrap();
        assert_eq!(
            found,
            BootKeyboard {
                config_value: 1,
                interface: 2,
                ep_addr: 0x83,
                max_packet: 8,
                interval: 8,
            }
        );
    }

    #[test]
    fn holtek_mouse_keeps_the_boot_mouse_and_not_its_keyboard() {
        // Interface 0 boot mouse `0x81`, interface 1 boot keyboard `0x82`.
        // The keyboard interface is the Holtek one and is not the shell keyboard.
        let config = [
            9u8, 2, 50, 0, 2, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 1, 3, 1, 2, 0, // boot mouse
            7, 5, 0x81, 3, 8, 0, 1, //
            9, 4, 1, 0, 1, 3, 1, 1, 0, // boot keyboard
            7, 5, 0x82, 3, 8, 0, 2, //
        ];
        let mouse = find_boot_mouse(&config).unwrap();
        assert_eq!(mouse.interface, 0);
        assert_eq!(mouse.ep_addr, 0x81);
        assert_eq!(mouse.interval, 1);
        assert_eq!(find_boot_keyboard(&config).unwrap().interface, 1);
        assert!(find_shell_keyboard(&config).is_none());
        let (col, row, buttons) = step_pointer(0, 0, &[0, 10, 4, 0, 0, 0, 0, 0]);
        assert_eq!((col, row, buttons), (10, 4, 0));
        let (col, row, buttons) = step_pointer(col, row, &[1, 0xff, 0]);
        assert_eq!((col, row, buttons), (9, 4, 1));
        let (col, row, _) = step_pointer(0, 0, &[0, 0x80, 0x7f]);
        assert_eq!((col, row), (0, 24));
    }

    #[test]
    fn the_receiver_keyboard_is_the_shell_keyboard_ahead_of_its_mouse() {
        let head = receiver_config();
        let mouse = [9u8, 4, 1, 0, 1, 3, 1, 2, 0, 7, 5, 0x82, 3, 4, 0, 1];
        let mut config = [0u8; 50];
        config[..head.len()].copy_from_slice(&head);
        config[head.len()..head.len() + mouse.len()].copy_from_slice(&mouse);
        let found = find_shell_keyboard(&config[..head.len() + mouse.len()]).unwrap();
        assert_eq!(found.interface, 0);
        assert_eq!(found.ep_addr, 0x81);
    }

    #[test]
    fn status_line_names_the_note_the_ports_and_the_code() {
        let mut out = [0u8; STATUS_MAX];
        let mut marks = [0u8; MAX_PORTS];
        marks[3] = b'a';
        marks[9] = b'n';
        marks[13] = MARK_KEYBOARD;
        let connected = (1 << 3) | (1 << 9) | (1 << 13) | (1 << 15);
        let n = status_line(
            "xhci no keyboard",
            connected,
            &marks,
            Some(4),
            None,
            &mut out,
        );
        assert_eq!(
            &out[..n],
            b"usb: xhci no keyboard, ports 4a 10n 14k 16, cc 4\n"
        );
        marks[3] = b'd';
        let n = status_line(
            "xhci no keyboard",
            1 << 3,
            &marks,
            Some(0),
            Some(1),
            &mut out,
        );
        assert_eq!(&out[..n], b"usb: xhci no keyboard, ports 4d, cc 0 ep 1\n");
        let n = status_line("xhci none", 0, &[0; MAX_PORTS], None, None, &mut out);
        assert_eq!(&out[..n], b"usb: xhci none\n");
    }

    #[test]
    fn a_mouse_only_config_is_not_a_keyboard() {
        let mouse = [
            9u8, 2, 25, 0, 1, 1, 0, 0xa0, 50, 9, 4, 0, 0, 1, 3, 1, 2, 0, 7, 5, 0x81, 3, 4, 0, 1,
        ];
        assert!(find_boot_keyboard(&mouse).is_none());
    }

    #[test]
    fn full_speed_interval_four_is_exponent_five() {
        // bInterval 4 frames = 32 microframes, fls(32) - 1 = 5.
        assert_eq!(xhci_interval(SPEED_FULL, 4), 5);
        assert_eq!(xhci_interval(3, 4), 3);
    }

    #[test]
    fn msi_data_adds_the_sel4_irq_offset() {
        // SDF vector 50, the console USB irq. Delivery vector is 98.
        assert_eq!(msi_data(50), 98);
    }

    #[test]
    fn scratchpad_count_joins_the_split_fields() {
        let low = 2u32 << 27;
        let high = 1u32 << 21;
        assert_eq!(scratchpad_count(low | high), (1 << 5) | 2);
        assert_eq!(scratchpad_count(0), 0);
    }

    #[test]
    fn keyboard_endpoint_dci_is_three() {
        assert_eq!(endpoint_dci(0x81), 3);
        assert_eq!(endpoint_dci(0x01), 2);
    }
}
