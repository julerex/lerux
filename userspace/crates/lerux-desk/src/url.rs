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
    let url = normalize_structure(text)?;
    if policy_allows(&url, policy) {
        Ok(url)
    } else {
        Err(())
    }
}

fn policy_allows(url: &HttpsUrl, policy: NamePolicy) -> bool {
    let Ok(host) = host_from_url(url.as_str()) else {
        return false;
    };
    match policy {
        NamePolicy::DemoHost => host == "host",
        NamePolicy::Public => host != "host",
    }
}

fn host_from_url(text: &str) -> Result<&str, ()> {
    const PREFIX: &str = "https://";
    if !text.starts_with(PREFIX) {
        return Err(());
    }
    let rest = &text[PREFIX.len()..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let (host, _) = match authority.rsplit_once(':') {
        Some((host, _)) => (host, ()),
        None => (authority, ()),
    };
    if host.is_empty() {
        return Err(());
    }
    Ok(host)
}

fn normalize_structure(text: &str) -> Result<HttpsUrl, ()> {
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

/// Navigation decision for a focused link href against a loaded page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HrefIntent {
    Fetch(HttpsUrl),
    Fragment(HttpsUrl),
    Already,
    Script,
    Rejected,
}

/// Resolve `href` against `base`. Join and scheme checks only; [`NamePolicy`] runs in activation.
pub(crate) fn resolve_href(base: &HttpsUrl, href: &str) -> HrefIntent {
    let href = trim_ascii(href);
    if href.is_empty() {
        return empty_href_intent(base);
    }
    if href.len() >= "javascript:".len()
        && href[.."javascript:".len()].eq_ignore_ascii_case("javascript:")
    {
        return HrefIntent::Script;
    }
    if let Some(idx) = href.find("://") {
        let scheme = &href[..idx];
        if !scheme.eq_ignore_ascii_case("https") {
            return HrefIntent::Rejected;
        }
        return finish_intent(base, href);
    }
    if href.starts_with("//") {
        let mut absolute = [0u8; LINE_CAP + 6];
        absolute[..6].copy_from_slice(b"https:");
        let tail = href.as_bytes();
        if tail.len() + 6 > absolute.len() {
            return HrefIntent::Rejected;
        }
        absolute[6..6 + tail.len()].copy_from_slice(tail);
        let text = core::str::from_utf8(&absolute[..6 + tail.len()]).unwrap_or("");
        return finish_intent(base, text);
    }
    match join_to_base(base, href) {
        Ok(text) => finish_intent(base, text.as_str()),
        Err(()) => HrefIntent::Rejected,
    }
}

fn finish_intent(base: &HttpsUrl, absolute: &str) -> HrefIntent {
    let Ok(url) = normalize_structure(absolute) else {
        return HrefIntent::Rejected;
    };
    classify_against_base(base, &url)
}

fn empty_href_intent(base: &HttpsUrl) -> HrefIntent {
    let Ok(parts) = split_url(base.as_str()) else {
        return HrefIntent::Rejected;
    };
    if parts.fragment.is_none() {
        return HrefIntent::Already;
    }
    match render_url(parts.authority, parts.path, parts.query, None) {
        Ok(url) => HrefIntent::Fragment(url),
        Err(()) => HrefIntent::Rejected,
    }
}

fn classify_against_base(base: &HttpsUrl, url: &HttpsUrl) -> HrefIntent {
    if url.as_str() == base.as_str() {
        return HrefIntent::Already;
    }
    let Ok(base_parts) = split_url(base.as_str()) else {
        return HrefIntent::Fetch(*url);
    };
    let Ok(url_parts) = split_url(url.as_str()) else {
        return HrefIntent::Fetch(*url);
    };
    if base_parts.authority == url_parts.authority
        && base_parts.path == url_parts.path
        && base_parts.query == url_parts.query
    {
        if base_parts.fragment.is_none() && url_parts.fragment.is_none() {
            return HrefIntent::Already;
        }
        return HrefIntent::Fragment(*url);
    }
    HrefIntent::Fetch(*url)
}

struct UrlParts<'a> {
    authority: &'a str,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

fn split_url(text: &str) -> Result<UrlParts<'_>, ()> {
    const PREFIX: &str = "https://";
    if !text.starts_with(PREFIX) {
        return Err(());
    }
    let rest = &text[PREFIX.len()..];
    let auth_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..auth_end];
    let tail = &rest[auth_end..];
    let (path, query, fragment) = split_path_query_fragment(tail)?;
    Ok(UrlParts {
        authority,
        path,
        query,
        fragment,
    })
}

