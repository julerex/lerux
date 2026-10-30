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

/// USB control payload for HCI Reset. The H4 type byte is not on this pipe.
pub const USB_HCI_RESET: [u8; 3] = [0x03, 0x0c, 0x00];

pub fn usb_hci_reset() -> [u8; 3] {
    USB_HCI_RESET
}

/// Interrupt IN endpoint on the Bluetooth interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HciIntr {
    pub config_value: u8,
    pub interface: u8,
    pub ep_addr: u8,
    pub max_packet: u16,
    pub interval: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XferFail {
    Halt,
    /// How many ports reported a device. Zero means none did.
    Port(u8),
    /// Vendor, product, and the first interface class, or the device class.
    Id(u16, u16, u8),
    Event,
}

pub fn xfer_line(fail: &XferFail) -> &'static [u8] {
    match fail {
        XferFail::Halt => b"bt: halt",
        XferFail::Port(_) => b"bt: port",
        XferFail::Id(..) => b"bt: id",
        XferFail::Event => b"bt: event",
    }
}

/// Panel line for [`XferFail`], including the port count or the device id.
pub fn fail_line(fail: &XferFail, out: &mut [u8]) -> usize {
    match *fail {
        XferFail::Halt | XferFail::Event => {
            let text = xfer_line(fail);
            let n = text.len().min(out.len());
            out[..n].copy_from_slice(&text[..n]);
            n
        }
        XferFail::Port(count) => port_line(count, out),
        XferFail::Id(vendor, product, class) => id_line(vendor, product, class, out),
    }
}

