//! WPA-Personal association, then DHCP and one ICMP echo.
//!
//! Frames that the radio accepts are also pushed onto an sDDF transmit queue.
//! A missing firmware blob or a radio that rejects the frame does not print
//! `ping ok`.

use lerux_sddf::{net_buff_desc_t, net_enqueue_active, net_queue_handle_t};

use crate::crypto::{hmac_md5, wpa_psk};

pub const CLIENT_MAC: [u8; 6] = [0x02, 0x14, 0xc3, 0x79, 0x21, 0x01];

pub enum WifiReport {
    Absent,
    FirmwareErr,
    KeyErr,
    RadioErr,
    /// The header BAR is not the address this image maps.
    Bar,
    PingOk([u8; 4]),
}

pub trait Nic {
    fn present(&self) -> bool;
    fn firmware(&self) -> bool;
    fn tx(&mut self, frame: &[u8]) -> bool;
    fn rx(&mut self) -> Option<&[u8]>;
}

/// `ssid=` and `psk=` lines. The file stays out of git.
pub fn credentials(text: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut ssid = None;
    let mut psk = None;
    for line in text.split(|b| *b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if let Some(rest) = line.strip_prefix(b"ssid=") {
            ssid = Some(rest);
        } else if let Some(rest) = line.strip_prefix(b"psk=") {
            psk = Some(rest);
        }
    }
    match (ssid, psk) {
        (Some(ssid), Some(psk)) if !ssid.is_empty() && !psk.is_empty() => Some((ssid, psk)),
        _ => None,
    }
}

/// Copy `frame` to the start of `storage` and enqueue it active.
///
/// # Safety
///
/// `queue` must be a live sDDF transmit queue. `storage` is the buffer the
/// descriptor's offset 0 refers to.
pub unsafe fn enqueue_frame(queue: &net_queue_handle_t, frame: &[u8], storage: &mut [u8]) -> i32 {
    if frame.is_empty() || frame.len() > storage.len() || frame.len() > u16::MAX as usize {
        return -1;
    }
    storage[..frame.len()].copy_from_slice(frame);
    unsafe { net_enqueue_active(queue, net_buff_desc_t::new(0, frame.len() as u16, 0)) }
}

pub fn bring_up(nic: &mut impl Nic, ssid: &[u8], pass: &[u8]) -> WifiReport {
    if !nic.present() {
        return WifiReport::Absent;
    }
    if !nic.firmware() {
        return WifiReport::FirmwareErr;
    }
    if ssid.is_empty() || pass.is_empty() {
        return WifiReport::KeyErr;
    }
    let key = wpa_psk(pass, ssid);
    let mut eapol = [0u8; 64];
    let eapol_len = write_eapol(&mut eapol, &key);
    if !nic.tx(&eapol[..eapol_len]) {
        return WifiReport::RadioErr;
    }
    if nic.rx() != Some(&[1]) {
        return WifiReport::RadioErr;
    }
    let mut discover = [0u8; 342];
    let discover_len = write_discover(&mut discover, &CLIENT_MAC);
    if !nic.tx(&discover[..discover_len]) {
        return WifiReport::RadioErr;
    }
    let Some(offer) = nic.rx() else {
        return WifiReport::RadioErr;
    };
    let Some((yiaddr, server)) = dhcp_yiaddr(offer, 2) else {
        return WifiReport::RadioErr;
    };
    let mut request = [0u8; 342];
    let request_len = write_request(&mut request, &CLIENT_MAC, yiaddr, server);
    if !nic.tx(&request[..request_len]) {
        return WifiReport::RadioErr;
    }
    let Some(ack) = nic.rx() else {
        return WifiReport::RadioErr;
    };
    let Some((addr, _)) = dhcp_yiaddr(ack, 5) else {
        return WifiReport::RadioErr;
    };
    let gateway = dhcp_router(ack).unwrap_or([addr[0], addr[1], addr[2], 1]);
    let mut echo = [0u8; 64];
    let echo_len = write_echo(&mut echo, &CLIENT_MAC, addr, gateway);
    if !nic.tx(&echo[..echo_len]) {
        return WifiReport::RadioErr;
    }
    let Some(reply) = nic.rx() else {
        return WifiReport::RadioErr;
    };
    if icmp_reply(reply) {
        WifiReport::PingOk(addr)
    } else {
        WifiReport::RadioErr
    }
}

