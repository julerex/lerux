//! WPA2-PSK association. The metal path does not send the scripted Ethernet EAPOL.
//!
//! DHCP and ICMP belong to the shell. This driver keeps per-frame CCMP, the
//! 802.11 to Ethernet translation, and an empty protected frame so the
//! association stays up.

use lerux_swift::{
    mt7921::{
        frame_off, is_rx_frame, read_scan_done, write_survey_list, write_survey_mark, AuthMiss,
        Stage, SurveyCounts, SCAN_MAX,
    },
    wpa::{
        assoc_aid, assoc_ok, auth_ok, beacon_ssid, ccmp_decrypt, derive_ptk, eapol_in_msdu,
        match_beacon, message1, message2, message3, message4, pmk, write_assoc, write_auth,
        write_qos_data, FoundAp,
    },
};

use crate::{
    device::{assoc_budget, drain_budget, scan_budget, Mac},
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
        // The boot that stopped on `auth` had found the network. That word was
        // both a transmit ring that did not take the frame and a wait that
        // saw no matching authentication reply.
        paint(Stage::AuthTx);
        if !self.add_peer(ap.bssid, ap.channel > 14) || !self.post_auth(&ap) {
            return Stage::AuthTx;
        }
        paint(Stage::AuthRx);
        if !self.recv_auth() {
            return Stage::AuthRx;
        }
        paint(Stage::Assoc);
        let Some(aid) = self.request_assoc(&ap, ssid) else {
            return Stage::Assoc;
        };
        if !self.note_assoc(ap.bssid, ap.channel, ap.channel > 14, aid) {
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

    /// Directed scan. The access point is returned only after scan-done.
    ///
    /// The boot that stayed on `auth rx` posted authentication as soon as the
    /// beacon arrived. The firmware scan was still hopping, so the descriptor
    /// completed on a channel the access point was not using.
    fn find_ap(&mut self, ssid: &[u8]) -> Option<FoundAp> {
        if !self.start_scan(ssid) {
            return None;
        }
        let mut frame = [0u8; 1600];
        let mut found = None;
        let mut done = false;
        for _ in 0..scan_budget() {
            if found.is_none()
                && let Some(n) = self.poll_data()
                && let Some(ap) = beacon_for(n, &mut frame, ssid)
            {
                found = Some(ap);
            }
            if let Some(n) = self.poll_mcu() {
                if read_scan_done(Mac::rx_bytes(n)).is_some() {
                    done = true;
                    break;
                }
                if found.is_none()
                    && is_rx_frame(Mac::rx_bytes(n))
                    && let Some(ap) = beacon_for(n, &mut frame, ssid)
                {
                    found = Some(ap);
                }
            }
            core::hint::spin_loop();
        }
        if !done {
            return None;
        }
        if found.is_some() {
            return found;
        }
        for _ in 0..drain_budget() {
            if let Some(n) = self.poll_frame()
                && let Some(ap) = beacon_for(n, &mut frame, ssid)
            {
                return Some(ap);
            }
            core::hint::spin_loop();
        }
        None
    }

    /// Next frame from the data ring or an MCU ring. Linux reads every
    /// receive ring by packet type. Other MCU packets are dropped here.
    fn poll_frame(&mut self) -> Option<usize> {
        if let Some(n) = self.poll_data() {
            return Some(n);
        }
        let n = self.poll_mcu()?;
        is_rx_frame(Mac::rx_bytes(n)).then_some(n)
    }

    /// Wildcard scan. Heard names are written for the shell, in byte order.
    ///
    /// A scan that does not start leaves the request tag, so the shell reports
    /// `wifi err` instead of an empty list. The mark under the list counts
    /// data-ring frames, MCU-ring frames, names kept, and whether the firmware
    /// signaled scan-done. `ch` and `bcn` are that event's completed channels
    /// and beacon count.
    pub(crate) fn survey(&mut self, page: &mut [u8]) {
        if !self.start_scan(&[]) {
            return;
        }
        let mut heard = Heard::default();
        let mut frame = [0u8; 1600];
        self.wait_survey(&mut heard, &mut frame);
        let mut refs: [&[u8]; SCAN_MAX] = [&[]; SCAN_MAX];
        let listed = heard.names.iter().zip(heard.lens.iter()).take(heard.count);
        for (slot, (name, len)) in refs.iter_mut().zip(listed) {
            let n = usize::from(*len);
            *slot = &name[..n];
        }
        let _ = write_survey_list(page, &refs[..heard.count]);
        let _ = write_survey_mark(page, &heard.mark);
    }

    fn wait_survey(&mut self, heard: &mut Heard, frame: &mut [u8]) {
        for _ in 0..scan_budget() {
            if let Some(n) = self.poll_data() {
                heard.data(n, frame);
            }
            if let Some(n) = self.poll_mcu() {
                if let Some(report) = read_scan_done(Mac::rx_bytes(n)) {
                    heard.mark.done = true;
                    heard.mark.channels = report.channels;
                    heard.mark.beacons = report.beacons;
                    break;
                }
                if is_rx_frame(Mac::rx_bytes(n)) {
                    heard.event(n, frame);
                }
            }
            core::hint::spin_loop();
        }
        if !heard.mark.done {
            return;
        }
        for _ in 0..drain_budget() {
            if let Some(n) = self.poll_data() {
                heard.data(n, frame);
            }
            if let Some(n) = self.poll_mcu()
                && is_rx_frame(Mac::rx_bytes(n))
            {
                heard.event(n, frame);
            }
            core::hint::spin_loop();
        }
    }

    fn post_auth(&mut self, ap: &FoundAp) -> bool {
        let sta = self.sta();
        let mut frame = [0u8; 64];
        let n = write_auth(&mut frame, &ap.bssid, &sta, self.next_seq_wifi());
        self.tx_80211(&frame[..n])
    }

    fn recv_auth(&mut self) -> bool {
        let sta = self.sta();
        let mut frame = [0u8; 1600];
        let mut miss = AuthMiss::default();
        for _ in 0..assoc_budget() {
            if let Some(n) = self.poll_data() {
                miss.rx = miss.rx.saturating_add(1);
                if copied_auth(n, &mut frame, &mut miss, sta) {
                    self.remember_auth(miss);
                    return true;
                }
            } else if let Some(n) = self.poll_mcu() {
                if is_rx_frame(Mac::rx_bytes(n)) {
                    miss.ev = miss.ev.saturating_add(1);
                    if copied_auth(n, &mut frame, &mut miss, sta) {
                        self.remember_auth(miss);
                        return true;
                    }
                } else {
                    miss.skip = miss.skip.saturating_add(1);
                }
            }
            core::hint::spin_loop();
        }
        self.remember_auth(miss);
        false
    }

    fn request_assoc(&mut self, ap: &FoundAp, ssid: &[u8]) -> Option<u16> {
        let sta = self.sta();
        let mut frame = [0u8; 160];
        let n = write_assoc(&mut frame, &ap.bssid, &sta, ssid, self.next_seq_wifi())?;
        if !self.tx_80211(&frame[..n]) {
            return None;
        }
        self.wait_assoc(sta)
    }

    fn wait_assoc(&mut self, sta: [u8; 6]) -> Option<u16> {
        let mut frame = [0u8; 1600];
        for _ in 0..assoc_budget() {
            if let Some(n) = self.poll_frame()
                && let Some(m) = copy_frame(n, &mut frame)
                && assoc_ok(&frame[..m], &sta)
            {
                return assoc_aid(&frame[..m]);
            }
            core::hint::spin_loop();
        }
        None
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

    fn wait_eapol(&mut self, sta: [u8; 6], tk: Option<&[u8; 16]>, out: &mut [u8]) -> Option<usize> {
        let mut frame = [0u8; 1600];
        let mut plain = [0u8; 400];
        for _ in 0..assoc_budget() {
            if let Some(n) = self.poll_frame()
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

#[derive(Default)]
struct Heard {
    names: [[u8; 32]; SCAN_MAX],
    lens: [u8; SCAN_MAX],
    count: usize,
    mark: SurveyCounts,
}

impl Heard {
    fn data(&mut self, slot: usize, frame: &mut [u8]) {
        self.mark.rx = self.mark.rx.saturating_add(1);
        self.keep(slot, frame);
    }

    fn event(&mut self, slot: usize, frame: &mut [u8]) {
        self.mark.ev = self.mark.ev.saturating_add(1);
        self.keep(slot, frame);
    }

    fn keep(&mut self, slot: usize, frame: &mut [u8]) {
        let Some(m) = copy_frame(slot, frame) else {
            return;
        };
        if self.mark.fc.is_none() && m > 0 {
            self.mark.fc = Some(frame[0]);
        }
        if let Some(ssid) = beacon_ssid(&frame[..m]) {
            self.mark.ok = self.mark.ok.saturating_add(1);
            self.count = remember(&mut self.names, &mut self.lens, self.count, ssid);
        }
    }
}

fn beacon_for(n: usize, frame: &mut [u8], ssid: &[u8]) -> Option<FoundAp> {
    let m = copy_frame(n, frame)?;
    match_beacon(&frame[..m], ssid)
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

fn copied_auth(n: usize, frame: &mut [u8], miss: &mut AuthMiss, sta: [u8; 6]) -> bool {
    let Some(m) = copy_frame(n, frame) else {
        return false;
    };
    miss.fc = Some(frame[0]);
    auth_ok(&frame[..m], &sta)
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
