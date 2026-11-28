//! `iwctl` session for the Swift MT7921 station.
//!
//! The words, the prompt, and the tables follow iwd 3.12 `client/`. This image
//! has one station, `wlan0`, on adapter `phy0`. A scan, a connection, or a
//! disconnect is an [`Action`]. The shell performs that action on the existing
//! wifi page and reports the radio's result with [`Session::after_radio`].

#![cfg_attr(not(test), no_std)]

mod fmt;
mod help;
mod parse;
mod run;
mod table;

/// The outer shell command, in either ASCII case. Inner `iwctl` words stay case-sensitive.
#[must_use]
pub fn is_iwctl(cmd: &[u8]) -> bool {
    cmd.eq_ignore_ascii_case(b"iwctl")
}

/// Interactive prompt, without the color escapes iwd wraps around `[iwd]`.
pub const PROMPT: &[u8] = b"[iwd]# ";

/// Names kept from one survey. Matches the radio's survey cap.
pub const SCAN_MAX: usize = 16;

/// Bytes one interactive line may hold. The outer shell line stays 80.
pub const LINE_MAX: usize = 128;

/// How a heard beacon authenticates.
///
/// `psk` is the RSN pairwise AKM `00:0f:ac:02`. A beacon with no RSN element
/// is `open`. Any other RSN element is `other`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Security {
    /// No RSN element.
    Open,
    /// WPA-Personal.
    Psk,
    /// An RSN element this station does not join.
    Other,
}

/// One name from the last survey.
#[derive(Clone, Copy, Debug)]
pub struct Network {
    /// SSID bytes. The live prefix is [`Network::name_len`].
    pub name: [u8; 32],
    /// Length of [`Network::name`]. At most 32.
    pub name_len: u8,
    /// Authentication read from the beacon.
    pub security: Security,
}

impl Network {
    /// The SSID bytes.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        let n = usize::from(self.name_len).min(self.name.len());
        &self.name[..n]
    }

    /// A network whose name fits in 32 bytes.
    #[must_use]
    pub fn new(name: &[u8], security: Security) -> Option<Self> {
        if name.is_empty() || name.len() > 32 {
            return None;
        }
        let mut owned = [0u8; 32];
        owned[..name.len()].copy_from_slice(name);
        let name_len = u8::try_from(name.len()).ok()?;
        Some(Self {
            name: owned,
            name_len,
            security,
        })
    }
}

/// The one station this image can show.
#[derive(Clone, Copy, Debug)]
pub struct Station {
    /// The radio protection domain is on this image.
    pub present: bool,
    /// The firmware has reached `wait`.
    pub powered: bool,
    /// Station address, printed as lowercase colon hex.
    pub address: [u8; 6],
    /// A handshake finished and has not been dropped.
    pub connected: bool,
    /// Connected SSID. The live prefix is [`Station::ssid_len`].
    pub ssid: [u8; 32],
    /// Length of [`Station::ssid`].
    pub ssid_len: u8,
    /// Heard names, in the order the survey stored them.
    pub networks: [Network; SCAN_MAX],
    /// How many of [`Station::networks`] are live.
    pub count: u8,
}

impl Station {
    /// A board with no MT7921 row.
    #[must_use]
    pub const fn absent() -> Self {
        Self {
            present: false,
            powered: false,
            address: [0; 6],
            connected: false,
            ssid: [0; 32],
            ssid_len: 0,
            networks: [Network {
                name: [0; 32],
                name_len: 0,
                security: Security::Open,
            }; SCAN_MAX],
            count: 0,
        }
    }

    /// The connected SSID.
    #[must_use]
    pub fn ssid(&self) -> &[u8] {
        let n = usize::from(self.ssid_len).min(self.ssid.len());
        &self.ssid[..n]
    }
}

/// Passphrase accepted for one `connect`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Connect {
    /// SSID the survey listed.
    pub ssid: [u8; 32],
    /// Length of [`Connect::ssid`].
    pub ssid_len: u8,
    /// Passphrase bytes. The live prefix is [`Connect::psk_len`].
    pub psk: [u8; 63],
    /// Length of [`Connect::psk`], from 8 to 63.
    pub psk_len: u8,
}

impl Connect {
    /// The SSID bytes.
    #[must_use]
    pub fn ssid(&self) -> &[u8] {
        let n = usize::from(self.ssid_len).min(self.ssid.len());
        &self.ssid[..n]
    }

    /// The passphrase bytes.
    #[must_use]
    pub fn psk(&self) -> &[u8] {
        let n = usize::from(self.psk_len).min(self.psk.len());
        &self.psk[..n]
    }
}