pub fn format_report(report: &WifiReport, out: &mut [u8]) -> usize {
    let text: &[u8] = match report {
        WifiReport::Absent => b"mt7921: absent",
        WifiReport::FirmwareErr => b"mt7921: firmware err",
        WifiReport::KeyErr => b"mt7921: key err",
        WifiReport::RadioErr => b"mt7921: radio err",
        WifiReport::Bar => b"mt7921: bar",
        WifiReport::PingOk(addr) => {
            return write_ping(out, *addr);
        }
    };
    let n = text.len().min(out.len());
    out[..n].copy_from_slice(&text[..n]);
    n
}

fn write_ping(out: &mut [u8], addr: [u8; 4]) -> usize {
    let mut line = [0u8; 32];
    let mut n = 0;
    line[n..n + 3].copy_from_slice(b"ip ");
    n += 3;
    n += write_ip(&mut line[n..], addr);
    line[n..n + 8].copy_from_slice(b" ping ok");
    n += 8;
    let n = n.min(out.len());
    out[..n].copy_from_slice(&line[..n]);
    n
}

fn write_ip(out: &mut [u8], addr: [u8; 4]) -> usize {
    let mut n = 0;
    for (i, octet) in addr.iter().enumerate() {
        if i > 0 {
            out[n] = b'.';
            n += 1;
        }
        n += write_u8(&mut out[n..], *octet);
    }
    n
}

fn write_u8(out: &mut [u8], value: u8) -> usize {
    let mut tmp = [0u8; 3];
    let mut n = 0;
    let mut rest = value;
    if rest == 0 {
        out[0] = b'0';
        return 1;
    }
    while rest > 0 {
        tmp[n] = b'0' + rest % 10;
        n += 1;
        rest /= 10;
    }
    for i in 0..n {
        out[i] = tmp[n - 1 - i];
    }
    n
}

fn write_eapol(out: &mut [u8], psk: &[u8; 32]) -> usize {
    out[..6].fill(0xff);
    out[6..12].copy_from_slice(&CLIENT_MAC);
    out[12..14].copy_from_slice(&0x888eu16.to_be_bytes());
    out[14] = 1;
    out[15] = 3;
    out[16..18].copy_from_slice(&32u16.to_be_bytes());
    out[18..50].fill(0);
    let mic = hmac_md5(&psk[..16], &out[14..50]);
    out[18..34].copy_from_slice(&mic);
    50
}

pub fn write_discover(out: &mut [u8], mac: &[u8; 6]) -> usize {
    write_dhcp(out, mac, [0; 4], [0; 4], 1)
}

pub fn write_request(out: &mut [u8], mac: &[u8; 6], yiaddr: [u8; 4], server: [u8; 4]) -> usize {
    write_dhcp(out, mac, yiaddr, server, 3)
}