fn port_line(count: u8, out: &mut [u8]) -> usize {
    let prefix = b"bt: port ";
    if out.len() < prefix.len() + 1 {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    let mut n = prefix.len();
    if count >= 10 {
        out[n] = b'0' + (count / 10);
        n += 1;
        if n >= out.len() {
            return prefix.len();
        }
    }
    out[n] = b'0' + (count % 10);
    n + 1
}

fn id_line(vendor: u16, product: u16, class: u8, out: &mut [u8]) -> usize {
    let prefix = b"bt: id ";
    if out.len() < prefix.len() + 4 + 1 + 4 + 1 + 2 {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    let mut n = prefix.len();
    n += write_hex16(&mut out[n..], vendor);
    out[n] = b':';
    n += 1;
    n += write_hex16(&mut out[n..], product);
    out[n] = b' ';
    n += 1;
    out[n] = hex(class >> 4);
    out[n + 1] = hex(class);
    n + 2
}

fn write_hex16(slot: &mut [u8], value: u16) -> usize {
    for (i, byte) in slot.iter_mut().take(4).enumerate() {
        let shift = (3 - i) * 4;
        *byte = hex((value >> shift) as u8);
    }
    4
}

fn hex(nibble: u8) -> u8 {
    let nibble = nibble & 0xf;
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

/// Prefer the attempt that got further: an event, then a device id, then a port, then a halt.
/// Two port counts keep the larger one. Two device ids keep the first.
pub fn further(left: XferFail, right: XferFail) -> XferFail {
    match (left, right) {
        (kept @ XferFail::Event, _) | (_, kept @ XferFail::Event) => kept,
        (kept @ XferFail::Id(..), _) => kept,
        (_, kept @ XferFail::Id(..)) => kept,
        (XferFail::Port(a), kept @ XferFail::Port(b)) if b > a => kept,
        (kept @ XferFail::Port(_), XferFail::Port(_)) => kept,
        (kept @ XferFail::Port(_), XferFail::Halt) => kept,
        (XferFail::Halt, kept @ XferFail::Port(_)) => kept,
        (XferFail::Halt, XferFail::Halt) => XferFail::Halt,
    }
}

/// Prefix a USB HCI event with the H4 event type so [`acknowledge_reset`] can read it.
pub fn frame_usb_event(usb: &[u8], out: &mut [u8]) -> usize {
    if usb.is_empty() || out.len() < 2 {
        return 0;
    }
    let n = usb.len().min(out.len() - 1);
    out[0] = 0x04;
    out[1..1 + n].copy_from_slice(&usb[..n]);
    1 + n
}

/// Class of the first interface descriptor, when the configuration names one.
pub fn first_interface_class(config: &[u8]) -> Option<u8> {
    if config.len() < 4 || config[1] != 0x02 {
        return None;
    }
    let total = u16::from_le_bytes([config[2], config[3]]) as usize;
    let mut at = 0;
    while at + 2 <= config.len() && at < total {
        let len = config[at] as usize;
        if len < 2 || at + len > config.len() {
            return None;
        }
        if config[at + 1] == 0x04 && len >= 6 {
            return Some(config[at + 5]);
        }
        at += len;
    }
    None
}

/// Last interrupt-IN endpoint on a Bluetooth HCI interface.
pub fn find_hci_interrupt(config: &[u8]) -> Option<HciIntr> {
    if config.len() < 4 || config[1] != 0x02 {
        return None;
    }
    let total = u16::from_le_bytes([config[2], config[3]]) as usize;
    let mut config_value = 0u8;
    let mut interface = None;
    let mut found = None;
    let mut at = 0;
    while at + 2 <= config.len() && at < total {
        let len = config[at] as usize;
        if len < 2 || at + len > config.len() {
            return found;
        }
        let kind = config[at + 1];
        if kind == 0x02 && len >= 6 {
            config_value = config[at + 5];
        } else if kind == 0x04 && len >= 9 {
            interface = is_bluetooth_interface(config[at + 5], config[at + 6], config[at + 7])
                .then_some(config[at + 2]);
        } else if kind == 0x05
            && len >= 7
            && let Some(iface) = interface
        {
            let addr = config[at + 2];
            let attr = config[at + 3];
            if addr & 0x80 != 0 && attr & 0x03 == 3 {
                let max_packet = u16::from(config[at + 4]) | (u16::from(config[at + 5]) << 8);
                found = Some(HciIntr {
                    config_value,
                    interface: iface,
                    ep_addr: addr,
                    max_packet,
                    interval: config[at + 6],
                });
                interface = None;
            }
        }
        at += len;
    }
    found
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
    fn usb_reset_event_is_the_h4_acknowledgement() {
        assert_eq!(usb_hci_reset(), [0x03, 0x0c, 0x00]);
        let usb = [0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
        let mut framed = [0u8; 8];
        let n = frame_usb_event(&usb, &mut framed);
        assert!(acknowledge_reset(&framed[..n]));
        assert_eq!(xfer_line(&XferFail::Halt), b"bt: halt");
        assert_eq!(xfer_line(&XferFail::Port(0)), b"bt: port");
        assert_eq!(xfer_line(&XferFail::Event), b"bt: event");
        let mut line = [0u8; 24];
        let n = fail_line(&XferFail::Port(0), &mut line);
        assert_eq!(&line[..n], b"bt: port 0");
        let n = fail_line(&XferFail::Port(12), &mut line);
        assert_eq!(&line[..n], b"bt: port 12");
        let n = fail_line(&XferFail::Id(0x0e8d, 0x7961, 0xe0), &mut line);
        assert_eq!(&line[..n], b"bt: id 0e8d:7961 e0");
        assert_eq!(fail_line(&XferFail::Port(0), &mut [0u8; 4]), 0);
        assert_eq!(
            further(XferFail::Halt, XferFail::Port(0)),
            XferFail::Port(0)
        );
        assert_eq!(
            further(XferFail::Port(0), XferFail::Port(2)),
            XferFail::Port(2)
        );
        assert_eq!(
            further(XferFail::Port(2), XferFail::Id(0x0e8d, 0x7961, 0xff)),
            XferFail::Id(0x0e8d, 0x7961, 0xff)
        );
        assert_eq!(
            further(
                XferFail::Id(0x0e8d, 0x7961, 0xff),
                XferFail::Id(0x1111, 0x2222, 0x00)
            ),
            XferFail::Id(0x0e8d, 0x7961, 0xff)
        );
        assert_eq!(
            further(XferFail::Id(0x0e8d, 0x7961, 0xff), XferFail::Event),
            XferFail::Event
        );
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

    #[test]
    fn selects_the_interrupt_endpoint() {
        let mut config = [0u8; 25];
        config[0] = 9;
        config[1] = 2;
        config[2] = 25;
        config[5] = 1;
        config[9] = 9;
        config[10] = 4;
        config[11] = 0;
        config[14] = 0xe0;
        config[15] = 0x01;
        config[16] = 0x01;
        config[18] = 7;
        config[19] = 5;
        config[20] = 0x81;
        config[21] = 3;
        config[22] = 16;
        config[24] = 1;
        let ep = find_hci_interrupt(&config).unwrap();
        assert_eq!(ep.config_value, 1);
        assert_eq!(ep.ep_addr, 0x81);
        assert_eq!(ep.max_packet, 16);
        assert_eq!(ep.interval, 1);
        config[14] = 0x03;
        assert!(find_hci_interrupt(&config).is_none());
    }
}
