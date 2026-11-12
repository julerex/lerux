use lerux_swift::{bring_up, key_ok, Nic, WifiReport};

use crate::scan;

struct Silent {
    present: bool,
    firmware: bool,
}

impl Nic for Silent {
    fn present(&self) -> bool {
        self.present
    }

    fn firmware(&self) -> bool {
        self.firmware
    }

    fn tx(&mut self, _frame: &[u8]) -> bool {
        true
    }

    fn rx(&mut self) -> Option<&[u8]> {
        None
    }
}

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let join = scan::read("userspace/pds/swift-mt7921/src/join.rs")?;
    let associate = scan::one(&join, "associate")?;
    let wifi = scan::read("userspace/crates/lerux-swift/src/wifi.rs")?;
    let host = scan::one(&wifi, "associate")?;
    let console = scan::read("userspace/pds/console-shell/src/main.rs")?;
    let psk = scan::one(&console, "dispatch_psk")?;
    let device = scan::read("userspace/pds/swift-mt7921/src/device.rs")?;
    Ok(vec![
        ("OwnLabel", own_label(associate)),
        ("EapolChecksReply", host.contains("nic.rx() != Some(&[1])")),
        ("KeyChecked", key_checked(psk)?),
        ("FirmwareWaits", device.contains("mac.stage == Stage::Wait")),
    ])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        if !value {
            return Err(match name {
                "OwnLabel" => {
                    "Mac::associate does not return the stage it just painted".to_string()
                }
                "EapolChecksReply" => {
                    "wifi::associate accepts a handshake reply other than 1".to_string()
                }
                "KeyChecked" => "dispatch_psk notifies the radio before key_ok".to_string(),
                "FirmwareWaits" => "ready is not gated on Stage::Wait".to_string(),
                _ => format!("{name} is false"),
            });
        }
    }
    host_calls()
}

fn key_checked(body: &str) -> Result<bool, String> {
    let Some(post) = body.find("post_wifi(") else {
        return Err("shape: dispatch_psk does not call post_wifi".to_string());
    };
    match body.find("key_ok(") {
        Some(key) => Ok(key < post),
        None => Ok(false),
    }
}

fn own_label(body: &str) -> bool {
    let stages = ["Scan", "Auth", "Assoc", "Eapol"];
    let mut from = 0;
    for stage in stages {
        let paint = format!("paint(Stage::{stage})");
        let ret = format!("return Stage::{stage}");
        let Some(rel) = body[from..].find(&paint) else {
            return false;
        };
        let at = from + rel;
        let after = at + paint.len();
        let window_end = body[after..]
            .find("paint(Stage::")
            .map_or(body.len(), |next| after + next);
        if !body[at..window_end].contains(&ret) {
            return false;
        }
        from = after;
    }
    // The handshake ends on `Wait`. A failed scan that returns `Wait` misses
    // `return Stage::Scan` above, so this line cannot hide that failure.
    body[from..].contains("Stage::Wait")
}

fn host_calls() -> Result<(), String> {
    if key_ok(b"net", b"short") {
        return Err("key_ok accepted a passphrase shorter than 8 bytes".to_string());
    }
    if !key_ok(b"net", b"password") {
        return Err("key_ok rejected an 8 byte passphrase".to_string());
    }
    if key_ok(b"", b"password") {
        return Err("key_ok accepted an empty network name".to_string());
    }
    expect(
        bring_up(
            &mut Silent {
                present: false,
                firmware: true,
            },
            b"net",
            b"password",
        ),
        "absent",
        "a missing radio did not stay absent",
    )?;
    expect(
        bring_up(
            &mut Silent {
                present: true,
                firmware: false,
            },
            b"net",
            b"password",
        ),
        "firmware",
        "missing firmware did not stop before the scan",
    )?;
    expect(
        bring_up(
            &mut Silent {
                present: true,
                firmware: true,
            },
            b"net",
            b"",
        ),
        "key",
        "an empty passphrase started the radio",
    )?;
    expect(
        bring_up(
            &mut Silent {
                present: true,
                firmware: true,
            },
            b"net",
            b"password",
        ),
        "radio",
        "a handshake with no reply finished as a successful ping",
    )?;
    Ok(())
}

fn expect(report: WifiReport, want: &str, fail: &str) -> Result<(), String> {
    let got = match report {
        WifiReport::Absent => "absent",
        WifiReport::FirmwareErr => "firmware",
        WifiReport::KeyErr => "key",
        WifiReport::RadioErr => "radio",
        WifiReport::Bar => "bar",
        WifiReport::PingOk(_) => "ping",
    };
    if got == want {
        Ok(())
    } else {
        Err(format!("{fail} ({got})"))
    }
}
