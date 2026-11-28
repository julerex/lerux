//! One command against a [`crate::Station`] snapshot.
//!
//! Scan, connect, and disconnect return an [`crate::Action`]. The shell performs
//! it on the MT7921 page and calls the `after_*` method with the result.

use crate::{
    fmt::{caption, footer, header, mac_text, row, Align, Col, Out},
    help,
    parse::{self, Parsed},
    table::{self, Spec, HELP, QUIT, VERSION},
    Action, Connect, Network, Security, Station, Turn, SCAN_MAX,
};

const ERR_INVALID: &[u8] = b"Invalid command\n\n";
const ERR_UNSUPPORTED: &[u8] = b"Unsupported command\n";
const ERR_OPERATION: &[u8] = b"Operation failed\n";
const ERR_PMK: &[u8] = b"PMK generation failed. Ensure Crypto Engine is properly configured\n";
const ERR_HIDDEN: &[u8] = b"Hidden networks are not kept\n";
const ERR_KNOWN: &[u8] = b"Known networks are not stored\n";
const ERR_NOT_CONNECTED: &[u8] = b"Not connected\n";
const ERR_NO_PASS: &[u8] = b"No passphrase is provided as '--passphrase' command-line option.\n\n";
const ERR_AMBIGUOUS: &[u8] = b"Provided network name is ambiguous. Please specify security type.\n";

pub(crate) struct Session {
    interactive: bool,
    dont_ask: bool,
    pass: Option<parse::Pass>,
    pending: Option<Pending>,
    want_secret: bool,
    connect: Option<Connect>,
}

struct Pending {
    ssid: [u8; 32],
    ssid_len: u8,
}

impl Session {
    pub(crate) const fn new() -> Self {
        Self {
            interactive: false,
            dont_ask: false,
            pass: None,
            pending: None,
            want_secret: false,
            connect: None,
        }
    }

    pub(crate) fn connect(&self) -> Option<Connect> {
        self.connect
    }

    pub(crate) fn open(&mut self, line: &[u8], station: &Station, buf: &mut [u8]) -> Turn {
        self.start(line, station, buf, true)
    }

    pub(crate) fn command(&mut self, line: &[u8], station: &Station, buf: &mut [u8]) -> Turn {
        self.start(line, station, buf, false)
    }

    pub(crate) fn secret(&mut self, bytes: &[u8], buf: &mut [u8]) -> Turn {
        let mut out = Out::new(buf);
        self.want_secret = false;
        self.connect = None;
        let Some(pending) = self.pending.take() else {
            out.push(ERR_INVALID);
            return self.finish(&out, Action::None);
        };
        let action = self.store_pass(&mut out, &pending, bytes);
        self.finish(&out, action)
    }

    pub(crate) fn after_radio(&mut self, ok: bool, buf: &mut [u8]) -> Turn {
        let mut out = Out::new(buf);
        if !ok {
            out.push(ERR_OPERATION);
        }
        self.finish(&out, Action::None)
    }
}

impl Session {
    fn start(&mut self, line: &[u8], station: &Station, buf: &mut [u8], argv: bool) -> Turn {
        let mut out = Out::new(buf);
        self.connect = None;
        let Some(parsed) = parse_line(line, argv) else {
            out.push(ERR_INVALID);
            return self.finish(&out, Action::None);
        };
        if argv {
            self.note_options(&parsed);
        }
        if parsed.help {
            help::render(&mut out, self.interactive);
            return self.finish(&out, Action::None);
        }
        if parsed.count == 0 {
            if argv {
                self.interactive = true;
            }
            return self.finish(&out, Action::None);
        }
        let action = dispatch(self, &parsed, station, &mut out);
        self.finish(&out, action)
    }

    fn note_options(&mut self, parsed: &Parsed) {
        self.dont_ask = parsed.dont_ask;
        if parsed.pass.is_some() {
            self.pass = parsed.pass.as_ref().map(clone_pass);
        }
    }

    fn finish(&self, out: &Out<'_>, action: Action) -> Turn {
        Turn {
            written: out.written(),
            truncated: out.truncated,
            action,
            interactive: self.interactive,
            read_secret: self.want_secret,
        }
    }
}

fn parse_line(line: &[u8], argv: bool) -> Option<Parsed> {
    if argv {
        parse::parse(line)
    } else {
        parse::words_only(line)
    }
}

fn clone_pass(pass: &parse::Pass) -> parse::Pass {
    parse::Pass {
        bytes: pass.bytes,
        len: pass.len,
    }
}

