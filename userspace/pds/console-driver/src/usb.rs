//! USB configuration walking for one HID boot keyboard.
//!
//! The Z97 receiver (`045e:07b2`) advertises interface 0 as boot protocol
//! keyboard, interrupt IN, 8-byte packets. Hubs and report-only interfaces
//! are skipped.

/// Full speed and low speed, the speeds a boot keyboard uses on this board.
pub const SPEED_FULL: u8 = 1;
pub const SPEED_LOW: u8 = 2;

/// One interrupt-IN endpoint on a boot-keyboard interface.
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

/// First HID boot keyboard (class 3, subclass 1, protocol 1) with an IN interrupt.
pub fn find_boot_keyboard(config: &[u8]) -> Option<BootKeyboard> {
    let mut config_value = 0u8;
    let mut boot_interface = None;
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
            let boot = config[index + 5] == 3 && config[index + 6] == 1 && config[index + 7] == 1;
            boot_interface = boot.then_some(config[index + 2]);
        } else if kind == 5
            && len >= 7
            && let Some(interface) = boot_interface
        {
            let addr = config[index + 2];
            let attr = config[index + 3];
            if addr & 0x80 != 0 && attr & 0x03 == 3 {
                let max_packet = u16::from(config[index + 4]) | (u16::from(config[index + 5]) << 8);
                return Some(BootKeyboard {
                    config_value,
                    interface,
                    ep_addr: addr,
                    max_packet,
                    interval: config[index + 6],
                });
            }
        }
        index += len;
    }
    None
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
