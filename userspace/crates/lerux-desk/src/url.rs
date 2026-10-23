use super::LINE_CAP;

/// Who may treat a hostname as public.
///
/// [`NamePolicy::DemoHost`] allows only `host`. There is no public-suffix list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamePolicy {
    /// A literal address, or the name `host`, is rejected.
    Public,
    /// The single public name is `host`.
    DemoHost,
}

/// Normalized `https` URL. The bytes are what the address line shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HttpsUrl {
    bytes: [u8; LINE_CAP],
    len: u16,
}

impl HttpsUrl {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > LINE_CAP || core::str::from_utf8(bytes).is_err() {
            return None;
        }
        let mut stored = [0u8; LINE_CAP];
        stored[..bytes.len()].copy_from_slice(bytes);
        let len = u16::try_from(bytes.len()).ok()?;
        Some(Self { bytes: stored, len })
    }

    pub fn as_str(&self) -> &str {
        let slice = &self.bytes[..usize::from(self.len)];
        core::str::from_utf8(slice).unwrap_or_default()
    }
}

impl core::fmt::Debug for HttpsUrl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("HttpsUrl").field(&self.as_str()).finish()
    }
}

pub(crate) fn normalize(text: &str, policy: NamePolicy) -> Result<HttpsUrl, ()> {
    let rest = https_rest(text)?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let tail = &rest[end..];
    if authority.is_empty() || authority.contains('@') || authority.starts_with('[') {
        return Err(());
    }
    let (host_raw, port_text) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if host_raw.is_empty() || host_raw.len() > 253 {
        return Err(());
    }
    let mut host_bytes = [0u8; 253];
    for (i, byte) in host_raw.bytes().enumerate() {
        host_bytes[i] = byte.to_ascii_lowercase();
    }
    let host = core::str::from_utf8(&host_bytes[..host_raw.len()]).map_err(|_| ())?;
    if is_ipv4(host) || !is_dns_name(host) {
        return Err(());
    }
    let allowed = match policy {
        NamePolicy::DemoHost => host == "host",
        NamePolicy::Public => host != "host",
    };
    if !allowed {
        return Err(());
    }
    let port = match port_text {
        Some(text) => Some(parse_port(text)?),
        None => None,
    };
    let mut out = [0u8; LINE_CAP];
    let mut n = 0;
    if !push(&mut out, &mut n, b"https://") || !push(&mut out, &mut n, host.as_bytes()) {
        return Err(());
    }
    if let Some(port) = port
        && (!push(&mut out, &mut n, b":") || !push_u16(&mut out, &mut n, port))
    {
        return Err(());
    }
    if !push(&mut out, &mut n, tail.as_bytes()) {
        return Err(());
    }
    HttpsUrl::from_bytes(&out[..n]).ok_or(())
}

fn https_rest(text: &str) -> Result<&str, ()> {
    match text.find("://") {
        None => Ok(text),
        Some(0) => Err(()),
        Some(idx) => {
            let scheme = &text[..idx];
            if scheme.eq_ignore_ascii_case("https") {
                Ok(&text[idx + 3..])
            } else {
                Err(())
            }
        }
    }
}

fn parse_port(text: &str) -> Result<u16, ()> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    let mut value: u32 = 0;
    for byte in text.bytes() {
        value = value * 10 + u32::from(byte - b'0');
    }
    if value == 0 || value > 65535 {
        return Err(());
    }
    u16::try_from(value).map_err(|_| ())
}

fn push(out: &mut [u8], n: &mut usize, bytes: &[u8]) -> bool {
    let Some(slot) = out.get_mut(*n..*n + bytes.len()) else {
        return false;
    };
    slot.copy_from_slice(bytes);
    *n += bytes.len();
    true
}

fn push_u16(out: &mut [u8], n: &mut usize, value: u16) -> bool {
    let mut buf = [0u8; 5];
    let mut rest = value;
    let mut i = buf.len();
    loop {
        i -= 1;
        let Ok(digit) = u8::try_from(rest % 10) else {
            return false;
        };
        buf[i] = b'0' + digit;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    push(out, n, &buf[i..])
}

fn is_ipv4(host: &str) -> bool {
    let mut count = 0;
    for part in host.split('.') {
        if count == 4 || !is_octet(part) {
            return false;
        }
        count += 1;
    }
    count == 4
}

fn is_octet(part: &str) -> bool {
    if part.is_empty() || part.len() > 3 || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let mut value: u16 = 0;
    for byte in part.bytes() {
        value = value * 10 + u16::from(byte - b'0');
    }
    value <= 255
}

fn is_dns_name(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    let mut any = false;
    for label in host.split('.') {
        any = true;
        let bytes = label.as_bytes();
        if bytes.is_empty() || bytes.len() > 63 {
            return false;
        }
        let Some((first, tail)) = bytes.split_first() else {
            return false;
        };
        let Some((last, middle)) = tail.split_last() else {
            return first.is_ascii_alphanumeric();
        };
        if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
            return false;
        }
        if !middle
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            return false;
        }
    }
    any
}
