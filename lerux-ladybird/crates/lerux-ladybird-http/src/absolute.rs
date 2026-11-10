//! Absolute `https` URL bytes stored on the request page.
//!
//! The request target is the path and the query. The fragment is not part of
//! the GET. This module does not resolve a host.

/// Port used when the URL omits one.
pub const HTTPS_PORT: u16 = 443;

pub(crate) struct Https<'a> {
    pub(crate) host: &'a str,
    pub(crate) port: u16,
    pub(crate) target: &'a str,
}

pub(crate) fn parse(url: &str) -> Option<Https<'_>> {
    if breaks_request(url) {
        return None;
    }
    let rest = https_rest(url)?;
    let (authority, tail) = split_authority(rest)?;
    let (host, port) = split_host(authority)?;
    let target = origin_form(tail)?;
    Some(Https { host, port, target })
}

fn breaks_request(url: &str) -> bool {
    url.bytes()
        .any(|byte| byte == 0 || byte.is_ascii_whitespace())
}

fn https_rest(url: &str) -> Option<&str> {
    let (scheme, rest) = url.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    rest.strip_prefix("//")
}

fn split_authority(rest: &str) -> Option<(&str, &str)> {
    if rest.is_empty() {
        return None;
    }
    Some(match rest.find(['/', '?', '#']) {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    })
}

fn split_host(authority: &str) -> Option<(&str, u16)> {
    if authority.is_empty() || authority.contains(['@', '[', ']']) {
        return None;
    }
    let Some((host, port_text)) = authority.rsplit_once(':') else {
        return Some((authority, HTTPS_PORT));
    };
    if host.is_empty() || host.contains(':') {
        return None;
    }
    Some((host, parse_port(port_text)?))
}

fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Origin-form target. An empty path is `/`. A query without a path is rejected.
fn origin_form(tail: &str) -> Option<&str> {
    let before_fragment = tail.split_once('#').map_or(tail, |(head, _)| head);
    if before_fragment.is_empty() {
        return Some("/");
    }
    if before_fragment.starts_with('/') {
        return Some(before_fragment);
    }
    None
}
