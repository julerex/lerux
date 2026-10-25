use alloc::string::String;
use core::{
    fmt::{self, Display, Formatter},
    write,
};

mod parse;

const HTTPS_DEFAULT_PORT: u16 = 443;

struct SchemeSpec {
    name: &'static str,
    default_port: Option<u16>,
    accepted: bool,
}

const SCHEMES: &[SchemeSpec] = &[
    SchemeSpec {
        name: "https",
        default_port: Some(HTTPS_DEFAULT_PORT),
        accepted: true,
    },
    SchemeSpec {
        name: "http",
        default_port: Some(80),
        accepted: false,
    },
];

fn scheme_spec(scheme: &str) -> Option<&'static SchemeSpec> {
    SCHEMES.iter().find(|spec| spec.name == scheme)
}

fn default_port(scheme: &str) -> Option<u16> {
    scheme_spec(scheme).and_then(|spec| spec.default_port)
}

fn accepted_default_port(scheme: &str) -> Result<u16, ParseError> {
    let Some(spec) = scheme_spec(scheme) else {
        return Err(ParseError::new());
    };
    if !spec.accepted {
        return Err(ParseError::new());
    }
    spec.default_port.ok_or(ParseError::new())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Url {
    scheme: String,
    host: String,
    port: u16,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Url {
    /// Absolute URL when `base` is `None`.
    ///
    /// `Some` resolves a relative reference, including an empty input, a query,
    /// or a fragment.
    pub fn parse(input: &str, base: Option<&Url>) -> Result<Self, ParseError> {
        parse::basic_parse(input, base)
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    /// Port stored for this URL. An omitted `https` port and an explicit `:443` are both 443.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// Bytes after `?`, when a query is present. The `?` is not included.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Bytes after `#`, when a fragment is present. The `#` is not included.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Path and query borrowed from this URL.
    pub fn request_target(&self) -> RequestTarget<'_> {
        RequestTarget {
            path: &self.path,
            query: self.query.as_deref(),
        }
    }

    fn from_parsed(
        scheme: String,
        host: String,
        port: u16,
        path: String,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        debug_assert!(!host.is_empty());
        debug_assert!(path.starts_with('/'));
        Self {
            scheme,
            host,
            port,
            path,
            query,
            fragment,
        }
    }
}

/// Path and query borrowed from a [`Url`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestTarget<'a> {
    path: &'a str,
    query: Option<&'a str>,
}

fn write_path_and_query(path: &str, query: Option<&str>, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(path)?;
    if let Some(query) = query {
        f.write_str("?")?;
        f.write_str(query)?;
    }
    Ok(())
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.scheme)?;
        f.write_str("://")?;
        f.write_str(&self.host)?;
        if default_port(&self.scheme) != Some(self.port) {
            let port = self.port;
            write!(f, ":{port}")?;
        }
        write_path_and_query(&self.path, self.query.as_deref(), f)?;
        if let Some(fragment) = &self.fragment {
            f.write_str("#")?;
            f.write_str(fragment)?;
        }
        Ok(())
    }
}

impl Display for RequestTarget<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_path_and_query(self.path, self.query, f)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseError {
    _private: (),
}

impl ParseError {
    const fn new() -> Self {
        Self { _private: () }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("invalid url")
    }
}

impl core::error::Error for ParseError {}
