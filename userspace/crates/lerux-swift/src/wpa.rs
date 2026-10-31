//! WPA2-PSK four-way handshake and CCMP for one station.
//!
//! The PMK is [`crate::crypto::wpa_psk`]. The MIC is HMAC-SHA1, descriptor
//! version 2. Frames the metal radio sends are 802.11, not the scripted
//! Ethernet EAPOL in `wifi`.

use crate::crypto::{hmac_sha1, wpa_psk};

pub const SSID_MAX: usize = 32;
pub const PSK_MIN: usize = 8;
pub const PSK_MAX: usize = 63;

const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

const RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

/// AES-128 encrypt of one block. FIPS-197.
pub fn aes128_encrypt_block(key: &[u8; 16], block: &mut [u8; 16]) {
    let mut round = [0u8; 176];
    expand(key, &mut round);
    add(block, &round[..16]);
    for r in 1..10 {
        sub(block);
        shift(block);
        mix(block);
        add(block, &round[r * 16..(r + 1) * 16]);
    }
    sub(block);
    shift(block);
    add(block, &round[160..176]);
}

fn expand(key: &[u8; 16], out: &mut [u8; 176]) {
    out[..16].copy_from_slice(key);
    let mut bytes = 16;
    let mut rcon = 1;
    let mut temp = [0u8; 4];
    while bytes < 176 {
        temp.copy_from_slice(&out[bytes - 4..bytes]);
        if bytes % 16 == 0 {
            let t = temp[0];
            temp[0] = SBOX[temp[1] as usize] ^ RCON[rcon];
            temp[1] = SBOX[temp[2] as usize];
            temp[2] = SBOX[temp[3] as usize];
            temp[3] = SBOX[t as usize];
            rcon += 1;
        }
        for slot in temp {
            out[bytes] = out[bytes - 16] ^ slot;
            bytes += 1;
        }
    }
}

fn add(block: &mut [u8; 16], round: &[u8]) {
    for i in 0..16 {
        block[i] ^= round[i];
    }
}

fn sub(block: &mut [u8; 16]) {
    for b in block.iter_mut() {
        *b = SBOX[*b as usize];
    }
}

fn shift(block: &mut [u8; 16]) {
    let t = *block;
    block[1] = t[5];
    block[5] = t[9];
    block[9] = t[13];
    block[13] = t[1];
    block[2] = t[10];
    block[6] = t[14];
    block[10] = t[2];
    block[14] = t[6];
    block[3] = t[15];
    block[7] = t[3];
    block[11] = t[7];
    block[15] = t[11];
}

fn xtime(x: u8) -> u8 {
    let x = u16::from(x) << 1;
    if x & 0x100 == 0 {
        x as u8
    } else {
        (x as u8) ^ 0x1b
    }
}

fn mix(block: &mut [u8; 16]) {
    for c in 0..4 {
        let i = c * 4;
        let a = block[i];
        let b = block[i + 1];
        let c0 = block[i + 2];
        let d = block[i + 3];
        let all = a ^ b ^ c0 ^ d;
        block[i] ^= all ^ xtime(a ^ b);
        block[i + 1] ^= all ^ xtime(b ^ c0);
        block[i + 2] ^= all ^ xtime(c0 ^ d);
        block[i + 3] ^= all ^ xtime(d ^ a);
    }
}

/// Pairwise transient key. CCMP uses 48 bytes: KCK, KEK, TK.
pub struct Ptk {
    pub kck: [u8; 16],
    pub kek: [u8; 16],
    pub tk: [u8; 16],
}

