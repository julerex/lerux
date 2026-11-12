//! WPA2-PSK association. The metal path does not send the scripted Ethernet EAPOL.
//!
//! DHCP and ICMP belong to the shell. This driver keeps per-frame CCMP, the
//! 802.11 to Ethernet translation, and an empty protected frame so the
//! association stays up.

use lerux_swift::{
    mt7921::{frame_off, write_survey_list, Stage, SCAN_DONE, SCAN_MAX},
    wpa::{
        assoc_ok, auth_ok, beacon_ssid, ccmp_decrypt, derive_ptk, eapol_in_msdu, match_beacon,
        message1, message2, message3, message4, pmk, write_assoc, write_auth, write_qos_data,
        FoundAp,
    },
};

use crate::{
    device::{assoc_budget, scan_budget, Mac},
    ether::{write_keepalive, Net, Note, TxFrame},
};

/// Quiet services between association keepalives.
const KEEP_EVERY: u32 = 32;

impl Mac {
    pub fn associate(&mut self, ssid: &[u8], psk: &[u8], mut paint: impl FnMut(Stage)) -> Stage {
        paint(Stage::Scan);
        let Some(ap) = self.find_ap(ssid) else {
            return Stage::Scan;
        };
        let band = u8::from(ap.channel > 14);
        if !self.set_channel(ap.channel, band) {
            return Stage::Scan;
        }
        paint(Stage::Auth);
        if !self.open_auth(&ap) {
            return Stage::Auth;
        }
        paint(Stage::Assoc);
        if !self.request_assoc(&ap, ssid) {
            return Stage::Assoc;
        }
        paint(Stage::Eapol);
        let Some(tk) = self.four_way(&ap, ssid, psk) else {
            return Stage::Eapol;
        };
        self.open_link(ap.bssid, tk);
        self.send_keepalive();
        Stage::Wait
    }

    /// Move Ethernet frames between the radio and the standard network queues.
    ///
    /// The queue pages are not loaded until this runs, and this runs only after
    /// the handshake. The firmware download never reaches it.
    pub(crate) fn pump(&mut self, net: &Net) -> Note {
        if self.link().is_none() {
            return Note::quiet();
        }
        let rx = self.receive(net);
        let tx = self.transmit(net);
        self.keepalive_if_idle(rx || tx);
        Note {
            rx: rx && net.note_rx(),
            tx: tx && net.note_tx(),
        }
    }

    fn find_ap(&mut self, ssid: &[u8]) -> Option<FoundAp> {
        if !self.start_scan(ssid) {
            return None;
        }
        let mut frame = [0u8; 1600];
        for _ in 0..scan_budget() {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(n, &mut frame)
                && let Some(ap) = match_beacon(&frame[..m], ssid)
            {
                return Some(ap);
            }
            if let Some(n) = self.poll_mcu()
                && scan_done(Mac::rx_bytes(n))
            {
                return None;
            }
            core::hint::spin_loop();
        }
        None
    }

    /// Wildcard scan. Heard names are written for the shell, in byte order.
    ///
    /// A scan that does not start leaves the request tag, so the shell reports
    /// `wifi err` instead of an empty list.
    pub(crate) fn survey(&mut self, page: &mut [u8]) {
        if !self.start_scan(&[]) {
            return;
        }
        let mut names = [[0u8; 32]; SCAN_MAX];
        let mut lens = [0u8; SCAN_MAX];
        let mut count = 0usize;
        let mut frame = [0u8; 1600];
        for _ in 0..scan_budget() {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(n, &mut frame)
                && let Some(ssid) = beacon_ssid(&frame[..m])
            {
                count = remember(&mut names, &mut lens, count, ssid);
            }
            if let Some(n) = self.poll_mcu()
                && scan_done(Mac::rx_bytes(n))
            {
                break;
            }
            core::hint::spin_loop();
        }
        let mut refs: [&[u8]; SCAN_MAX] = [&[]; SCAN_MAX];
        for i in 0..count {
            let n = usize::from(lens[i]);
            refs[i] = &names[i][..n];
        }
        let _ = write_survey_list(page, &refs[..count]);
    }

