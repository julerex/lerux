use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

pub fn read(rel: &str) -> Result<String, String> {
    let path = root().join(rel);
    fs::read_to_string(&path).map_err(|err| format!("cannot read {rel}: {err}"))
}

pub fn bodies<'a>(src: &'a str, name: &str) -> Result<Vec<&'a str>, String> {
    let marker = format!("fn {name}(");
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(rel) = src[from..].find(&marker) {
        let start = from + rel;
        let body = brace_body(src, start).ok_or_else(|| format!("unbalanced braces in {name}"))?;
        found.push(body);
        from = start + marker.len();
    }
    Ok(found)
}

pub fn one<'a>(src: &'a str, name: &str) -> Result<&'a str, String> {
    let found = bodies(src, name)?;
    match found.len() {
        1 => Ok(found[0]),
        0 => Err(format!("no fn {name}")),
        n => Err(format!("{n} fn {name}")),
    }
}

pub fn between<'a>(body: &'a str, start: &str, end: &str) -> Result<&'a str, String> {
    let Some(at) = body.find(start) else {
        return Err(format!("missing `{start}`"));
    };
    let after = at + start.len();
    let Some(stop) = body[after..].find(end) else {
        return Err(format!("missing `{end}` after `{start}`"));
    };
    Ok(&body[after..after + stop])
}

fn brace_body(src: &str, from: usize) -> Option<&str> {
    let bytes = src.as_bytes();
    let mut i = from;
    while i < bytes.len() && bytes[i] != b'{' {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let open = i;
    let mut depth = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }
            b'"' => {
                i = skip_string(bytes, i)?;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[open + 1..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn skip_string(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::one;

    #[test]
    fn a_function_body_stops_at_its_own_brace() {
        let src = "fn outer() {\n    if ready {\n        return;\n    }\n}\nfn next() {}\n";
        let body = one(src, "outer").expect("outer");
        assert!(body.contains("return"));
        assert!(!body.contains("fn next"));
    }
}
