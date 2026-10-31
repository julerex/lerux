//! CSS object model for one style sheet.
//!
//! A sheet holds rules. A selector is a sequence of compound selectors in
//! source order. The combinator on a compound joins it to the previous
//! compound, and the first compound has [`Combinator::None`]. Combinators are
//! descendant and child. A compound holds type, class, and id selectors.
//!
//! A declaration stores a property and a value. There is no computed style.
//! A custom property, and any value that still names `var()`, is unresolved.
//! The accepted properties are `color`, `background-color`, `font-size`,
//! `display`, `margin`, `padding`, and `width`. Any other property is kept
//! as an unknown declaration.

use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt::{self, Display, Formatter};

mod parse;

/// One style sheet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleSheet {
    rules: Vec<StyleRule>,
}

impl StyleSheet {
    /// Parse `input` into rules, selectors, and declarations.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        parse::sheet(input)
    }

    pub fn rules(&self) -> &[StyleRule] {
        &self.rules
    }
}

/// One qualified rule: a selector list and its declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyleRule {
    selectors: Vec<Selector>,
    declarations: Vec<Declaration>,
}

impl StyleRule {
    pub fn selectors(&self) -> &[Selector] {
        &self.selectors
    }

    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }
}

/// A sequence of compound selectors in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selector {
    compounds: Vec<CompoundSelector>,
}

impl Selector {
    pub fn compounds(&self) -> &[CompoundSelector] {
        &self.compounds
    }
}

/// How this compound joins the compound before it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Combinator {
    /// The first compound in a selector.
    None,
    /// Whitespace.
    Descendant,
    /// `>`.
    Child,
}

/// One compound selector and the combinator that precedes it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompoundSelector {
    combinator: Combinator,
    simples: Vec<SimpleSelector>,
}

impl CompoundSelector {
    pub fn combinator(&self) -> Combinator {
        self.combinator
    }

    pub fn simples(&self) -> &[SimpleSelector] {
        &self.simples
    }
}

/// Type, class, or id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpleSelector {
    Type(Box<str>),
    Class(Box<str>),
    Id(Box<str>),
}

/// One property and the value as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    property: Property,
    value: Value,
}

impl Declaration {
    pub fn property(&self) -> &Property {
        &self.property
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// A property the parser accepted, a custom property, or an unknown name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Property {
    Color,
    BackgroundColor,
    FontSize,
    Display,
    Margin,
    Padding,
    Width,
    /// The name includes the leading `--` and keeps its case.
    Custom(Box<str>),
    /// Lowercased property name that this milestone does not resolve.
    Unknown(Box<str>),
}

/// A value stored for a later cascade. Nothing here is computed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// The text the sheet wrote, with surrounding whitespace and comments removed.
    Specified(Box<str>),
    /// A custom property, or a value that still names `var()`.
    Unresolved(Box<str>),
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
        f.write_str("invalid style sheet")
    }
}

impl core::error::Error for ParseError {}

fn declaration(name: String, value: String, has_var: bool) -> Declaration {
    let property = property_from_name(name);
    let unresolved = matches!(property, Property::Custom(_)) || has_var;
    let value = if unresolved {
        Value::Unresolved(value.into_boxed_str())
    } else {
        Value::Specified(value.into_boxed_str())
    };
    Declaration { property, value }
}

fn property_from_name(name: String) -> Property {
    if name.starts_with("--") {
        return Property::Custom(name.into_boxed_str());
    }
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "background-color" => Property::BackgroundColor,
        "color" => Property::Color,
        "display" => Property::Display,
        "font-size" => Property::FontSize,
        "margin" => Property::Margin,
        "padding" => Property::Padding,
        "width" => Property::Width,
        _ => Property::Unknown(lower.into_boxed_str()),
    }
}