    fn open_auth(&mut self, ap: &FoundAp) -> bool {
        let sta = self.sta();
        let mut frame = [0u8; 64];
        let n = write_auth(&mut frame, &ap.bssid, &sta, self.next_seq_wifi());
        if !self.tx_80211(&frame[..n]) {
            return false;
        }
        self.wait_frame(assoc_budget(), |got| auth_ok(got, &sta))
    }

    fn request_assoc(&mut self, ap: &FoundAp, ssid: &[u8]) -> bool {
        let sta = self.sta();
        let mut frame = [0u8; 160];
        let Some(n) = write_assoc(&mut frame, &ap.bssid, &sta, ssid, self.next_seq_wifi()) else {
            return false;
        };
        if !self.tx_80211(&frame[..n]) {
            return false;
        }
        self.wait_frame(assoc_budget(), |got| assoc_ok(got, &sta))
    }

    fn four_way(&mut self, ap: &FoundAp, ssid: &[u8], psk: &[u8]) -> Option<[u8; 16]> {
        let sta = self.sta();
        let mut eapol = [0u8; 256];
        let m = self.wait_eapol(sta, None, &mut eapol)?;
        let (anonce, replay) = message1(&eapol[..m])?;
        let key = pmk(psk, ssid);
        let snonce = snonce();
        let ptk = derive_ptk(&key, &ap.bssid, &sta, &anonce, &snonce);
        let n = message2(&mut eapol, &anonce, &snonce, &replay, &ptk.kck)?;
        self.send_eapol(ap.bssid, sta, &eapol[..n])?;
        let m = self.wait_eapol(sta, None, &mut eapol)?;
        let replay = message3(&eapol[..m], &ptk.kck)?;
        let n = message4(&mut eapol, &replay, &ptk.kck)?;
        self.send_eapol(ap.bssid, sta, &eapol[..n])?;
        Some(ptk.tk)
    }

    fn receive(&mut self, net: &Net) -> bool {
        let mut moved = false;
        let mut frame = [0u8; 1600];
        while net.rx_has_room() {
            let Some(n) = self.poll_data() else {
                break;
            };
            let Some(m) = copy_frame(n, &mut frame) else {
                continue;
            };
            let queued = self
                .link()
                .is_some_and(|link| net.enqueue_received(link, &frame[..m]));
            if queued {
                moved = true;
            }
        }
        moved
    }

    fn transmit(&mut self, net: &Net) -> bool {
        let mut moved = false;
        let mut wifi = [0u8; 1600];
        loop {
            let taken = {
                let Some(link) = self.link() else {
                    break;
                };
                net.dequeue_transmit(link, &mut wifi)
            };
            let n = match taken {
                TxFrame::Empty => break,
                TxFrame::Dropped => 0,
                TxFrame::Wifi(n) => n,
            };
            moved = true;
            if n > 0 {
                let _ = self.tx_80211(&wifi[..n]);
            }
        }
        moved
    }

    fn keepalive_if_idle(&mut self, busy: bool) {
        if self.keepalive_due(busy) {
            self.send_keepalive();
        }
    }

    fn keepalive_due(&mut self, busy: bool) -> bool {
        let Some(link) = self.link() else {
            return false;
        };
        if busy {
            link.quiet = 0;
            return false;
        }
        link.quiet = link.quiet.wrapping_add(1);
        if link.quiet < KEEP_EVERY {
            return false;
        }
        link.quiet = 0;
        true
    }

    fn send_keepalive(&mut self) {
        let mut frame = [0u8; 64];
        let n = {
            let Some(link) = self.link() else {
                return;
            };
            let Some(n) = write_keepalive(link, &mut frame) else {
                return;
            };
            n
        };
        let _ = self.tx_80211(&frame[..n]);
    }

    fn wait_frame(&mut self, spins: u32, mut pred: impl FnMut(&[u8]) -> bool) -> bool {
        let mut frame = [0u8; 1600];
        for _ in 0..spins {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(n, &mut frame)
                && pred(&frame[..m])
            {
                return true;
            }
            let _ = self.poll_mcu();
            core::hint::spin_loop();
        }
        false
    }

