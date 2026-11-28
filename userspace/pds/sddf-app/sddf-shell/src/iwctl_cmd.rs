//! `iwctl` on the Swift shell. The radio path is the one `setup-wifi` uses.

use lerux_iwctl::{
    Action, Connect, Network, Security, Session, Station, Turn, LINE_MAX, PROMPT, SCAN_MAX,
};
use lerux_swift::{
    drop_requested, join_pending, join_up, mirror_heard, note_dropped, read_heard_count,
    read_heard_security, read_heard_ssid, read_key, read_link_name, read_station,
    read_survey_count, read_survey_security, read_survey_ssid, request_drop, survey_requested,
    write_key, write_link_name, write_survey, SURVEY_SEC_OTHER, SURVEY_SEC_PSK,
};

use crate::{discard_pending, emit, key_page, kick_radio, read_byte, say};

pub(crate) fn run(rest: &[u8]) {
    let mut session = Session::new();
    let mut station = load_station();
    let mut buf = [0u8; 16384];
    let mut turn = session.open(rest, &station, &mut buf);
    while let Some(next) = advance(&mut session, turn, &mut station, &mut buf) {
        turn = next;
    }
}

fn advance(
    session: &mut Session,
    turn: Turn,
    station: &mut Station,
    buf: &mut [u8],
) -> Option<Turn> {
    say(&buf[..turn.written]);
    match turn.action {
        Action::Exit => None,
        Action::Scan => Some(session.after_radio(post_scan(), buf)),
        Action::Connect => Some(session.after_radio(post_connect(session.connect()), buf)),
        Action::Disconnect => Some(session.after_radio(post_drop(), buf)),
        Action::None => follow(session, turn, station, buf),
    }
}

fn follow(
    session: &mut Session,
    turn: Turn,
    station: &mut Station,
    buf: &mut [u8],
) -> Option<Turn> {
    if turn.read_secret {
        let mut secret = [0u8; 64];
        let n = read_line(&mut secret, true)?;
        return Some(session.secret(&secret[..n], buf));
    }
    if !turn.interactive {
        return None;
    }
    say(PROMPT);
    let mut line = [0u8; LINE_MAX];
    let n = read_line(&mut line, false)?;
    *station = load_station();
    Some(session.command(&line[..n], station, buf))
}

fn post_scan() -> bool {
    if !write_survey(key_page()) {
        return false;
    }
    kick_radio();
    discard_pending();
    if survey_requested(key_page()) {
        return false;
    }
    mirror_heard(key_page())
}

fn post_connect(connect: Option<Connect>) -> bool {
    let Some(connect) = connect else {
        return false;
    };
    if !write_key(key_page(), connect.ssid(), connect.psk()) {
        return false;
    }
    kick_radio();
    discard_pending();
    if join_up(key_page()) {
        let _ = write_link_name(key_page(), connect.ssid());
        return true;
    }
    // The radio drops the previous BSS before it scans. A finished failure
    // must not leave that name. A join that never ran stays pending.
    if !join_pending(key_page()) {
        note_dropped(key_page());
    }
    false
}

fn post_drop() -> bool {
    if !request_drop(key_page()) {
        return false;
    }
    kick_radio();
    discard_pending();
    if drop_requested(key_page()) {
        note_dropped(key_page());
        return false;
    }
    true
}

fn load_station() -> Station {
    let page = key_page();
    let mut station = Station::absent();
    station.present = true;
    if let Some((powered, address)) = read_station(page) {
        station.powered = powered;
        station.address = address;
    }
    apply_link(&mut station, page);
    apply_networks(&mut station, page);
    station
}

fn apply_link(station: &mut Station, page: &[u8]) {
    if let Some(ssid) = read_link_name(page) {
        store_ssid(station, ssid);
        return;
    }
    if join_up(page)
        && let Some((ssid, _)) = read_key(page)
    {
        store_ssid(station, ssid);
    }
}

fn store_ssid(station: &mut Station, ssid: &[u8]) {
    let n = ssid.len().min(station.ssid.len());
    station.ssid[..n].copy_from_slice(&ssid[..n]);
    station.ssid_len = u8::try_from(n).unwrap_or(0);
    station.connected = n > 0;
}

fn apply_networks(station: &mut Station, page: &[u8]) {
    if read_heard_count(page).unwrap_or(0) > 0 {
        let n = copy_heard(station, page);
        station.count = u8::try_from(n).unwrap_or(0);
        return;
    }
    let n = copy_survey(station, page);
    station.count = u8::try_from(n).unwrap_or(0);
}

fn copy_heard(station: &mut Station, page: &[u8]) -> usize {
    let n = read_heard_count(page).unwrap_or(0).min(SCAN_MAX);
    for i in 0..n {
        let Some(name) = read_heard_ssid(page, i) else {
            return i;
        };
        let sec = read_heard_security(page, i).unwrap_or(0);
        if !store_net(station, i, name, sec) {
            return i;
        }
    }
    n
}

fn copy_survey(station: &mut Station, page: &[u8]) -> usize {
    let n = read_survey_count(page).unwrap_or(0).min(SCAN_MAX);
    for i in 0..n {
        let Some(name) = read_survey_ssid(page, i) else {
            return i;
        };
        let sec = read_survey_security(page, i).unwrap_or(0);
        if !store_net(station, i, name, sec) {
            return i;
        }
    }
    n
}

fn store_net(station: &mut Station, index: usize, name: &[u8], sec: u8) -> bool {
    let Some(net) = Network::new(name, security(sec)) else {
        return false;
    };
    station.networks[index] = net;
    true
}

fn security(sec: u8) -> Security {
    match sec {
        SURVEY_SEC_PSK => Security::Psk,
        SURVEY_SEC_OTHER => Security::Other,
        _ => Security::Open,
    }
}

fn read_line(buf: &mut [u8], mask: bool) -> Option<usize> {
    let mut len = 0usize;
    let mut overflow = false;
    loop {
        let byte = read_byte();
        if byte == 0x04 {
            return None;
        }
        if byte == b'\n' {
            continue;
        }
        if byte == b'\r' {
            emit(b"\n");
            return Some(if overflow { 0 } else { len });
        }
        if byte == 0x7f {
            rubout(&mut len, overflow);
            continue;
        }
        if len == buf.len() {
            overflow = true;
            continue;
        }
        buf[len] = byte;
        len += 1;
        paint_in(byte, mask);
    }
}

fn rubout(len: &mut usize, overflow: bool) {
    if *len > 0 && !overflow {
        *len -= 1;
        emit(&[0x08, b' ', 0x08]);
    }
}

fn paint_in(byte: u8, mask: bool) {
    if mask {
        emit(b"*");
    } else {
        emit(&[byte]);
    }
}