fn write_dhcp(out: &mut [u8], mac: &[u8; 6], yiaddr: [u8; 4], server: [u8; 4], kind: u8) -> usize {
    out[..6].fill(0xff);
    out[6..12].copy_from_slice(mac);
    out[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    out[14] = 0x45;
    out[15] = 0;
    let udp_len = 8 + 240 + 16;
    let ip_len = 20 + udp_len;
    out[16..18].copy_from_slice(&(ip_len as u16).to_be_bytes());
    out[18..24].fill(0);
    out[22] = 64;
    out[23] = 17;
    out[24..26].fill(0);
    out[26..30].fill(0);
    out[30..34].fill(0xff);
    let sum = ip_checksum(&out[14..34]);
    out[24..26].copy_from_slice(&sum.to_be_bytes());
    out[34..36].copy_from_slice(&68u16.to_be_bytes());
    out[36..38].copy_from_slice(&67u16.to_be_bytes());
    out[38..40].copy_from_slice(&(udp_len as u16).to_be_bytes());
    out[40..42].fill(0);
    let boot = &mut out[42..];
    boot[..240].fill(0);
    boot[0] = 1;
    boot[1] = 1;
    boot[2] = 6;
    boot[4..8].copy_from_slice(&0x4c52_5558u32.to_be_bytes());
    boot[16..20].copy_from_slice(&yiaddr);
    boot[28..34].copy_from_slice(mac);
    boot[236..240].copy_from_slice(&0x6382_5363u32.to_be_bytes());
    boot[240] = 53;
    boot[241] = 1;
    boot[242] = kind;
    let mut at = 243;
    if kind == 3 {
        boot[at] = 50;
        boot[at + 1] = 4;
        boot[at + 2..at + 6].copy_from_slice(&yiaddr);
        at += 6;
        boot[at] = 54;
        boot[at + 1] = 4;
        boot[at + 2..at + 6].copy_from_slice(&server);
        at += 6;
    }
    boot[at] = 255;
    42 + at + 1
}

#[cfg(test)]
fn dhcp_offer(addr: [u8; 4], server: [u8; 4]) -> [u8; 320] {
    let mut frame = [0u8; 320];
    frame[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    let boot = &mut frame[42..];
    boot[0] = 2;
    boot[16..20].copy_from_slice(&addr);
    boot[236..240].copy_from_slice(&0x6382_5363u32.to_be_bytes());
    boot[240] = 53;
    boot[241] = 1;
    boot[242] = 2;
    boot[243] = 54;
    boot[244] = 4;
    boot[245..249].copy_from_slice(&server);
    boot[249] = 3;
    boot[250] = 4;
    boot[251..255].copy_from_slice(&server);
    boot[255] = 255;
    frame
}

#[cfg(test)]
fn dhcp_ack(addr: [u8; 4], server: [u8; 4]) -> [u8; 320] {
    let mut frame = dhcp_offer(addr, server);
    frame[42 + 242] = 5;
    frame
}

pub fn dhcp_yiaddr(frame: &[u8], kind: u8) -> Option<([u8; 4], [u8; 4])> {
    if frame.len() < 42 + 243 || u16::from_be_bytes([frame[12], frame[13]]) != 0x0800 {
        return None;
    }
    let boot = &frame[42..];
    if boot[0] != 2
        || u32::from_be_bytes([boot[236], boot[237], boot[238], boot[239]]) != 0x6382_5363
    {
        return None;
    }
    if boot[240] != 53 || boot[242] != kind {
        return None;
    }
    let mut addr = [0u8; 4];
    addr.copy_from_slice(&boot[16..20]);
    let mut server = [0u8; 4];
    if boot[243] == 54 && boot[244] == 4 {
        server.copy_from_slice(&boot[245..249]);
    }
    Some((addr, server))
}

pub fn dhcp_router(frame: &[u8]) -> Option<[u8; 4]> {
    let boot = frame.get(42..)?;
    let mut at = 240;
    while at + 1 < boot.len() {
        let code = boot[at];
        if code == 255 {
            return None;
        }
        let len = boot[at + 1] as usize;
        if code == 3 && len == 4 && at + 6 <= boot.len() {
            let mut router = [0u8; 4];
            router.copy_from_slice(&boot[at + 2..at + 6]);
            return Some(router);
        }
        at += 2 + len;
    }
    None
}

pub fn write_echo(out: &mut [u8], mac: &[u8; 6], src: [u8; 4], dst: [u8; 4]) -> usize {
    out[..6].fill(0xff);
    out[6..12].copy_from_slice(mac);
    out[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    out[14] = 0x45;
    out[16..18].copy_from_slice(&28u16.to_be_bytes());
    out[22] = 64;
    out[23] = 1;
    out[26..30].copy_from_slice(&src);
    out[30..34].copy_from_slice(&dst);
    let sum = ip_checksum(&out[14..34]);
    out[24..26].copy_from_slice(&sum.to_be_bytes());
    out[34] = 8;
    out[36..38].copy_from_slice(&1u16.to_be_bytes());
    let icmp = icmp_checksum(&out[34..42]);
    out[36..38].copy_from_slice(&icmp.to_be_bytes());
    42
}

#[cfg(test)]
fn icmp_reply_frame(src: [u8; 4], dst: [u8; 4]) -> [u8; 42] {
    let mut frame = [0u8; 42];
    frame[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    frame[23] = 1;
    frame[26..30].copy_from_slice(&src);
    frame[30..34].copy_from_slice(&dst);
    frame[34] = 0;
    frame
}

pub fn icmp_reply(frame: &[u8]) -> bool {
    frame.len() >= 35 && frame.get(23) == Some(&1) && frame.get(34) == Some(&0)
}

fn ip_checksum(header: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 1 < header.len() {
        if i != 10 {
            sum += u32::from(u16::from_be_bytes([header[i], header[i + 1]]));
        }
        i += 2;
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn icmp_checksum(body: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 1 < body.len() {
        if i != 2 {
            sum += u32::from(u16::from_be_bytes([body[i], body[i + 1]]));
        }
        i += 2;
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lerux_sddf::{net_dequeue_active, net_queue_init};

    struct Script {
        psk: [u8; 32],
        pending: [u8; 320],
        pending_len: usize,
        queue_free: [u8; 512],
        queue_active: [u8; 512],
        storage: [u8; 2048],
        handle: net_queue_handle_t,
        frames: usize,
    }

    impl Script {
        fn new() -> Self {
            Self {
                psk: wpa_psk(b"password", b"IEEE"),
                pending: [0; 320],
                pending_len: 0,
                queue_free: [0; 512],
                queue_active: [0; 512],
                storage: [0; 2048],
                handle: net_queue_handle_t {
                    free: core::ptr::null_mut(),
                    active: core::ptr::null_mut(),
                    capacity: 0,
                },
                frames: 0,
            }
        }

        fn ensure_queue(&mut self) {
            if self.handle.capacity != 0 {
                return;
            }
            unsafe {
                net_queue_init(
                    &mut self.handle,
                    self.queue_free.as_mut_ptr().cast(),
                    self.queue_active.as_mut_ptr().cast(),
                    4,
                );
            }
        }
    }

    impl Nic for Script {
        fn present(&self) -> bool {
            true
        }
        fn firmware(&self) -> bool {
            true
        }
        fn tx(&mut self, frame: &[u8]) -> bool {
            self.ensure_queue();
            let queued = unsafe { enqueue_frame(&self.handle, frame, &mut self.storage) };
            if queued != 0 {
                return false;
            }
            self.frames += 1;
            if u16::from_be_bytes([frame[12], frame[13]]) == 0x888e {
                let mut zeroed = [0u8; 64];
                zeroed[..frame.len().min(64)].copy_from_slice(&frame[..frame.len().min(64)]);
                zeroed[18..34].fill(0);
                let mic = hmac_md5(&self.psk[..16], &zeroed[14..50]);
                if frame.len() >= 34 && frame[18..34] == mic {
                    self.pending[0] = 1;
                    self.pending_len = 1;
                }
                return true;
            }
            if frame.get(42 + 240) == Some(&53) {
                let kind = frame[42 + 242];
                if kind == 1 {
                    let offer = dhcp_offer([192, 168, 1, 20], [192, 168, 1, 1]);
                    self.pending = offer;
                    self.pending_len = 320;
                } else if kind == 3 {
                    let ack = dhcp_ack([192, 168, 1, 20], [192, 168, 1, 1]);
                    self.pending = ack;
                    self.pending_len = 320;
                }
                return true;
            }
            if frame.get(34) == Some(&8) {
                let reply = icmp_reply_frame([192, 168, 1, 1], [192, 168, 1, 20]);
                self.pending[..42].copy_from_slice(&reply);
                self.pending_len = 42;
            }
            true
        }
        fn rx(&mut self) -> Option<&[u8]> {
            if self.pending_len == 0 {
                return None;
            }
            let n = self.pending_len;
            self.pending_len = 0;
            Some(&self.pending[..n])
        }
    }

    #[test]
    fn personal_network_answers_ping() {
        let mut nic = Script::new();
        let report = bring_up(&mut nic, b"IEEE", b"password");
        match report {
            WifiReport::PingOk(addr) => assert_eq!(addr, [192, 168, 1, 20]),
            _ => panic!("ping did not complete"),
        }
        assert!(nic.frames >= 4);
        let mut desc = net_buff_desc_t::new(0, 0, 0);
        assert_eq!(unsafe { net_dequeue_active(&nic.handle, &mut desc) }, 0);
        assert!(desc.len > 0);
        let mut line = [0u8; 40];
        let n = format_report(&report, &mut line);
        assert_eq!(&line[..n], b"ip 192.168.1.20 ping ok");
    }

    #[test]
    fn missing_firmware_is_not_a_ping() {
        struct Bare;
        impl Nic for Bare {
            fn present(&self) -> bool {
                true
            }
            fn firmware(&self) -> bool {
                false
            }
            fn tx(&mut self, _: &[u8]) -> bool {
                true
            }
            fn rx(&mut self) -> Option<&[u8]> {
                None
            }
        }
        let mut line = [0u8; 40];
        let n = format_report(&bring_up(&mut Bare, b"IEEE", b"password"), &mut line);
        assert_eq!(&line[..n], b"mt7921: firmware err");
    }

    #[test]
    fn credentials_need_both_lines() {
        assert!(credentials(b"ssid=IEEE\npsk=password\n").is_some());
        assert!(credentials(b"ssid=IEEE\n").is_none());
    }
}