fn split_path_query_fragment(tail: &str) -> Result<(&str, Option<&str>, Option<&str>), ()> {
    if tail.is_empty() {
        return Ok(("/", None, None));
    }
    let path_end = tail.find(['?', '#']).unwrap_or(tail.len());
    let path = if path_end == 0 {
        "/"
    } else {
        &tail[..path_end]
    };
    let mut query = None;
    let mut fragment = None;
    let mut rest = &tail[path_end..];
    if let Some(after) = rest.strip_prefix('?') {
        rest = after;
        let q_end = rest.find('#').unwrap_or(rest.len());
        query = Some(&rest[..q_end]);
        rest = &rest[q_end..];
    }
    if let Some(after) = rest.strip_prefix('#') {
        fragment = Some(after);
    }
    Ok((path, query, fragment))
}

fn join_to_base(base: &HttpsUrl, href: &str) -> Result<HttpsUrl, ()> {
    let parts = split_url(base.as_str())?;
    let (href_path, href_query, href_fragment) = split_href_tail(href);
    let path_url = if href.starts_with('#') || href.starts_with('?') {
        None
    } else if href.starts_with('/') {
        Some(normalize_path_segments(
            href_path,
            href_path.ends_with('/'),
        )?)
    } else {
        Some(merge_relative_path(parts.path, href_path)?)
    };
    let path = path_url
        .as_ref()
        .map(HttpsUrl::as_str)
        .unwrap_or(parts.path);
    // A `#` reference keeps the base query. A path reference carries only its own query.
    let query = if href.starts_with('#') {
        parts.query
    } else if href.contains('?') {
        href_query
    } else {
        None
    };
    let fragment = if href.contains('#') {
        href_fragment
    } else {
        None
    };
    render_url(parts.authority, path, query, fragment)
}

fn split_href_tail(href: &str) -> (&str, Option<&str>, Option<&str>) {
    if let Some(fragment) = href.strip_prefix('#') {
        return ("/", None, Some(fragment));
    }
    if let Some(rest) = href.strip_prefix('?') {
        let f = rest.find('#').map(|i| &rest[i + 1..]);
        let q_end = rest.find('#').unwrap_or(rest.len());
        return ("/", Some(&rest[..q_end]), f);
    }
    let path_end = href.find(['?', '#']).unwrap_or(href.len());
    let path = &href[..path_end];
    let mut rest = &href[path_end..];
    let mut query = None;
    let mut fragment = None;
    if let Some(after) = rest.strip_prefix('?') {
        rest = after;
        let q_end = rest.find('#').unwrap_or(rest.len());
        query = Some(&rest[..q_end]);
        rest = &rest[q_end..];
    }
    if let Some(after) = rest.strip_prefix('#') {
        fragment = Some(after);
    }
    (path, query, fragment)
}

fn merge_relative_path(base_path: &str, href_path: &str) -> Result<HttpsUrl, ()> {
    let dir = base_directory(base_path);
    let trailing = href_path.ends_with('/');
    let merged = merge_path_segments(dir, href_path)?;
    normalize_path_segments(merged.as_str(), trailing)
}

fn base_directory(path: &str) -> &str {
    if path == "/" {
        return "/";
    }
    match path.rfind('/') {
        Some(0) => "/",
        Some(idx) => &path[..=idx],
        None => "/",
    }
}

fn merge_path_segments(base_dir: &str, href_path: &str) -> Result<HttpsUrl, ()> {
    let mut segments: [&str; 64] = [""; 64];
    let mut count = 0usize;

    if base_dir != "/" {
        for seg in base_dir.trim_matches('/').split('/') {
            push_path_segment(seg, &mut segments, &mut count)?;
        }
    }
    for seg in href_path.split('/') {
        push_path_segment(seg, &mut segments, &mut count)?;
    }

    let mut out = [0u8; LINE_CAP];
    let mut n = 0usize;
    if !push(&mut out, &mut n, b"/") {
        return Err(());
    }
    if count == 0 {
        return HttpsUrl::from_bytes(&out[..1]).ok_or(());
    }
    for (i, segment) in segments.iter().take(count).enumerate() {
        if !push(&mut out, &mut n, segment.as_bytes()) {
            return Err(());
        }
        if i + 1 < count && !push(&mut out, &mut n, b"/") {
            return Err(());
        }
    }
    HttpsUrl::from_bytes(&out[..n]).ok_or(())
}

fn push_path_segment<'a>(
    seg: &'a str,
    segments: &mut [&'a str; 64],
    count: &mut usize,
) -> Result<(), ()> {
    if seg.is_empty() || seg == "." {
        return Ok(());
    }
    if seg == ".." {
        *count = count.saturating_sub(1);
        return Ok(());
    }
    if *count >= segments.len() {
        return Err(());
    }
    segments[*count] = seg;
    *count += 1;
    Ok(())
}