/// What the shell does after printing a command's text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// The reply is complete.
    None,
    /// Post a survey. Do not print the survey mark.
    Scan,
    /// Write the key from [`Session::connect`] and notify.
    Connect,
    /// Ask the radio to drop the link.
    Disconnect,
    /// Leave the prompt. `exit` and `quit` both produce this.
    Exit,
}

/// One reply: the bytes to print, and what to do next.
#[derive(Clone, Copy, Debug)]
pub struct Turn {
    /// Bytes written into the caller's buffer.
    pub written: usize,
    /// The buffer was shorter than the reply.
    pub truncated: bool,
    /// The radio or prompt action that follows the text.
    pub action: Action,
    /// `iwctl` with no command stays at the prompt.
    pub interactive: bool,
    /// The next read is a masked passphrase.
    pub read_secret: bool,
}

/// Command state for one `iwctl` invocation.
pub struct Session {
    inner: run::Session,
}

impl Session {
    /// A session that has not seen a command.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: run::Session::new(),
        }
    }

    /// The passphrase from the last `connect`, when [`Action::Connect`] is set.
    #[must_use]
    pub fn connect(&self) -> Option<Connect> {
        self.inner.connect()
    }

    /// Parse an outer `iwctl` line, including `--passphrase` and `--help`.
    #[must_use]
    pub fn open(&mut self, line: &[u8], station: &Station, buf: &mut [u8]) -> Turn {
        self.inner.open(line, station, buf)
    }

    /// Parse one line typed at `[iwd]# `. Options are not read again.
    #[must_use]
    pub fn command(&mut self, line: &[u8], station: &Station, buf: &mut [u8]) -> Turn {
        self.inner.command(line, station, buf)
    }

    /// Accept the masked passphrase.
    #[must_use]
    pub fn secret(&mut self, bytes: &[u8], buf: &mut [u8]) -> Turn {
        self.inner.secret(bytes, buf)
    }

    /// Finish a scan, connect, or disconnect. `ok` false prints `Operation failed`.
    #[must_use]
    pub fn after_radio(&mut self, ok: bool, buf: &mut [u8]) -> Turn {
        self.inner.after_radio(ok, buf)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Network, Security, Session, Station, PROMPT, SCAN_MAX};

    const MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

    fn powered() -> Station {
        let mut station = Station::absent();
        station.present = true;
        station.powered = true;
        station.address = MAC;
        station
    }

    fn with_net(name: &[u8], security: Security) -> Station {
        let mut station = powered();
        station.networks[0] = Network::new(name, security).expect("name");
        station.count = 1;
        station
    }

    fn text(buf: &[u8], n: usize) -> &str {
        core::str::from_utf8(&buf[..n]).expect("utf8")
    }

    fn run(line: &str, station: &Station) -> (Session, String, Action) {
        let mut session = Session::new();
        let mut buf = [0u8; 8192];
        let turn = session.open(line.as_bytes(), station, &mut buf);
        let body = text(&buf, turn.written).to_string();
        (session, body, turn.action)
    }

    #[test]
    fn prompt_matches_iwd() {
        assert_eq!(PROMPT, b"[iwd]# ");
    }

    #[test]
    fn empty_invocation_is_interactive() {
        let mut session = Session::new();
        let mut buf = [0u8; 64];
        let turn = session.open(b"", &powered(), &mut buf);
        assert!(turn.interactive);
        assert_eq!(turn.action, Action::None);
        assert_eq!(turn.written, 0);
    }

    #[test]
    fn unknown_command() {
        let (_session, body, action) = run("frobnicate", &powered());
        assert_eq!(action, Action::None);
        assert_eq!(body, "Invalid command\n\n");
    }

    #[test]
    fn device_list_row() {
        let (_session, body, action) = run("device list", &powered());
        assert_eq!(action, Action::None);
        assert!(body.contains("Devices"));
        assert!(body.contains("wlan0"));
        assert!(body.contains("52:54:00:12:34:56"));
        assert!(body.contains("phy0"));
        assert!(body.contains("station"));
        assert!(body.contains("on"));
        let row = body
            .lines()
            .find(|line| line.contains("wlan0"))
            .expect("row");
        assert!(row.contains("52:54:00:12:34:56"));
    }

    #[test]
    fn device_list_powered_off() {
        let mut station = powered();
        station.powered = false;
        let (_session, body, _) = run("device list", &station);
        assert!(body.contains("off"));
        assert!(!body.contains(" on "));
    }

    #[test]
    fn absent_device_has_header_only() {
        let (_session, body, _) = run("device list", &Station::absent());
        assert!(body.contains("Name"));
        assert!(!body.contains("wlan0"));
    }

    #[test]
    fn scan_is_silent_when_powered() {
        let (_session, body, action) = run("station wlan0 scan", &powered());
        assert_eq!(action, Action::Scan);
        assert_eq!(body, "");
    }

    #[test]
    fn scan_fails_when_off() {
        let mut station = powered();
        station.powered = false;
        let (_session, body, action) = run("station wlan0 scan", &station);
        assert_eq!(action, Action::None);
        assert_eq!(body, "Operation failed\n");
    }

    #[test]
    fn missing_device() {
        let (_session, body, action) = run("station eth0 scan", &powered());
        assert_eq!(action, Action::None);
        assert_eq!(body, "Device eth0 not found.\n");
    }

    #[test]
    fn get_networks_table() {
        let mut station = with_net(b"lerux", Security::Psk);
        station.networks[1] = Network::new(b"open net", Security::Open).expect("name");
        station.count = 2;
        station.connected = true;
        station.ssid[..5].copy_from_slice(b"lerux");
        station.ssid_len = 5;
        let (_session, body, action) = run("station wlan0 get-networks", &station);
        assert_eq!(action, Action::None);
        assert!(body.contains("Available networks"));
        assert!(body.contains("lerux"));
        assert!(body.contains("psk"));
        assert!(body.contains("open"));
        assert!(body.contains("> "));
        assert!(body.contains("open net"));
    }

    #[test]
    fn get_networks_empty() {
        let (_session, body, _) = run("station wlan0 get-networks", &powered());
        assert!(body.contains("No networks available\n"));
    }

    #[test]
    fn connect_asks_for_passphrase() {
        let station = with_net(b"lerux", Security::Psk);
        let (session, body, action) = run("station wlan0 connect lerux", &station);
        assert_eq!(action, Action::None);
        assert!(session.connect().is_none());
        assert!(body.contains("Type the network passphrase for lerux psk.\n"));
        assert!(body.ends_with("Passphrase: "));
    }

    #[test]
    fn quoted_name_with_space() {
        let station = with_net(b"open net", Security::Psk);
        let (_session, body, _) = run("station wlan0 connect \"open net\"", &station);
        assert!(body.contains("open net psk."));
    }

    #[test]
    fn bad_passphrase_does_not_connect() {
        let station = with_net(b"lerux", Security::Psk);
        let mut session = Session::new();
        let mut buf = [0u8; 512];
        let turn = session.open(b"station wlan0 connect lerux", &station, &mut buf);
        assert!(turn.read_secret);
        let turn = session.secret(b"short", &mut buf);
        assert_eq!(turn.action, Action::None);
        assert!(session.connect().is_none());
        let body = text(&buf, turn.written);
        assert_eq!(
            body,
            "PMK generation failed. Ensure Crypto Engine is properly configured\n"
        );
    }

    #[test]
    fn passphrase_option_connects() {
        let station = with_net(b"lerux", Security::Psk);
        let (session, body, action) = run(
            "--passphrase=correct12 station wlan0 connect lerux",
            &station,
        );
        assert_eq!(body, "");
        assert_eq!(action, Action::Connect);
        let connect = session.connect().expect("key");
        assert_eq!(connect.ssid(), b"lerux");
        assert_eq!(connect.psk(), b"correct12");
    }

    #[test]
    fn dash_p_passphrase() {
        let station = with_net(b"lerux", Security::Psk);
        let (session, _, action) = run("-P correct12 station wlan0 connect lerux", &station);
        assert_eq!(action, Action::Connect);
        assert_eq!(session.connect().expect("key").psk(), b"correct12");
    }

    #[test]
    fn open_network_fails_without_notify() {
        let station = with_net(b"guest", Security::Open);
        let (session, body, action) = run("station wlan0 connect guest", &station);
        assert_eq!(action, Action::None);
        assert!(session.connect().is_none());
        assert_eq!(body, "Operation failed\n");
    }

    #[test]
    fn unknown_network() {
        let (_session, body, action) = run("station wlan0 connect missing", &powered());
        assert_eq!(action, Action::None);
        assert_eq!(body, "Invalid network name 'missing'\n");
    }

    #[test]
    fn ambiguous_network() {
        let mut station = with_net(b"lerux", Security::Psk);
        station.networks[1] = Network::new(b"lerux", Security::Open).expect("name");
        station.count = 2;
        let (_session, body, _) = run("station wlan0 connect lerux", &station);
        assert!(body.contains("ambiguous"));
    }

    #[test]
    fn security_selects_one() {
        let mut station = with_net(b"lerux", Security::Psk);
        station.networks[1] = Network::new(b"lerux", Security::Open).expect("name");
        station.count = 2;
        let (_session, body, _) = run("station wlan0 connect lerux psk", &station);
        assert!(body.contains("Passphrase: "));
    }

    #[test]
    fn disconnect_when_connected() {
        let mut station = powered();
        station.connected = true;
        let (_session, body, action) = run("station wlan0 disconnect", &station);
        assert_eq!(action, Action::Disconnect);
        assert_eq!(body, "");
    }

    #[test]
    fn disconnect_when_idle() {
        let (_session, body, action) = run("station wlan0 disconnect", &powered());
        assert_eq!(action, Action::None);
        assert_eq!(body, "Not connected\n");
    }

    #[test]
    fn hidden_and_known_are_honest() {
        let (_session, hidden, action) = run("station wlan0 get-hidden-access-points", &powered());
        assert_eq!(action, Action::None);
        assert_eq!(hidden, "Hidden networks are not kept\n");
        let (_session, known, _) = run("known-networks list", &powered());
        assert_eq!(known, "Known networks are not stored\n");
        let (_session, forget, _) = run("known-networks lerux forget", &powered());
        assert_eq!(forget, "Known networks are not stored\n");
    }

    #[test]
    fn unsupported_family_command() {
        let (_session, body, _) = run("adapter phy0 set-property Powered off", &powered());
        assert_eq!(body, "Unsupported command\n");
        let (_session, ap, _) = run("ap list", &powered());
        assert_eq!(ap, "Unsupported command\n");
    }

    #[test]
    fn quit_and_exit() {
        let (_session, _, action) = run("quit", &powered());
        assert_eq!(action, Action::Exit);
        let (_session, _, action) = run("exit", &powered());
        assert_eq!(action, Action::Exit);
    }

    #[test]
    fn version_string() {
        let (_session, body, _) = run("version", &powered());
        assert_eq!(body, "IWD version 3.12\n");
    }

    #[test]
    fn help_names_the_station_commands() {
        let (_session, body, _) = run("--help", &powered());
        assert!(body.contains("iwctl version 3.12"));
        assert!(body.contains("station <wlan> scan"));
        assert!(body.contains("Scan for networks"));
        assert!(body.contains("known-networks list"));
        assert!(!body.contains("Miscellaneous"));
    }

    #[test]
    fn interactive_help_includes_misc() {
        let mut session = Session::new();
        let mut buf = [0u8; 16384];
        let opened = session.open(b"", &powered(), &mut buf);
        assert!(opened.interactive);
        let turn = session.command(b"help", &powered(), &mut buf);
        let body = text(&buf, turn.written);
        assert!(body.contains("Miscellaneous"));
        assert!(body.contains("version"));
        assert!(body.contains("Quit program"));
    }

    #[test]
    fn dont_ask_without_passphrase() {
        let station = with_net(b"lerux", Security::Psk);
        let (_session, body, action) = run("--dont-ask station wlan0 connect lerux", &station);
        assert_eq!(action, Action::None);
        assert!(body.contains("No passphrase is provided as '--passphrase' command-line option."));
    }

    #[test]
    fn connect_pattern() {
        let (_session, body, _) = run("station wlan0 connect", &powered());
        assert!(body.contains("Invalid command. Use the following pattern:\n"));
        assert!(body.contains("station <wlan> connect"));
    }

    #[test]
    fn radio_failure() {
        let mut session = Session::new();
        let mut buf = [0u8; 64];
        let turn = session.after_radio(false, &mut buf);
        assert_eq!(text(&buf, turn.written), "Operation failed\n");
        let turn = session.after_radio(true, &mut buf);
        assert_eq!(turn.written, 0);
    }

    #[test]
    fn scan_cap_matches_radio() {
        assert_eq!(SCAN_MAX, 16);
    }

    #[test]
    fn station_show_connected() {
        let mut station = powered();
        station.connected = true;
        station.ssid[..5].copy_from_slice(b"lerux");
        station.ssid_len = 5;
        let (_session, body, _) = run("station wlan0 show", &station);
        assert!(body.contains("connected"));
        assert!(body.contains("lerux"));
        assert!(!body.contains("IPv4"));
    }

    #[test]
    fn adapter_show() {
        let (_session, body, _) = run("adapter phy0 show", &powered());
        assert!(body.contains("MediaTek"));
        assert!(body.contains("MT7921"));
        assert!(body.contains("station "));
    }
}
