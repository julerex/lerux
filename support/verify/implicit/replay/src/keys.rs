use lerux_input::{from_set1, Key, Set1State};
use lerux_swift::PortStop;

use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let input = scan::read("userspace/crates/lerux-input/src/lib.rs")?;
    let set1 = scan::one(&input, "from_set1")?;
    let hid = scan::read("userspace/pds/console-driver/src/hid.rs")?;
    let decode = scan::one(&hid, "decode")?;
    let xhci = scan::read("userspace/pds/console-driver/src/xhci.rs")?;
    let bt = scan::read("userspace/crates/lerux-swift/src/bt.rs")?;
    let later = scan::one(&bt, "later")?;
    Ok(vec![
        ("ClearsExtended", clears_extended(set1)?),
        (
            "SuppressHeld",
            decode.contains("state.down.contains(&usage)"),
        ),
        ("PollChecksReady", poll_checks(&xhci)?),
        ("OnlyForward", later.contains("other.rank() > self.rank()")),
    ])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        if !value {
            return Err(match name {
                "ClearsExtended" => {
                    "from_set1 keeps the extended prefix after the next byte".to_string()
                }
                "SuppressHeld" => "decode repeats a usage that is still held".to_string(),
                "PollChecksReady" => {
                    "UsbKbd::poll emits a key before the controller is ready".to_string()
                }
                "OnlyForward" => "PortStop::later can move to an earlier stage".to_string(),
                _ => format!("{name} is false"),
            });
        }
    }
    letters()?;
    if PortStop::Desc.later(PortStop::Reset) != PortStop::Desc {
        return Err("a later port failure replaced a further stage".to_string());
    }
    if PortStop::Reset.later(PortStop::Addr) != PortStop::Addr {
        return Err("a further port stage did not replace reset".to_string());
    }
    Ok(())
}

fn clears_extended(body: &str) -> Result<bool, String> {
    let marker = "let extended = state.extended;";
    let Some(at) = body.find(marker) else {
        return Err("shape: from_set1 does not capture extended".to_string());
    };
    let after = &body[at + marker.len()..];
    let Some(end) = after.find("if byte & 0x80") else {
        return Err("shape: from_set1 does not test the break bit".to_string());
    };
    Ok(after[..end].contains("state.extended = false"))
}

fn poll_checks(src: &str) -> Result<bool, String> {
    let found = scan::bodies(src, "poll")?;
    let Some(body) = found.into_iter().find(|body| body.contains("pop_event")) else {
        return Err("shape: UsbKbd::poll was not found".to_string());
    };
    Ok(body.contains("if !self.ready"))
}

fn letters() -> Result<(), String> {
    let mut state = Set1State::new();
    if from_set1(&mut state, 0xe0).is_some() {
        return Err("the extended prefix produced a key".to_string());
    }
    if from_set1(&mut state, 0x1e).is_some() {
        return Err("the byte after the prefix produced a key".to_string());
    }
    match from_set1(&mut state, 0x1e) {
        Some(event) if event.key == Key::Char(b'a') && !event.ctrl => Ok(()),
        Some(_) => Err("the second A was not the letter a".to_string()),
        None => Err("the second A was swallowed".to_string()),
    }
}