pub fn derive_ptk(
    pmk: &[u8; 32],
    ap: &[u8; 6],
    sta: &[u8; 6],
    anonce: &[u8; 32],
    snonce: &[u8; 32],
) -> Ptk {
    let mut label = [0u8; 128];
    let text = b"Pairwise key expansion";
    label[..text.len()].copy_from_slice(text);
    let mut n = text.len() + 1;
    let (min_mac, max_mac) = if ap <= sta { (ap, sta) } else { (sta, ap) };
    let (min_nonce, max_nonce) = if anonce <= snonce {
        (anonce, snonce)
    } else {
        (snonce, anonce)
    };
    label[n..n + 6].copy_from_slice(min_mac);
    n += 6;
    label[n..n + 6].copy_from_slice(max_mac);
    n += 6;
    label[n..n + 32].copy_from_slice(min_nonce);
    n += 32;
    label[n..n + 32].copy_from_slice(max_nonce);
    n += 32;
    let mut out = [0u8; 48];
    prf(pmk, &label[..n], &mut out);
    let mut kck = [0u8; 16];
    let mut kek = [0u8; 16];
    let mut tk = [0u8; 16];
    kck.copy_from_slice(&out[..16]);
    kek.copy_from_slice(&out[16..32]);
    tk.copy_from_slice(&out[32..48]);
    Ptk { kck, kek, tk }
}

fn prf(key: &[u8], prefix: &[u8], out: &mut [u8]) {
    let mut filled = 0;
    let mut counter = 0u8;
    while filled < out.len() {
        let mut block = [0u8; 160];
        let n = prefix.len().min(block.len() - 1);
        block[..n].copy_from_slice(&prefix[..n]);
        block[n] = counter;
        let mac = hmac_sha1(key, &block[..=n]);
        let take = (out.len() - filled).min(20);
        out[filled..filled + take].copy_from_slice(&mac[..take]);
        filled += take;
        counter = counter.wrapping_add(1);
    }
}

const KEY_PAIR: u16 = 1 << 3;
const KEY_INSTALL: u16 = 1 << 6;
const KEY_ACK: u16 = 1 << 7;
const KEY_MIC: u16 = 1 << 8;
const KEY_SECURE: u16 = 1 << 9;
const MIC_AT: usize = 81;

/// Message 1 of the four-way handshake: ANonce and the replay counter.
pub fn message1(eapol: &[u8]) -> Option<([u8; 32], [u8; 8])> {
    let info = key_info(eapol)?;
    if info & KEY_ACK == 0 || info & KEY_MIC != 0 || info & 7 != 2 {
        return None;
    }
    let mut anonce = [0u8; 32];
    let mut replay = [0u8; 8];
    anonce.copy_from_slice(&eapol[17..49]);
    replay.copy_from_slice(&eapol[9..17]);
    Some((anonce, replay))
}

/// Message 2. `out` receives the EAPOL key frame. The MIC uses `kck`.
pub fn message2(
    out: &mut [u8],
    anonce: &[u8; 32],
    snonce: &[u8; 32],
    replay: &[u8; 8],
    kck: &[u8; 16],
) -> Option<usize> {
    let n = write_eapol_key(out, KEY_PAIR | KEY_MIC, replay, snonce, &[])?;
    let _ = anonce;
    seal_mic(out, n, kck);
    Some(n)
}

/// Message 3 must carry a MIC that `kck` verifies. Returns the replay counter.
pub fn message3(eapol: &[u8], kck: &[u8; 16]) -> Option<[u8; 8]> {
    let info = key_info(eapol)?;
    if info & KEY_ACK == 0 || info & KEY_MIC == 0 || info & KEY_INSTALL == 0 {
        return None;
    }
    if !mic_ok(eapol, kck) {
        return None;
    }
    let mut replay = [0u8; 8];
    replay.copy_from_slice(&eapol[9..17]);
    Some(replay)
}

/// Message 4, the acknowledgement.
pub fn message4(out: &mut [u8], replay: &[u8; 8], kck: &[u8; 16]) -> Option<usize> {
    let n = write_eapol_key(out, KEY_PAIR | KEY_MIC | KEY_SECURE, replay, &[0; 32], &[])?;
    seal_mic(out, n, kck);
    Some(n)
}

