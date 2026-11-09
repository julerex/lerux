use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let src = scan::read("userspace/crates/lerux-tls/src/lib.rs")?;
    let drive = scan::one(&src, "drive")?;
    Ok(vec![("EarlyConnected", early(drive)?)])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        let bad = match name {
            "EarlyConnected" => value,
            _ => !value,
        };
        if bad {
            return Err("drive sets connected outside WriteTraffic and ReadTraffic".to_string());
        }
    }
    Ok(())
}

fn early(body: &str) -> Result<bool, String> {
    let needle = "self.connected = true";
    let mut count = 0;
    let mut from = 0;
    while let Some(rel) = body[from..].find(needle) {
        let at = from + rel;
        let window = &body[at.saturating_sub(180)..at];
        if !window.contains("WriteTraffic") && !window.contains("ReadTraffic") {
            return Ok(true);
        }
        count += 1;
        from = at + needle.len();
    }
    if count != 2 {
        return Err(format!(
            "shape: drive has {count} assignments of connected, the model expects 2"
        ));
    }
    Ok(false)
}
