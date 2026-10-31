//! WPA2-PSK association. The metal path does not send the scripted Ethernet EAPOL.

use lerux_swift::{
    dhcp_router, dhcp_yiaddr, icmp_reply,
    mt7921::{frame_off, Stage, SCAN_DONE},
    wpa::{
        assoc_ok, auth_ok, ccmp_decrypt, ccmp_encrypt, derive_ptk, eapol_in_msdu,
        ethernet_from_msdu, llc_from_ethernet, match_beacon, message1, message2, message3,
        message4, pmk, write_assoc, write_auth, write_qos_data, FoundAp,
    },
    write_discover, write_echo, write_request,
};

use crate::device::{assoc_budget, scan_budget, Mac};

const BCAST: [u8; 6] = [0xff; 6];

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
        paint(Stage::Dhcp);
        let Some((addr, gateway)) = self.lease(&ap, &tk) else {
            return Stage::Dhcp;
        };
        paint(Stage::Ping);
        if !self.echo(&ap, &tk, addr, gateway) {
            return Stage::Ping;
        }
        Stage::PingOk(addr)
    }

    fn find_ap(&mut self, ssid: &[u8]) -> Option<FoundAp> {
        if !self.start_scan(ssid) {
            return None;
        }
        let mut frame = [0u8; 1600];
        for _ in 0..scan_budget() {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(self, n, &mut frame)
                && let Some(ap) = match_beacon(&frame[..m], ssid)
            {
                return Some(ap);
            }
            if let Some(n) = self.poll_mcu()
                && scan_done(self.rx_bytes(n))
            {
                return None;
            }
            core::hint::spin_loop();
        }
        None
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
        let m = self.wait_eapol(&sta, None, &mut eapol)?;
        let (anonce, replay) = message1(&eapol[..m])?;
        let key = pmk(psk, ssid);
        let snonce = snonce();
        let ptk = derive_ptk(&key, &ap.bssid, &sta, &anonce, &snonce);
        let n = message2(&mut eapol, &anonce, &snonce, &replay, &ptk.kck)?;
        self.send_eapol(&ap.bssid, &sta, &eapol[..n])?;
        let m = self.wait_eapol(&sta, None, &mut eapol)?;
        let replay = message3(&eapol[..m], &ptk.kck)?;
        let n = message4(&mut eapol, &replay, &ptk.kck)?;
        self.send_eapol(&ap.bssid, &sta, &eapol[..n])?;
        Some(ptk.tk)
    }

    fn lease(&mut self, ap: &FoundAp, tk: &[u8; 16]) -> Option<([u8; 4], [u8; 4])> {
        let sta = self.sta();
        let mut ether = [0u8; 400];
        let n = write_discover(&mut ether, &sta);
        self.send_ether(&ap.bssid, &sta, tk, &ether[..n])?;
        let (yiaddr, server) = self.wait_dhcp(&sta, tk, 2, &mut ether)?;
        let n = write_request(&mut ether, &sta, yiaddr, server);
        self.send_ether(&ap.bssid, &sta, tk, &ether[..n])?;
        let (addr, _) = self.wait_dhcp(&sta, tk, 5, &mut ether)?;
        let gateway = dhcp_router(&ether).unwrap_or([addr[0], addr[1], addr[2], 1]);
        Some((addr, gateway))
    }

    fn echo(&mut self, ap: &FoundAp, tk: &[u8; 16], addr: [u8; 4], gateway: [u8; 4]) -> bool {
        let sta = self.sta();
        let mut ether = [0u8; 64];
        let n = write_echo(&mut ether, &sta, addr, gateway);
        if self.send_ether(&ap.bssid, &sta, tk, &ether[..n]).is_none() {
            return false;
        }
        let mut frame = [0u8; 1600];
        let mut plain = [0u8; 400];
        let mut rebuilt = [0u8; 400];
        for _ in 0..assoc_budget() {
            if let Some(len) = self.poll_data()
                && let Some(m) = copy_frame(self, len, &mut frame)
                && let Some(body) = msdu(&frame[..m], Some(tk), &mut plain)
                && let Some(en) = ethernet_from_msdu(body, &sta, &mut rebuilt)
                && icmp_reply(&rebuilt[..en])
            {
                return true;
            }
            let _ = self.poll_mcu();
            core::hint::spin_loop();
        }
        false
    }

    fn wait_frame(&mut self, spins: u32, mut pred: impl FnMut(&[u8]) -> bool) -> bool {
        let mut frame = [0u8; 1600];
        for _ in 0..spins {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(self, n, &mut frame)
                && pred(&frame[..m])
            {
                return true;
            }
            let _ = self.poll_mcu();
            core::hint::spin_loop();
        }
        false
    }

    fn wait_eapol(
        &mut self,
        sta: &[u8; 6],
        tk: Option<&[u8; 16]>,
        out: &mut [u8],
    ) -> Option<usize> {
        let mut frame = [0u8; 1600];
        let mut plain = [0u8; 400];
        for _ in 0..assoc_budget() {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(self, n, &mut frame)
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

    fn wait_dhcp(
        &mut self,
        sta: &[u8; 6],
        tk: &[u8; 16],
        kind: u8,
        ether: &mut [u8],
    ) -> Option<([u8; 4], [u8; 4])> {
        let mut frame = [0u8; 1600];
        let mut plain = [0u8; 400];
        for _ in 0..assoc_budget() {
            if let Some(n) = self.poll_data()
                && let Some(m) = copy_frame(self, n, &mut frame)
                && let Some(body) = msdu(&frame[..m], Some(tk), &mut plain)
                && let Some(en) = ethernet_from_msdu(body, sta, ether)
                && let Some(pair) = dhcp_yiaddr(&ether[..en], kind)
            {
                return Some(pair);
            }
            let _ = self.poll_mcu();
            core::hint::spin_loop();
        }
        None
    }

    fn send_eapol(&mut self, bssid: &[u8; 6], sta: &[u8; 6], eapol: &[u8]) -> Option<()> {
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

    fn send_ether(
        &mut self,
        bssid: &[u8; 6],
        sta: &[u8; 6],
        tk: &[u8; 16],
        ether: &[u8],
    ) -> Option<()> {
        let mut llc = [0u8; 400];
        let n = llc_from_ethernet(ether, &mut llc)?;
        let mut da = BCAST;
        if ether.len() >= 6 {
            da.copy_from_slice(&ether[..6]);
        }
        self.send_protected(bssid, sta, &da, tk, &llc[..n])
    }

    fn send_plain(
        &mut self,
        bssid: &[u8; 6],
        sta: &[u8; 6],
        da: &[u8; 6],
        msdu: &[u8],
    ) -> Option<()> {
        let mut frame = [0u8; 400];
        let n = write_qos_data(
            frame.as_mut_slice(),
            bssid,
            sta,
            da,
            msdu,
            self.next_seq_wifi(),
            false,
        )?;
        self.tx_80211(&frame[..n]).then_some(())
    }

    fn send_protected(
        &mut self,
        bssid: &[u8; 6],
        sta: &[u8; 6],
        da: &[u8; 6],
        tk: &[u8; 16],
        msdu: &[u8],
    ) -> Option<()> {
        let seq = self.next_seq_wifi();
        let mut hdr = [0u8; 26];
        write_qos_data(&mut hdr, bssid, sta, da, &[], seq, true)?;
        let mut body = [0u8; 500];
        let pn = self.next_pn();
        let clen = ccmp_encrypt(tk, &hdr, pn, msdu, &mut body)?;
        let mut frame = [0u8; 560];
        let n = write_qos_data(&mut frame, bssid, sta, da, &body[..clen], seq, true)?;
        self.tx_80211(&frame[..n]).then_some(())
    }
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

fn copy_frame(mac: &Mac, n: usize, out: &mut [u8]) -> Option<usize> {
    let bytes = mac.rx_bytes(n);
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