fn key_info(eapol: &[u8]) -> Option<u16> {
    if eapol.len() < 99 || eapol[1] != 3 || eapol[4] != 2 {
        return None;
    }
    Some(u16::from_be_bytes([eapol[5], eapol[6]]))
}

fn write_eapol_key(
    out: &mut [u8],
    info: u16,
    replay: &[u8; 8],
    nonce: &[u8; 32],
    data: &[u8],
) -> Option<usize> {
    let n = 99 + data.len();
    if out.len() < n {
        return None;
    }
    out[..n].fill(0);
    out[0] = 1;
    out[1] = 3;
    let body = (n - 4) as u16;
    out[2..4].copy_from_slice(&body.to_be_bytes());
    out[4] = 2;
    out[5..7].copy_from_slice(&(info | 2).to_be_bytes());
    out[7..9].copy_from_slice(&16u16.to_be_bytes());
    out[9..17].copy_from_slice(replay);
    out[17..49].copy_from_slice(nonce);
    out[97..99].copy_from_slice(&(data.len() as u16).to_be_bytes());
    out[99..n].copy_from_slice(data);
    Some(n)
}

fn seal_mic(frame: &mut [u8], n: usize, kck: &[u8; 16]) {
    let mic = hmac_sha1(kck, &frame[..n]);
    frame[MIC_AT..MIC_AT + 16].copy_from_slice(&mic[..16]);
}

fn mic_ok(eapol: &[u8], kck: &[u8; 16]) -> bool {
    if eapol.len() < MIC_AT + 16 {
        return false;
    }
    let mut bare = [0u8; 512];
    if eapol.len() > bare.len() {
        return false;
    }
    bare[..eapol.len()].copy_from_slice(eapol);
    bare[MIC_AT..MIC_AT + 16].fill(0);
    let mic = hmac_sha1(kck, &bare[..eapol.len()]);
    mic[..16] == eapol[MIC_AT..MIC_AT + 16]
}

/// CCMP encrypt. `msdu` is the plaintext after the 802.11 header. `out` receives
/// the 8-byte CCMP header, the ciphertext, and the 8-byte MIC.
pub fn ccmp_encrypt(
    tk: &[u8; 16],
    hdr: &[u8],
    pn: u64,
    msdu: &[u8],
    out: &mut [u8],
) -> Option<usize> {
    let n = 8 + msdu.len() + 8;
    if out.len() < n || hdr.len() < 24 {
        return None;
    }
    write_ccmp_hdr(&mut out[..8], pn);
    let mut nonce = [0u8; 13];
    nonce[0] = qos_tid(hdr);
    nonce[1..7].copy_from_slice(&hdr[10..16]);
    for i in 0..6 {
        nonce[7 + i] = (pn >> (8 * (5 - i))) as u8;
    }
    let mut aad = [0u8; 32];
    let aad_len = ccmp_aad(hdr, &mut aad);
    let mut mic = [0u8; 8];
    ccm_mic(tk, &nonce, &aad[..aad_len], msdu, &mut mic);
    ctr(tk, &nonce, msdu, &mic, &mut out[8..n]);
    Some(n)
}

/// Inverse of [`ccmp_encrypt`]. `frame` is header || CCMP header || body || MIC.
/// The plaintext MSDU is written to `out`.
pub fn ccmp_decrypt(tk: &[u8; 16], hdr_len: usize, frame: &[u8], out: &mut [u8]) -> Option<usize> {
    if frame.len() < hdr_len + 16 || hdr_len < 24 {
        return None;
    }
    let body = &frame[hdr_len + 8..frame.len() - 8];
    let mic = &frame[frame.len() - 8..];
    if out.len() < body.len() {
        return None;
    }
    let pn = read_pn(&frame[hdr_len..hdr_len + 8]);
    let mut nonce = [0u8; 13];
    nonce[0] = qos_tid(&frame[..hdr_len]);
    nonce[1..7].copy_from_slice(&frame[10..16]);
    for i in 0..6 {
        nonce[7 + i] = (pn >> (8 * (5 - i))) as u8;
    }
    let mut aad = [0u8; 32];
    let aad_len = ccmp_aad(&frame[..hdr_len], &mut aad);
    let mut got_mic = [0u8; 8];
    ctr_dec(tk, &nonce, body, mic, out, &mut got_mic);
    let mut expect = [0u8; 8];
    ccm_mic(tk, &nonce, &aad[..aad_len], &out[..body.len()], &mut expect);
    if got_mic != expect {
        return None;
    }
    Some(body.len())
}