fn dispatch(
    session: &mut Session,
    parsed: &Parsed,
    station: &Station,
    out: &mut Out<'_>,
) -> Action {
    let Some(spec) = resolve(parsed) else {
        out.push(ERR_INVALID);
        return Action::None;
    };
    if spec.id == HELP {
        help::render(out, true);
        return Action::None;
    }
    if spec.id == QUIT {
        return Action::Exit;
    }
    if spec.id == VERSION {
        out.push(b"IWD version 3.12\n");
        return Action::None;
    }
    if spec.id == table::UNSUPPORTED {
        out.push(ERR_UNSUPPORTED);
        return Action::None;
    }
    run_id(session, spec, parsed, station, out)
}

fn resolve(parsed: &Parsed) -> Option<&'static Spec> {
    let first = parsed.word(0);
    if let Some(spec) = table::find(b"", false, first) {
        return Some(spec);
    }
    if !table::family_known(first) || parsed.count < 2 {
        return None;
    }
    if parsed.count >= 3
        && let Some(spec) = table::find(first, true, parsed.word(2))
    {
        return Some(spec);
    }
    table::find(first, false, parsed.word(1))
}

fn run_id(
    session: &mut Session,
    spec: &Spec,
    parsed: &Parsed,
    station: &Station,
    out: &mut Out<'_>,
) -> Action {
    let entity = entity_of(spec, parsed);
    let args = args_of(spec, parsed);
    match spec.id {
        table::ADAPTER_LIST => adapter_list(station, out),
        table::ADAPTER_SHOW => adapter_show(entity, station, out),
        table::DEVICE_LIST => device_list(station, out),
        table::DEVICE_SHOW => device_show(entity, station, out),
        table::STATION_LIST => station_list(station, out),
        table::STATION_SCAN => station_scan(entity, station, out),
        table::STATION_NETWORKS => station_networks(entity, station, out),
        table::STATION_CONNECT => station_connect(session, entity, &args, station, out),
        table::STATION_DISCONNECT => station_disconnect(entity, station, out),
        table::STATION_SHOW => station_show(entity, station, out),
        table::STATION_HIDDEN | table::STATION_HIDDEN_CONNECT => hidden(entity, station, out),
        table::KNOWN_LIST | table::KNOWN_FORGET | table::KNOWN_SHOW => {
            out.push(ERR_KNOWN);
            Action::None
        }
        _ => {
            out.push(ERR_UNSUPPORTED);
            Action::None
        }
    }
}

fn entity_of<'a>(spec: &Spec, parsed: &'a Parsed) -> &'a [u8] {
    if spec.entity && parsed.count >= 2 {
        parsed.word(1)
    } else {
        b""
    }
}

fn args_of<'a>(spec: &Spec, parsed: &'a Parsed) -> Args<'a> {
    let start = if spec.entity { 3 } else { 2 };
    Args { parsed, start }
}

struct Args<'a> {
    parsed: &'a Parsed,
    start: usize,
}

impl Args<'_> {
    fn get(&self, index: usize) -> Option<&[u8]> {
        let at = self.start + index;
        (at < usize::from(self.parsed.count)).then(|| self.parsed.word(at))
    }
}