    fn wait_eapol(&mut self, sta: [u8; 6], tk: Option<&[u8; 16]>, out: &mut [u8]) -> Option<usize> {
        let mut frame = [0u8; 1600];
        let mut plain = [0u8; 400];
        for _ in 0..assoc_budget() {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(n, &mut frame)
                && frame.get(4..10) == Some(&sta[..])
                && let Some(body) = msdu(&frame[..m], tk, &mut plain)
                && let Some(eapol) = eapol_in_msdu(body)
                && eapol.len() <= out.len()
            {
                let n = eapol.len();
                out[..n].copy_from_slice(eapol);
                return Some(n);
            }
            let _ = self.poll_mcu();
            core::hint::spin_loop();
        }
        None
    }

    fn send_eapol(&mut self, bssid: [u8; 6], sta: [u8; 6], eapol: &[u8]) -> Option<()> {
        let mut llc = [0u8; 280];
        if eapol.len() + 8 > llc.len() {
            return None;
        }
        llc[..6].copy_from_slice(&[0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00]);
        llc[6] = 0x88;
        llc[7] = 0x8e;
        llc[8..8 + eapol.len()].copy_from_slice(eapol);
        self.send_plain(bssid, sta, bssid, &llc[..8 + eapol.len()])
    }

    fn send_plain(&mut self, bssid: [u8; 6], sta: [u8; 6], da: [u8; 6], msdu: &[u8]) -> Option<()> {
        let mut frame = [0u8; 400];
        let n = write_qos_data(
            frame.as_mut_slice(),
            &bssid,
            &sta,
            &da,
            msdu,
            self.next_seq_wifi(),
            false,
        )?;
        self.tx_80211(&frame[..n]).then_some(())
    }
}

fn remember(names: &mut [[u8; 32]], lens: &mut [u8], count: usize, ssid: &[u8]) -> usize {
    if ssid.is_empty() || ssid.len() > 32 || count >= names.len() {
        return count;
    }
    for i in 0..count {
        if usize::from(lens[i]) == ssid.len() && names[i][..ssid.len()] == *ssid {
            return count;
        }
    }
    names[count][..ssid.len()].copy_from_slice(ssid);
    lens[count] = u8::try_from(ssid.len()).expect("ssid length");
    count + 1
}

fn scan_done(bytes: &[u8]) -> bool {
    let Some(word) = bytes.get(..4) else {
        return false;
    };
    let Ok(word) = word.try_into() else {
        return false;
    };
    let kind = u32::from_le_bytes(word) >> 27;
    kind == 7 && bytes.get(28) == Some(&SCAN_DONE)
}

fn copy_frame(n: usize, out: &mut [u8]) -> Option<usize> {
    let bytes = Mac::rx_bytes(n);
    let off = frame_off(bytes)?;
    let frame = bytes.get(off..)?;
    let m = frame.len().min(out.len());
    out[..m].copy_from_slice(&frame[..m]);
    Some(m)
}

fn msdu<'a>(frame: &[u8], tk: Option<&[u8; 16]>, out: &'a mut [u8]) -> Option<&'a [u8]> {
    if frame.len() < 24 {
        return None;
    }
    let hdr = if frame[0] & 0x0c == 0x08 && frame[0] & 0x80 != 0 {
        26
    } else {
        24
    };
    if frame[1] & 0x40 != 0 {
        let n = ccmp_decrypt(tk?, hdr, frame, out)?;
        Some(&out[..n])
    } else {
        let n = frame.len().saturating_sub(hdr).min(out.len());
        out[..n].copy_from_slice(&frame[hdr..hdr + n]);
        Some(&out[..n])
    }
}

fn snonce() -> [u8; 32] {
    let tick = tick();
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&tick.to_le_bytes());
    let mixed = tick.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    out[8..16].copy_from_slice(&mixed.to_le_bytes());
    out
}

fn tick() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
    #[cfg(not(target_arch = "x86_64"))]
    0x4c52_5558
}