fn write_ccmp_hdr(out: &mut [u8], pn: u64) {
    out[0] = pn as u8;
    out[1] = (pn >> 8) as u8;
    out[2] = 0;
    out[3] = 0x20;
    out[4] = (pn >> 16) as u8;
    out[5] = (pn >> 24) as u8;
    out[6] = (pn >> 32) as u8;
    out[7] = (pn >> 40) as u8;
}

fn read_pn(hdr: &[u8]) -> u64 {
    u64::from(hdr[0])
        | u64::from(hdr[1]) << 8
        | u64::from(hdr[4]) << 16
        | u64::from(hdr[5]) << 24
        | u64::from(hdr[6]) << 32
        | u64::from(hdr[7]) << 40
}

fn qos_tid(hdr: &[u8]) -> u8 {
    if hdr.len() >= 26 && hdr[0] & 0x80 != 0 {
        hdr[24] & 0x0f
    } else {
        0
    }
}

fn ccmp_aad(hdr: &[u8], out: &mut [u8]) -> usize {
    let mut fc = u16::from_le_bytes([hdr[0], hdr[1]]);
    fc &= !0x0070;
    fc &= !(0x0800 | 0x1000 | 0x2000 | 0x4000);
    if hdr.len() < 26 {
        fc &= !0x8000;
    }
    out[0] = fc as u8;
    out[1] = (fc >> 8) as u8;
    out[2..8].copy_from_slice(&hdr[4..10]);
    out[8..14].copy_from_slice(&hdr[10..16]);
    out[14..20].copy_from_slice(&hdr[16..22]);
    let sc = u16::from_le_bytes([hdr[22], hdr[23]]) & 0x000f;
    out[20] = sc as u8;
    out[21] = 0;
    if hdr.len() >= 26 && hdr[0] & 0x80 != 0 {
        let mut qc = u16::from(hdr[24]);
        qc &= 0x000f;
        if hdr[24] & 0x80 != 0 {
            qc |= 0x0080;
        }
        out[22] = qc as u8;
        out[23] = 0;
        24
    } else {
        22
    }
}

fn ccm_mic(key: &[u8; 16], nonce: &[u8; 13], aad: &[u8], msdu: &[u8], mic: &mut [u8; 8]) {
    let mut block = [0u8; 16];
    block[0] = 0x59;
    block[1..14].copy_from_slice(nonce);
    block[14] = (msdu.len() >> 8) as u8;
    block[15] = msdu.len() as u8;
    aes128_encrypt_block(key, &mut block);
    let mut extra = [0u8; 16];
    extra[0] = (aad.len() >> 8) as u8;
    extra[1] = aad.len() as u8;
    let n = aad.len().min(14);
    extra[2..2 + n].copy_from_slice(&aad[..n]);
    xor_block(&mut block, &extra);
    aes128_encrypt_block(key, &mut block);
    let mut off = 14;
    while off < aad.len() {
        let mut chunk = [0u8; 16];
        let take = (aad.len() - off).min(16);
        chunk[..take].copy_from_slice(&aad[off..off + take]);
        xor_block(&mut block, &chunk);
        aes128_encrypt_block(key, &mut block);
        off += take;
    }
    off = 0;
    while off < msdu.len() {
        let mut chunk = [0u8; 16];
        let take = (msdu.len() - off).min(16);
        chunk[..take].copy_from_slice(&msdu[off..off + take]);
        xor_block(&mut block, &chunk);
        aes128_encrypt_block(key, &mut block);
        off += take;
    }
    mic.copy_from_slice(&block[..8]);
}