fn col(text: &'static [u8], width: usize) -> Col<'static> {
    Col {
        text,
        width,
        align: Align::Left,
    }
}

fn adapter_list(station: &Station, out: &mut Out<'_>) -> Action {
    caption(out, b"Adapters");
    header(
        out,
        &[
            col(b"Name", 8),
            col(b"Powered", 8),
            col(b"Vendor", 20),
            col(b"Model", 20),
        ],
    );
    crate::fmt::rule(out);
    if station.present {
        row(
            out,
            &[
                (b"phy0", 8),
                (powered(station), 8),
                (b"MediaTek", 20),
                (b"MT7921", 20),
            ],
        );
    }
    footer(out);
    Action::None
}

fn adapter_show(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if !station.present || name != b"phy0" {
        missing(out, b"Adapter '", name, b"' not found\n");
        return Action::None;
    }
    property_table(out, b"Adapter: phy0", 17, 50);
    prop(out, false, b"Name", b"phy0", 17, 50);
    prop(out, true, b"Powered", powered(station), 17, 50);
    prop(out, false, b"Vendor", b"MediaTek", 17, 50);
    prop(out, false, b"Model", b"MT7921", 17, 50);
    prop(out, false, b"SupportedModes", b"station ", 17, 50);
    footer(out);
    Action::None
}

fn device_list(station: &Station, out: &mut Out<'_>) -> Action {
    caption(out, b"Devices");
    header(
        out,
        &[
            col(b"Name", 20),
            col(b"Address", 20),
            col(b"Powered", 10),
            col(b"Adapter", 10),
            col(b"Mode", 10),
        ],
    );
    crate::fmt::rule(out);
    if station.present {
        let addr = mac_text(station.address);
        row(
            out,
            &[
                (b"wlan0", 20),
                (&addr, 20),
                (powered(station), 10),
                (b"phy0", 10),
                (b"station", 10),
            ],
        );
    }
    footer(out);
    Action::None
}

fn device_show(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if device_missing(name, station, out).is_none() {
        return Action::None;
    }
    property_table(out, b"Device: wlan0", 20, 47);
    prop(out, false, b"Name", b"wlan0", 20, 47);
    prop(out, true, b"Mode", b"station", 20, 47);
    prop(out, true, b"Powered", powered(station), 20, 47);
    let addr = mac_text(station.address);
    prop(out, false, b"Address", &addr, 20, 47);
    prop(out, false, b"Adapter", b"phy0", 20, 47);
    footer(out);
    Action::None
}

fn station_list(station: &Station, out: &mut Out<'_>) -> Action {
    caption(out, b"Devices in Station Mode");
    header(
        out,
        &[col(b"Name", 20), col(b"State", 15), col(b"Scanning", 8)],
    );
    crate::fmt::rule(out);
    if station.present {
        row(out, &[(b"wlan0", 20), (state(station), 15), (b"", 8)]);
    } else {
        out.push(b"No devices in Station mode available.\n");
    }
    footer(out);
    Action::None
}

fn station_scan(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if !ready(name, station, out) {
        return Action::None;
    }
    Action::Scan
}

fn station_networks(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if !ready(name, station, out) {
        return Action::None;
    }
    networks_table(station, out);
    Action::None
}

fn station_show(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if device_missing(name, station, out).is_none() {
        return Action::None;
    }
    let title = b"Station: wlan0";
    property_table(out, title, 20, 47);
    prop(out, false, b"Scanning", b"no", 20, 47);
    prop(out, false, b"State", state(station), 20, 47);
    if station.connected {
        let ssid = station.ssid();
        prop(out, false, b"Connected network", ssid, 20, 47);
    }
    footer(out);
    Action::None
}

fn station_disconnect(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if !ready(name, station, out) {
        return Action::None;
    }
    if !station.connected {
        out.push(ERR_NOT_CONNECTED);
        return Action::None;
    }
    Action::Disconnect
}

fn station_connect(
    session: &mut Session,
    name: &[u8],
    args: &Args<'_>,
    station: &Station,
    out: &mut Out<'_>,
) -> Action {
    if !ready(name, station, out) {
        return Action::None;
    }
    let Some(ssid) = args.get(0) else {
        pattern(out, table::find(b"station", true, b"connect"));
        return Action::None;
    };
    match find_net(station, ssid, args.get(1)) {
        Hit::None => unknown_net(out, ssid),
        Hit::Many => out.push(ERR_AMBIGUOUS),
        Hit::One(net) => return begin_connect(session, net, out),
    }
    Action::None
}

fn begin_connect(session: &mut Session, net: &Network, out: &mut Out<'_>) -> Action {
    if net.security != Security::Psk {
        out.push(ERR_OPERATION);
        return Action::None;
    }
    if let Some(pass) = session.pass {
        return accept_stored(session, out, net, &pass);
    }
    if session.dont_ask {
        out.push(ERR_NO_PASS);
        return Action::None;
    }
    ask_pass(session, net, out);
    Action::None
}

fn accept_stored(
    session: &mut Session,
    out: &mut Out<'_>,
    net: &Network,
    pass: &parse::Pass,
) -> Action {
    let bytes = &pass.bytes[..usize::from(pass.len)];
    let pending = pending_from(net);
    session.store_pass(out, &pending, bytes)
}

fn ask_pass(session: &mut Session, net: &Network, out: &mut Out<'_>) {
    out.push(b"Type the network passphrase for ");
    out.push(net.name());
    out.push(b" psk.\nPassphrase: ");
    session.pending = Some(pending_from(net));
    session.want_secret = true;
}

fn pending_from(net: &Network) -> Pending {
    Pending {
        ssid: net.name,
        ssid_len: net.name_len,
    }
}

fn hidden(name: &[u8], station: &Station, out: &mut Out<'_>) -> Action {
    if device_missing(name, station, out).is_some() {
        out.push(ERR_HIDDEN);
    }
    Action::None
}

impl Session {
    fn store_pass(&mut self, out: &mut Out<'_>, pending: &Pending, bytes: &[u8]) -> Action {
        if !(8..=63).contains(&bytes.len()) {
            out.push(ERR_PMK);
            return Action::None;
        }
        let mut psk = [0u8; 63];
        psk[..bytes.len()].copy_from_slice(bytes);
        let psk_len = u8::try_from(bytes.len()).unwrap_or(0);
        self.connect = Some(Connect {
            ssid: pending.ssid,
            ssid_len: pending.ssid_len,
            psk,
            psk_len,
        });
        Action::Connect
    }
}

fn ready(name: &[u8], station: &Station, out: &mut Out<'_>) -> bool {
    if device_missing(name, station, out).is_none() {
        return false;
    }
    if station.powered {
        return true;
    }
    out.push(ERR_OPERATION);
    false
}

fn device_missing<'a>(name: &[u8], station: &'a Station, out: &mut Out<'_>) -> Option<&'a Station> {
    if station.present && name == b"wlan0" {
        return Some(station);
    }
    missing(out, b"Device ", name, b" not found.\n");
    None
}

fn missing(out: &mut Out<'_>, prefix: &[u8], name: &[u8], suffix: &[u8]) {
    out.push(prefix);
    out.push(name);
    out.push(suffix);
}

fn unknown_net(out: &mut Out<'_>, name: &[u8]) {
    out.push(b"Invalid network name '");
    out.push(name);
    out.push(b"'\n");
}

fn pattern(out: &mut Out<'_>, spec: Option<&Spec>) {
    out.push(b"Invalid command. Use the following pattern:\n");
    if let Some(spec) = spec {
        row(out, &[(spec.line, 50), (spec.desc, 30)]);
    }
}

#[derive(Clone, Copy)]
enum Hit<'a> {
    None,
    One(&'a Network),
    Many,
}

fn find_net<'a>(station: &'a Station, name: &[u8], kind: Option<&[u8]>) -> Hit<'a> {
    if name.is_empty() || name.len() > 32 {
        return Hit::None;
    }
    let mut found = Hit::None;
    let n = usize::from(station.count).min(SCAN_MAX);
    for net in &station.networks[..n] {
        if net.name() != name {
            continue;
        }
        if kind.is_some_and(|kind| kind != sec_name(net.security)) {
            continue;
        }
        found = push_hit(found, net);
    }
    found
}

