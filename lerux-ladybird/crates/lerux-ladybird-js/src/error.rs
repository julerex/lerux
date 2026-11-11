use core::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Syntax,
    Runtime,
}

/// A script that did not parse or did not finish.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error {
    kind: Kind,
}

impl Error {
    pub(crate) const fn syntax() -> Self {
        Self { kind: Kind::Syntax }
    }

    pub(crate) const fn runtime() -> Self {
        Self {
            kind: Kind::Runtime,
        }
    }

    /// `true` when the script text is outside the language subset.
    #[must_use]
    pub const fn is_syntax(self) -> bool {
        matches!(self.kind, Kind::Syntax)
    }

    /// `true` when the script parsed and the run failed.
    #[must_use]
    pub const fn is_runtime(self) -> bool {
        matches!(self.kind, Kind::Runtime)
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::Syntax => formatter.write_str("syntax error"),
            Kind::Runtime => formatter.write_str("runtime error"),
        }
    }
}

impl core::error::Error for Error {}