fn ctr(key: &[u8; 16], nonce: &[u8; 13], msdu: &[u8], mic: &[u8; 8], out: &mut [u8]) {
    let s0 = ctr_block(key, nonce, 0);
    let mut mic_mask = [0u8; 8];
    mic_mask.copy_from_slice(&s0[..8]);
    for (i, b) in mic.iter().enumerate() {
        out[msdu.len() + i] = b ^ mic_mask[i];
    }
    let mut off = 0;
    let mut counter = 1u16;
    while off < msdu.len() {
        let s = ctr_block(key, nonce, counter);
        let take = (msdu.len() - off).min(16);
        for i in 0..take {
            out[off + i] = msdu[off + i] ^ s[i];
        }
        off += take;
        counter = counter.wrapping_add(1);
    }
}

fn ctr_dec(
    key: &[u8; 16],
    nonce: &[u8; 13],
    body: &[u8],
    mic: &[u8],
    plain: &mut [u8],
    mic_out: &mut [u8; 8],
) {
    let s0 = ctr_block(key, nonce, 0);
    for i in 0..8 {
        mic_out[i] = mic[i] ^ s0[i];
    }
    let mut off = 0;
    let mut counter = 1u16;
    while off < body.len() {
        let s = ctr_block(key, nonce, counter);
        let take = (body.len() - off).min(16);
        for i in 0..take {
            plain[off + i] = body[off + i] ^ s[i];
        }
        off += take;
        counter = counter.wrapping_add(1);
    }
}

fn ctr_block(key: &[u8; 16], nonce: &[u8; 13], counter: u16) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[0] = 1;
    block[1..14].copy_from_slice(nonce);
    block[14] = (counter >> 8) as u8;
    block[15] = counter as u8;
    aes128_encrypt_block(key, &mut block);
    block
}

fn xor_block(block: &mut [u8; 16], other: &[u8; 16]) {
    for i in 0..16 {
        block[i] ^= other[i];
    }
}

pub fn pmk(pass: &[u8], ssid: &[u8]) -> [u8; 32] {
    wpa_psk(pass, ssid)
}

/// A beacon whose SSID matches. The channel comes from the DS parameter set.
pub struct FoundAp {
    pub bssid: [u8; 6],
    pub channel: u8,
}

pub fn match_beacon(frame: &[u8], ssid: &[u8]) -> Option<FoundAp> {
    if frame.len() < 36 || frame[0] & 0xfc != 0x80 {
        return None;
    }
    let mut bssid = [0u8; 6];
    bssid.copy_from_slice(&frame[16..22]);
    let mut ies = &frame[36..];
    let mut channel = 1u8;
    let mut saw = false;
    while ies.len() >= 2 {
        let tag = ies[0];
        let len = usize::from(ies[1]);
        if ies.len() < 2 + len {
            break;
        }
        let body = &ies[2..2 + len];
        if tag == 0 && body == ssid {
            saw = true;
        }
        if tag == 3 && body.len() == 1 {
            channel = body[0];
        }
        ies = &ies[2 + len..];
    }
    if !saw {
        return None;
    }
    Some(FoundAp { bssid, channel })
}

pub fn auth_ok(frame: &[u8], sta: &[u8; 6]) -> bool {
    frame.len() >= 30
        && frame[0] & 0xfc == 0xb0
        && frame[4..10] == sta[..]
        && frame[24] == 0
        && frame[25] == 0
        && frame[26] == 2
        && frame[28] == 0
        && frame[29] == 0
}

