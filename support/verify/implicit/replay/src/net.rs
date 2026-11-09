use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let shell = scan::read("userspace/pds/sddf-shell/src/net.rs")?;
    let agent = scan::read("userspace/pds/sddf-agent/src/net.rs")?;
    let client = scan::read("userspace/pds/sddf-net-client/src/main.rs")?;
    let http = scan::read("userspace/pds/http-server/src/net.rs")?;
    let hello = scan::read("userspace/pds/hello/src/net.rs")?;
    let http_poll = poll_with(&http, "self.served")?;
    let udp = scan::one(&hello, "poll_udp_tx")?;
    let done = scan::one(&hello, "is_done")?;
    Ok(vec![
        (
            "IgnoresDeconfigured",
            ignores(&shell) && ignores(&agent) && ignores(&client),
        ),
        (
            "ConnectClearsListen",
            clears(
                &shell,
                "connect",
                "self.connecting = true",
                "self.listening = false",
            )? && clears(
                &agent,
                "connect",
                "self.connecting = true",
                "self.listening = false",
            )?,
        ),
        (
            "ListenClearsConnect",
            clears(
                &shell,
                "listen",
                "self.listening = true",
                "self.connecting = false",
            )? && clears(
                &agent,
                "listen",
                "self.listening = true",
                "self.connecting = false",
            )?,
        ),
        (
            "HttpStops",
            http_poll.contains("if !self.served") && http_poll.contains("if self.served"),
        ),
        (
            "OnceStops",
            udp.contains("if self.udp.tx_done")
                && done.contains("self.udp.tx_done")
                && done.contains("self.tcp.rx_done"),
        ),
    ])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        if !value {
            return Err(match name {
                "IgnoresDeconfigured" => {
                    "take_dhcp installs an address on Deconfigured".to_string()
                }
                "ConnectClearsListen" => "connect leaves listening set".to_string(),
                "ListenClearsConnect" => "listen leaves connecting set".to_string(),
                "HttpStops" => "the HTTP server keeps serving after the first page".to_string(),
                "OnceStops" => "the hello datagram can be sent again".to_string(),
                _ => format!("{name} is false"),
            });
        }
    }
    Ok(())
}

fn ignores(src: &str) -> bool {
    src.contains("Some(Event::Deconfigured) | None => None")
}

fn clears(src: &str, name: &str, sets: &str, clears_this: &str) -> Result<bool, String> {
    let found = scan::bodies(src, name)?;
    if found.is_empty() {
        return Err(format!("shape: no fn {name}"));
    }
    Ok(found
        .iter()
        .all(|body| body.contains(sets) && body.contains(clears_this)))
}

fn poll_with<'a>(src: &'a str, needle: &str) -> Result<&'a str, String> {
    let found = scan::bodies(src, "poll")?;
    found
        .into_iter()
        .find(|body| body.contains(needle))
        .ok_or_else(|| format!("shape: no poll mentions {needle}"))
}
