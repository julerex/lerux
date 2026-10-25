use alloc::{string::String, vec::Vec};
use core::str::FromStr;

use super::{accepted_default_port, ParseError, Url};

pub(super) fn basic_parse(input: &str, base: Option<&Url>) -> Result<Url, ParseError> {
    if input.is_empty() {
        return base.cloned().ok_or(ParseError::new());
    }
    if let Some((scheme, rest)) = scheme_and_rest(input) {
        return parse_absolute(scheme, rest);
    }
    let Some(base) = base else {
        return Err(ParseError::new());
    };
    parse_relative(input, base)
}

fn parse_absolute(scheme_raw: &str, rest: &str) -> Result<Url, ParseError> {
    let scheme = scheme_raw.to_ascii_lowercase();
    let default_port = accepted_default_port(&scheme)?;
    let Some(after_slashes) = rest.strip_prefix("//") else {
        return Err(ParseError::new());
    };
    let (authority, tail) = split_authority(after_slashes);
    let (host, port) = parse_authority(authority, default_port)?;
    let (path_text, query, fragment) = split_tail(tail);
    if !path_text.is_empty() && !path_text.starts_with('/') {
        return Err(ParseError::new());
    }
    let path = remove_dot_segments(if path_text.is_empty() { "/" } else { path_text });
    Ok(Url::from_parsed(
        scheme,
        host,
        port,
        path,
        owned(query),
        owned(fragment),
    ))
}

fn parse_relative(input: &str, base: &Url) -> Result<Url, ParseError> {
    if let Some(fragment) = input.strip_prefix('#') {
        return Ok(Url::from_parsed(
            base.scheme.clone(),
            base.host.clone(),
            base.port,
            base.path.clone(),
            base.query.clone(),
            Some(String::from(fragment)),
        ));
    }
    let (path_text, query, fragment) = split_tail(input);
    let path = if path_text.is_empty() {
        base.path.clone()
    } else if path_text.starts_with('/') {
        remove_dot_segments(path_text)
    } else {
        remove_dot_segments(&merge_paths(&base.path, path_text))
    };
    Ok(Url::from_parsed(
        base.scheme.clone(),
        base.host.clone(),
        base.port,
        path,
        owned(query),
        owned(fragment),
    ))
}

fn scheme_and_rest(input: &str) -> Option<(&str, &str)> {
    let mut bytes = input.bytes();
    let first = bytes.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let mut length = 1;
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.') {
            length += 1;
            continue;
        }
        if byte == b':' {
            return Some((&input[..length], &input[length + 1..]));
        }
        return None;
    }
    None
}

fn split_authority(input: &str) -> (&str, &str) {
    match input.find(['/', '?', '#']) {
        Some(end) => (&input[..end], &input[end..]),
        None => (input, ""),
    }
}

fn parse_authority(authority: &str, default_port: u16) -> Result<(String, u16), ParseError> {
    if authority.is_empty() || authority.contains(['@', '[', ']']) {
        return Err(ParseError::new());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port_text)) => (host, parse_port(port_text)?),
        None => (authority, default_port),
    };
    if host.is_empty() || host.contains(':') {
        return Err(ParseError::new());
    }
    Ok((host.to_ascii_lowercase(), port))
}

fn parse_port(text: &str) -> Result<u16, ParseError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::new());
    }
    u16::from_str(text).map_err(|_| ParseError::new())
}

fn split_tail(input: &str) -> (&str, Option<&str>, Option<&str>) {
    let (before_fragment, fragment) = match input.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (input, None),
    };
    let (path, query) = match before_fragment.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (before_fragment, None),
    };
    (path, query, fragment)
}

fn owned(part: Option<&str>) -> Option<String> {
    part.map(String::from)
}

fn merge_paths(base_path: &str, relative: &str) -> String {
    let directory = match base_path.rfind('/') {
        Some(slash) => &base_path[..=slash],
        None => "/",
    };
    let mut merged = String::with_capacity(directory.len() + relative.len());
    merged.push_str(directory);
    merged.push_str(relative);
    merged
}

fn remove_dot_segments(path: &str) -> String {
    debug_assert!(path.starts_with('/'));
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/').skip(1) {
        match segment {
            "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    if ends_with_dot_segment(path) {
        push_directory_slash(&mut segments);
    }
    let mut normalized = String::from("/");
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            normalized.push('/');
        }
        normalized.push_str(segment);
    }
    normalized
}

fn ends_with_dot_segment(path: &str) -> bool {
    matches!(path.rsplit_once('/'), Some((_, "." | "..")))
}

fn push_directory_slash(segments: &mut Vec<&str>) {
    if segments.last().is_some_and(|segment| !segment.is_empty()) {
        segments.push("");
    }
}