pub fn assoc_ok(frame: &[u8], sta: &[u8; 6]) -> bool {
    frame.len() >= 30 && frame[0] & 0xfc == 0x10 && frame[4..10] == sta[..] && frame[26] == 0
}

/// Open-system authentication, sequence 1.
pub fn write_auth(out: &mut [u8], bssid: &[u8; 6], sta: &[u8; 6], seq: u16) -> usize {
    write_mgmt(out, 0xb0, bssid, sta, bssid, seq);
    out[24] = 0;
    out[25] = 0;
    out[26] = 1;
    out[27] = 0;
    out[28] = 0;
    out[29] = 0;
    30
}

/// Association request carrying the SSID and a CCMP/PSK RSN element.
pub fn write_assoc(
    out: &mut [u8],
    bssid: &[u8; 6],
    sta: &[u8; 6],
    ssid: &[u8],
    seq: u16,
) -> Option<usize> {
    if ssid.len() > SSID_MAX || out.len() < 36 + ssid.len() + 24 {
        return None;
    }
    write_mgmt(out, 0x00, bssid, sta, bssid, seq);
    out[24] = 0x01;
    out[25] = 0x00;
    out[26] = 0x0a;
    out[27] = 0x00;
    let mut n = 28;
    out[n] = 0;
    out[n + 1] = ssid.len() as u8;
    out[n + 2..n + 2 + ssid.len()].copy_from_slice(ssid);
    n += 2 + ssid.len();
    out[n] = 1;
    out[n + 1] = 8;
    out[n + 2..n + 10].copy_from_slice(&[0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24]);
    n += 10;
    out[n] = 48;
    out[n + 1] = 20;
    out[n + 2..n + 4].copy_from_slice(&[1, 0]);
    out[n + 4..n + 8].copy_from_slice(&[0x00, 0x0f, 0xac, 0x04]);
    out[n + 8..n + 10].copy_from_slice(&[1, 0]);
    out[n + 10..n + 14].copy_from_slice(&[0x00, 0x0f, 0xac, 0x04]);
    out[n + 14..n + 16].copy_from_slice(&[1, 0]);
    out[n + 16..n + 20].copy_from_slice(&[0x00, 0x0f, 0xac, 0x02]);
    out[n + 20..n + 22].copy_from_slice(&[0, 0]);
    n += 22;
    Some(n)
}

fn write_mgmt(out: &mut [u8], subtype: u8, a1: &[u8; 6], a2: &[u8; 6], a3: &[u8; 6], seq: u16) {
    out[..24].fill(0);
    out[0] = subtype;
    out[4..10].copy_from_slice(a1);
    out[10..16].copy_from_slice(a2);
    out[16..22].copy_from_slice(a3);
    let sc = seq << 4;
    out[22] = sc as u8;
    out[23] = (sc >> 8) as u8;
}

/// QoS data, To DS. `msdu` is the LLC payload, already encrypted when `protect` is set.
pub fn write_qos_data(
    out: &mut [u8],
    bssid: &[u8; 6],
    sta: &[u8; 6],
    da: &[u8; 6],
    msdu: &[u8],
    seq: u16,
    protect: bool,
) -> Option<usize> {
    let n = 26 + msdu.len();
    if out.len() < n {
        return None;
    }
    out[..n].fill(0);
    out[0] = 0x88;
    out[1] = if protect { 0x41 } else { 0x01 };
    out[4..10].copy_from_slice(bssid);
    out[10..16].copy_from_slice(sta);
    out[16..22].copy_from_slice(da);
    let sc = seq << 4;
    out[22] = sc as u8;
    out[23] = (sc >> 8) as u8;
    out[26..n].copy_from_slice(msdu);
    Some(n)
}

/// LLC/SNAP plus the ethertype and payload of an Ethernet frame from `wifi`.
pub fn llc_from_ethernet(ether: &[u8], out: &mut [u8]) -> Option<usize> {
    if ether.len() < 14 || out.len() < 6 + ether.len() - 12 {
        return None;
    }
    out[..6].copy_from_slice(&[0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00]);
    let rest = &ether[12..];
    out[6..6 + rest.len()].copy_from_slice(rest);
    Some(6 + rest.len())
}