fn normalize_path_segments(path: &str, keep_trailing_slash: bool) -> Result<HttpsUrl, ()> {
    let trailing = keep_trailing_slash || path.ends_with('/');
    let mut segments: [&str; 64] = [""; 64];
    let mut count = 0usize;
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            count = count.saturating_sub(1);
            continue;
        }
        if count >= segments.len() {
            return Err(());
        }
        segments[count] = seg;
        count += 1;
    }
    let mut out = [0u8; LINE_CAP];
    let mut n = 0usize;
    if !push(&mut out, &mut n, b"/") {
        return Err(());
    }
    if count == 0 {
        return HttpsUrl::from_bytes(&out[..1]).ok_or(());
    }
    for (i, segment) in segments.iter().take(count).enumerate() {
        if !push(&mut out, &mut n, segment.as_bytes()) {
            return Err(());
        }
        if i + 1 < count && !push(&mut out, &mut n, b"/") {
            return Err(());
        }
    }
    if trailing && !out[..n].ends_with(b"/") && !push(&mut out, &mut n, b"/") {
        return Err(());
    }
    HttpsUrl::from_bytes(&out[..n]).ok_or(())
}

fn render_url(
    authority: &str,
    path: &str,
    query: Option<&str>,
    fragment: Option<&str>,
) -> Result<HttpsUrl, ()> {
    let mut out = [0u8; LINE_CAP];
    let mut n = 0;
    if !push(&mut out, &mut n, b"https://")
        || !push(&mut out, &mut n, authority.as_bytes())
        || !push(&mut out, &mut n, path.as_bytes())
    {
        return Err(());
    }
    if let Some(q) = query
        && (!push(&mut out, &mut n, b"?") || !push(&mut out, &mut n, q.as_bytes()))
    {
        return Err(());
    }
    if let Some(f) = fragment
        && !f.is_empty()
        && (!push(&mut out, &mut n, b"#") || !push(&mut out, &mut n, f.as_bytes()))
    {
        return Err(());
    }
    HttpsUrl::from_bytes(&out[..n]).ok_or(())
}

fn trim_ascii(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_ascii_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> HttpsUrl {
        normalize("https://host:8443/fixture.html", NamePolicy::DemoHost).expect("fixture base")
    }

    fn intent(href: &str) -> HrefIntent {
        resolve_href(&base(), href)
    }

    #[test]
    fn join_rows_desk_link_and_abs() {
        match intent("desk-link.html") {
            HrefIntent::Fetch(url) => {
                assert_eq!(url.as_str(), "https://host:8443/desk-link.html");
            }
            other => panic!("expected fetch, got {other:?}"),
        }
        match intent("/other.html") {
            HrefIntent::Fetch(url) => assert_eq!(url.as_str(), "https://host:8443/other.html"),
            other => panic!("expected fetch, got {other:?}"),
        }
    }

    #[test]
    fn join_rows_relative_query_fragment() {
        match intent("../x") {
            HrefIntent::Fetch(url) => assert_eq!(url.as_str(), "https://host:8443/x"),
            other => panic!("expected fetch, got {other:?}"),
        }
        match intent("?q=1") {
            HrefIntent::Fetch(url) => {
                assert_eq!(url.as_str(), "https://host:8443/fixture.html?q=1")
            }
            other => panic!("expected fetch, got {other:?}"),
        }
        match intent("#section") {
            HrefIntent::Fragment(url) => {
                assert_eq!(url.as_str(), "https://host:8443/fixture.html#section");
            }
            other => panic!("expected fragment, got {other:?}"),
        }
    }

    #[test]
    fn join_rows_script_and_http() {
        assert_eq!(intent("javascript:void(0)"), HrefIntent::Script);
        assert_eq!(intent("http://host/no"), HrefIntent::Rejected);
    }

    #[test]
    fn path_href_drops_base_query_and_fragment() {
        let base = normalize(
            "https://host:8443/dir/fixture.html?keep=1#old",
            NamePolicy::DemoHost,
        )
        .expect("base with a query and a fragment");
        match resolve_href(&base, "desk-link.html") {
            HrefIntent::Fetch(url) => {
                assert_eq!(url.as_str(), "https://host:8443/dir/desk-link.html");
            }
            other => panic!("expected fetch, got {other:?}"),
        }
        match resolve_href(&base, "?q=1") {
            HrefIntent::Fetch(url) => {
                assert_eq!(url.as_str(), "https://host:8443/dir/fixture.html?q=1");
            }
            other => panic!("expected fetch, got {other:?}"),
        }
        match resolve_href(&base, "#section") {
            HrefIntent::Fragment(url) => {
                assert_eq!(
                    url.as_str(),
                    "https://host:8443/dir/fixture.html?keep=1#section"
                );
            }
            other => panic!("expected fragment, got {other:?}"),
        }
    }
}
