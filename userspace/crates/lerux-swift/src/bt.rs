//! USB HCI selection and the HCI Reset acknowledgement.
//!
//! Wi-Fi firmware and Bluetooth firmware stay separate. A command-complete
//! event is the only path to `bt: reset ok`.

pub const HCI_RESET: [u8; 4] = [0x01, 0x03, 0x0c, 0x00];

pub enum BtReport {
    Absent,
    FirmwareErr,
    ResetOk,
}

pub fn is_bluetooth_interface(class: u8, subclass: u8, protocol: u8) -> bool {
    class == 0xe0 && subclass == 0x01 && protocol == 0x01
}

/// Command-complete for Reset: event packet, event 0x0e, opcode 0x0c03, status 0.
pub fn acknowledge_reset(event: &[u8]) -> bool {
    event.len() >= 7
        && event[0] == 0x04
        && event[1] == 0x0e
        && event[3] >= 1
        && u16::from_le_bytes([event[4], event[5]]) == 0x0c03
        && event[6] == 0
}

pub fn hci_reset() -> [u8; 4] {
    HCI_RESET
}

/// Firmware must already be loaded. `event` is the controller's answer.
pub fn bring_up(controller_present: bool, firmware_loaded: bool, event: Option<&[u8]>) -> BtReport {
    if !controller_present {
        return BtReport::Absent;
    }
    if !firmware_loaded {
        return BtReport::FirmwareErr;
    }
    match event {
        Some(event) if acknowledge_reset(event) => BtReport::ResetOk,
        _ => BtReport::FirmwareErr,
    }
}

pub fn format_report(report: &BtReport) -> &'static [u8] {
    match report {
        BtReport::Absent => b"bt: absent",
        BtReport::FirmwareErr => b"bt: firmware err",
        BtReport::ResetOk => b"bt: reset ok",
    }
}

/// `bt: ver` and four hex digits. The protection domain paints this when the
/// capability word is outside the USB range.
pub fn version_line(version: u16, out: &mut [u8]) -> usize {
    let prefix = b"bt: ver ";
    if out.len() < prefix.len() + 4 {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    for (i, byte) in out[prefix.len()..prefix.len() + 4].iter_mut().enumerate() {
        let shift = (3 - i) * 4;
        let nibble = ((version >> shift) & 0xf) as u8;
        *byte = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
    }
    prefix.len() + 4
}

/// Walk a configuration descriptor for the Bluetooth interface.
pub fn find_bluetooth_interface(config: &[u8]) -> bool {
    if config.len() < 4 || config[1] != 0x02 {
        return false;
    }
    let total = u16::from_le_bytes([config[2], config[3]]) as usize;
    let mut at = 0;
    while at + 2 <= config.len() && at < total {
        let len = config[at] as usize;
        if len < 2 || at + len > config.len() {
            return false;
        }
        if config[at + 1] == 0x04
            && len >= 9
            && is_bluetooth_interface(config[at + 5], config[at + 6], config[at + 7])
        {
            return true;
        }
        at += len;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_is_acknowledged_only_after_firmware() {
        let event = [0x04, 0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
        assert!(acknowledge_reset(&event));
        assert_eq!(
            format_report(&bring_up(true, true, Some(&event))),
            b"bt: reset ok"
        );
        assert_eq!(
            format_report(&bring_up(true, false, Some(&event))),
            b"bt: firmware err"
        );
        assert_eq!(hci_reset(), [0x01, 0x03, 0x0c, 0x00]);
    }

    #[test]
    fn version_line_is_four_hex_digits() {
        let mut line = [0u8; 16];
        let n = version_line(0xffff, &mut line);
        assert_eq!(&line[..n], b"bt: ver ffff");
        let n = version_line(0x0110, &mut line);
        assert_eq!(&line[..n], b"bt: ver 0110");
        assert_eq!(version_line(0x0100, &mut [0u8; 4]), 0);
    }

    #[test]
    fn selects_the_hci_interface() {
        let mut config = [0u8; 18];
        config[0] = 9;
        config[1] = 2;
        config[2] = 18;
        config[9] = 9;
        config[10] = 4;
        config[14] = 0xe0;
        config[15] = 0x01;
        config[16] = 0x01;
        assert!(find_bluetooth_interface(&config));
        config[14] = 0x03;
        assert!(!find_bluetooth_interface(&config));
    }
}