/// Ethernet frame reconstructed from an 802.11 data body, for the DHCP parser.
pub fn ethernet_from_msdu(msdu: &[u8], sta: &[u8; 6], out: &mut [u8]) -> Option<usize> {
    let payload = msdu.strip_prefix(&[0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00])?;
    if out.len() < 12 + payload.len() {
        return None;
    }
    out[..6].fill(0xff);
    out[6..12].copy_from_slice(sta);
    out[12..12 + payload.len()].copy_from_slice(payload);
    Some(12 + payload.len())
}

pub fn eapol_in_msdu(msdu: &[u8]) -> Option<&[u8]> {
    let rest = msdu.strip_prefix(&[0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00])?;
    if rest.len() >= 2 && rest[0] == 0x88 && rest[1] == 0x8e {
        Some(&rest[2..])
    } else {
        None
    }
}

pub fn key_ok(ssid: &[u8], psk: &[u8]) -> bool {
    !ssid.is_empty() && ssid.len() <= SSID_MAX && (PSK_MIN..=PSK_MAX).contains(&psk.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_block_matches_fips() {
        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let mut block = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        aes128_encrypt_block(&key, &mut block);
        assert_eq!(
            block,
            [
                0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
                0xc5, 0x5a
            ]
        );
    }

    #[test]
    fn ccmp_round_trip() {
        let tk = [0x11u8; 16];
        let mut hdr = [0u8; 26];
        hdr[0] = 0x88;
        hdr[1] = 0x41;
        hdr[10] = 0x02;
        let msdu = b"hello-lerux-ccmp";
        let mut enc = [0u8; 64];
        let n = ccmp_encrypt(&tk, &hdr, 1, msdu, &mut enc).unwrap();
        let mut frame = [0u8; 96];
        frame[..26].copy_from_slice(&hdr);
        frame[26..26 + n].copy_from_slice(&enc[..n]);
        let mut plain = [0u8; 64];
        let got = ccmp_decrypt(&tk, 26, &frame[..26 + n], &mut plain).unwrap();
        assert_eq!(&plain[..got], msdu);
    }

    #[test]
    fn message2_mic_checks() {
        let pmk = pmk(b"password", b"IEEE");
        let ap = [0x02, 0, 0, 0, 0, 1];
        let sta = [0x02, 0, 0, 0, 0, 2];
        let anonce = [1u8; 32];
        let snonce = [2u8; 32];
        let ptk = derive_ptk(&pmk, &ap, &sta, &anonce, &snonce);
        let replay = [0, 0, 0, 0, 0, 0, 0, 1];
        let mut m1 = [0u8; 99];
        write_eapol_key(&mut m1, KEY_PAIR | KEY_ACK, &replay, &anonce, &[]).unwrap();
        let (got_nonce, got_replay) = message1(&m1).unwrap();
        assert_eq!(got_nonce, anonce);
        assert_eq!(got_replay, replay);
        let mut m2 = [0u8; 128];
        let n = message2(&mut m2, &anonce, &snonce, &replay, &ptk.kck).unwrap();
        assert!(mic_ok(&m2[..n], &ptk.kck));
    }

    #[test]
    fn beacon_names_the_ssid() {
        let mut frame = [0u8; 48];
        frame[0] = 0x80;
        frame[16] = 0xaa;
        frame[36] = 0;
        frame[37] = 4;
        frame[38..42].copy_from_slice(b"home");
        frame[42] = 3;
        frame[43] = 1;
        frame[44] = 6;
        let found = match_beacon(&frame, b"home").unwrap();
        assert_eq!(found.bssid[0], 0xaa);
        assert_eq!(found.channel, 6);
        assert!(match_beacon(&frame, b"other").is_none());
    }
}