fn push_hit<'a>(found: Hit<'a>, net: &'a Network) -> Hit<'a> {
    match found {
        Hit::None => Hit::One(net),
        Hit::One(_) | Hit::Many => Hit::Many,
    }
}

fn networks_table(station: &Station, out: &mut Out<'_>) {
    caption(out, b"Available networks");
    header(out, &net_cols());
    crate::fmt::rule(out);
    let n = usize::from(station.count).min(SCAN_MAX);
    if n == 0 {
        out.push(b"No networks available\n");
        footer(out);
        return;
    }
    for net in &station.networks[..n] {
        network_row(out, station, net);
    }
    footer(out);
}

fn net_cols() -> [Col<'static>; 4] {
    [
        Col {
            text: b"",
            width: 2,
            align: Align::Left,
        },
        Col {
            text: b"Network name",
            width: 32,
            align: Align::Left,
        },
        Col {
            text: b"Security",
            width: 18,
            align: Align::Left,
        },
        Col {
            text: b"Signal",
            width: 6,
            align: Align::Right,
        },
    ]
}

fn network_row(out: &mut Out<'_>, station: &Station, net: &Network) {
    let mark: &[u8] = if station.connected && station.ssid() == net.name() {
        b"> "
    } else {
        b""
    };
    row(
        out,
        &[
            (mark, 2),
            (net.name(), 32),
            (sec_name(net.security), 18),
            (b"", 6),
        ],
    );
}

fn property_table(out: &mut Out<'_>, title: &[u8], name: usize, value: usize) {
    caption(out, title);
    header(out, &prop_cols(name, value));
    crate::fmt::rule(out);
}

fn prop_cols(name: usize, value: usize) -> [Col<'static>; 3] {
    [
        Col {
            text: b"Settable",
            width: 8,
            align: Align::Left,
        },
        Col {
            text: b"Property",
            width: name,
            align: Align::Left,
        },
        Col {
            text: b"Value",
            width: value,
            align: Align::Left,
        },
    ]
}

fn prop(
    out: &mut Out<'_>,
    settable: bool,
    name: &[u8],
    value: &[u8],
    name_w: usize,
    value_w: usize,
) {
    let star: &[u8] = if settable { b"       *" } else { b"" };
    row(out, &[(star, 8), (name, name_w), (value, value_w)]);
}

fn powered(station: &Station) -> &'static [u8] {
    if station.powered {
        b"on"
    } else {
        b"off"
    }
}

fn state(station: &Station) -> &'static [u8] {
    if station.connected {
        b"connected"
    } else {
        b"disconnected"
    }
}

fn sec_name(security: Security) -> &'static [u8] {
    match security {
        Security::Open => b"open",
        Security::Psk => b"psk",
        Security::Other => b"other",
    }
}
