//! USB configuration walking for the desk keyboard.
//!
//! [`classify_keyboard`] prefers the Microsoft 2.4 GHz transceiver (`045e:07b2`)
//! when that configuration has a boot-keyboard endpoint. Any other device counts
//! only when interface 0 itself is the boot keyboard. A boot keyboard on a later
//! interface is not a fallback. Hubs and report-only interfaces are skipped.

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

/// Last HID boot keyboard (class 3, subclass 1, protocol 1) with an IN interrupt
/// in this configuration.
///
/// An earlier boot interface must not hide a later one. A mouse interface after
/// the only keyboard does not replace it.
pub fn find_boot_keyboard(config: &[u8]) -> Option<BootKeyboard> {
    let mut config_value = 0u8;
    let mut boot_interface = None;
    let mut found = None;
    let mut index = 0usize;
    while index + 1 < config.len() {
        let len = usize::from(config[index]);
        if len < 2 || index + len > config.len() {
            break;
        }
        let desc = &config[index..index + len];
        let kind = desc[1];
        if kind == 2 && len >= 6 {
            config_value = desc[5];
        } else if kind == 4 && len >= 8 {
            boot_interface = is_boot_keyboard(desc).then_some(desc[2]);
        } else if kind == 5
            && let Some(interface) = boot_interface
            && let Some(kbd) = boot_keyboard_endpoint(desc, config_value, interface)
        {
            found = Some(kbd);
            boot_interface = None;
        }
        index += len;
    }
    found
}

const MICROSOFT_VENDOR: u16 = 0x045e;
const TRANSCEIVER_PRODUCT: u16 = 0x07b2;

fn is_boot_keyboard(interface: &[u8]) -> bool {
    interface.len() >= 8 && interface[5] == 3 && interface[6] == 1 && interface[7] == 1
}

fn boot_keyboard_endpoint(desc: &[u8], config_value: u8, interface: u8) -> Option<BootKeyboard> {
    if desc.len() < 7 {
        return None;
    }
    let addr = desc[2];
    let attr = desc[3];
    if addr & 0x80 == 0 || attr & 0x03 != 3 {
        return None;
    }
    Some(BootKeyboard {
        config_value,
        interface,
        ep_addr: addr,
        max_packet: u16::from(desc[4]) | (u16::from(desc[5]) << 8),
        interval: desc[6],
    })
}

/// Boot keyboard on interface 0, ignoring a boot keyboard on a later interface.
fn interface_zero_keyboard(config: &[u8]) -> Option<BootKeyboard> {
    let mut config_value = 0u8;
    let mut on_zero = false;
    let mut found = None;
    let mut index = 0usize;
    while index + 1 < config.len() {
        let len = usize::from(config[index]);
        if len < 2 || index + len > config.len() {
            break;
        }
        let desc = &config[index..index + len];
        let kind = desc[1];
        if kind == 2 && len >= 6 {
            config_value = desc[5];
        } else if kind == 4 && len >= 8 {
            on_zero = desc[2] == 0 && is_boot_keyboard(desc);
        } else if kind == 5 && on_zero && found.is_none() {
            found = boot_keyboard_endpoint(desc, config_value, 0);
        }
        index += len;
    }
    found
}

/// Shell keyboard on the Z97 desk.
///
/// The Microsoft 2.4 GHz transceiver (`045e:07b2`) wins when its configuration
/// has a boot-keyboard endpoint. Otherwise only interface 0 counts. A boot
/// keyboard on a later interface is not a fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardChoice {
    /// Transceiver `045e:07b2` and the boot-keyboard endpoint in its configuration.
    MicrosoftTransceiver(BootKeyboard),
    /// Interface 0 itself is the boot keyboard.
    InterfaceZero(BootKeyboard),
}

/// Classify one device as the shell keyboard.
///
/// `vendor` and `product` come from the device descriptor. `config` is one
/// configuration descriptor. The Microsoft transceiver is
/// [`KeyboardChoice::MicrosoftTransceiver`] when [`find_boot_keyboard`] finds
/// an endpoint. Any other device is [`KeyboardChoice::InterfaceZero`] only
/// when interface 0 is that boot keyboard.
pub fn classify_keyboard(vendor: u16, product: u16, config: &[u8]) -> Option<KeyboardChoice> {
    if vendor == MICROSOFT_VENDOR && product == TRANSCEIVER_PRODUCT {
        return find_boot_keyboard(config).map(KeyboardChoice::MicrosoftTransceiver);
    }
    interface_zero_keyboard(config).map(KeyboardChoice::InterfaceZero)
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
    fn a_mouse_only_config_is_not_a_keyboard() {
        let mouse = [
            9u8, 2, 25, 0, 1, 1, 0, 0xa0, 50, 9, 4, 0, 0, 1, 3, 1, 2, 0, 7, 5, 0x81, 3, 4, 0, 1,
        ];
        assert!(find_boot_keyboard(&mouse).is_none());
    }

    /// Device descriptor plus the one configuration, copied from sysfs on 2026-11-22.
    /// `1-4` is the Microsoft transceiver. `1-10` is the Holtek mouse.
    const MICROSOFT_DESCRIPTORS: [u8; 102] = [
        0x12, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x40, 0x5e, 0x04, 0xb2, 0x07, 0x43, 0x07, 0x01,
        0x02, 0x00, 0x01, 0x09, 0x02, 0x54, 0x00, 0x03, 0x01, 0x00, 0xa0, 0x32, 0x09, 0x04, 0x00,
        0x00, 0x01, 0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x11, 0x01, 0x00, 0x01, 0x22, 0x4b, 0x00,
        0x07, 0x05, 0x81, 0x03, 0x08, 0x00, 0x04, 0x09, 0x04, 0x01, 0x00, 0x01, 0x03, 0x01, 0x02,
        0x00, 0x09, 0x21, 0x11, 0x01, 0x00, 0x01, 0x22, 0xdf, 0x00, 0x07, 0x05, 0x82, 0x03, 0x0a,
        0x00, 0x01, 0x09, 0x04, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x09, 0x21, 0x11, 0x01,
        0x00, 0x01, 0x22, 0x3f, 0x01, 0x07, 0x05, 0x83, 0x03, 0x20, 0x00, 0x01,
    ];
    const HOLTEK_DESCRIPTORS: [u8; 102] = [
        0x12, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x08, 0xd9, 0x04, 0x4d, 0xfc, 0x52, 0x01, 0x00,
        0x02, 0x00, 0x01, 0x09, 0x02, 0x54, 0x00, 0x03, 0x01, 0x00, 0xa0, 0x32, 0x09, 0x04, 0x00,
        0x00, 0x01, 0x03, 0x01, 0x02, 0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x43, 0x00,
        0x07, 0x05, 0x81, 0x03, 0x08, 0x00, 0x01, 0x09, 0x04, 0x01, 0x00, 0x01, 0x03, 0x01, 0x01,
        0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x2f, 0x00, 0x07, 0x05, 0x82, 0x03, 0x08,
        0x00, 0x02, 0x09, 0x04, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x09, 0x21, 0x10, 0x01,
        0x00, 0x01, 0x22, 0x6a, 0x00, 0x07, 0x05, 0x83, 0x03, 0x08, 0x00, 0x04,
    ];

    fn configuration(descriptors: &[u8]) -> &[u8] {
        assert_eq!(descriptors[0], 18);
        assert_eq!(descriptors[1], 1);
        let total = usize::from(descriptors[20]) | (usize::from(descriptors[21]) << 8);
        assert_eq!(descriptors.len(), 18 + total);
        &descriptors[18..18 + total]
    }

    fn classify_descriptors(descriptors: &[u8]) -> Option<KeyboardChoice> {
        let vendor = u16::from(descriptors[8]) | (u16::from(descriptors[9]) << 8);
        let product = u16::from(descriptors[10]) | (u16::from(descriptors[11]) << 8);
        classify_keyboard(vendor, product, configuration(descriptors))
    }

    #[test]
    fn microsoft_transceiver_config_is_the_shell_keyboard() {
        assert_eq!(
            classify_descriptors(&MICROSOFT_DESCRIPTORS),
            Some(KeyboardChoice::MicrosoftTransceiver(BootKeyboard {
                config_value: 1,
                interface: 0,
                ep_addr: 0x81,
                max_packet: 8,
                interval: 4,
            }))
        );
    }

    #[test]
    fn holtek_mouse_config_is_not_a_keyboard() {
        // Interface 0 is the boot mouse. Interface 1 is a boot keyboard.
        assert_eq!(classify_descriptors(&HOLTEK_DESCRIPTORS), None);
    }

    #[test]
    fn interface_zero_boot_keyboard_is_the_fallback() {
        let found = classify_keyboard(0x1234, 0x0001, &receiver_config()).unwrap();
        assert_eq!(
            found,
            KeyboardChoice::InterfaceZero(BootKeyboard {
                config_value: 1,
                interface: 0,
                ep_addr: 0x81,
                max_packet: 8,
                interval: 4,
            })
        );
    }

    #[test]
    fn classify_keeps_interface_zero_when_a_later_interface_is_also_a_keyboard() {
        let config = [
            9u8, 2, 57, 0, 3, 1, 0, 0xa0, 50, // configuration, three interfaces
            9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0, boot keyboard
            7, 5, 0x81, 3, 8, 0, 4, // interrupt IN
            9, 4, 1, 0, 1, 0xff, 0, 0, 0, // interface 1, vendor
            7, 5, 0x82, 3, 8, 0, 1, //
            9, 4, 2, 0, 1, 3, 1, 1, 0, // interface 2, also a boot keyboard
            7, 5, 0x83, 3, 8, 0, 8, //
        ];
        let found = classify_keyboard(0x04d9, 0x0001, &config).unwrap();
        assert_eq!(
            found,
            KeyboardChoice::InterfaceZero(BootKeyboard {
                config_value: 1,
                interface: 0,
                ep_addr: 0x81,
                max_packet: 8,
                interval: 4,
            })
        );
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
