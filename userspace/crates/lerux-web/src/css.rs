//! Subset CSS parser: type selectors, the Phase 75 properties, px/em lengths.

use alloc::{string::String, vec::Vec};

use lerux_html::TagName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisplayKind {
    Block,
    Inline,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Length {
    Px(i32),
    Em(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selector {
    Star,
    Tag(TagName),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Property {
    Color,
    BackgroundColor,
    FontSize,
    Display,
    Margin,
    Padding,
    Width,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Color(u32),
    Display(DisplayKind),
    WidthAuto,
    Lengths(Vec<Length>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decl {
    pub prop: Property,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rule {
    pub selectors: Vec<Selector>,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Stylesheet {
    pub rules: Vec<Rule>,
}

pub(crate) fn parse_stylesheet(input: &str) -> Stylesheet {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let mut rules = Vec::new();
    loop {
        p.skip_ws_and_comments();
        if p.eof() {
            break;
        }
        if p.peek() == Some(b'@') {
            p.skip_at_rule();
            continue;
        }
        let Some(rule) = p.take_rule() else {
            if !p.bump() {
                break;
            }
            continue;
        };
        if !rule.selectors.is_empty() && !rule.decls.is_empty() {
            rules.push(rule);
        }
    }
    Stylesheet { rules }
}

pub(crate) fn parse_inline_decls(input: &str) -> Vec<Decl> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    p.take_decls_until(None)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> bool {
        if self.eof() {
            return false;
        }
        self.pos += 1;
        true
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
            if self.starts(b"/*") {
                self.pos += 2;
                while !self.eof() && !self.starts(b"*/") {
                    self.pos += 1;
                }
                if self.starts(b"*/") {
                    self.pos += 2;
                }
                continue;
            }
            break;
        }
    }

    fn starts(&self, prefix: &[u8]) -> bool {
        let rest = &self.bytes[self.pos.min(self.bytes.len())..];
        rest.starts_with(prefix)
    }

    fn skip_at_rule(&mut self) {
        while let Some(b) = self.peek() {
            if b == b'{' {
                self.skip_balanced();
                return;
            }
            if b == b';' {
                self.pos += 1;
                return;
            }
            self.pos += 1;
        }
    }

    fn skip_balanced(&mut self) {
        let mut depth = 0;
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'{' => depth += 1,
                b'}' if depth <= 1 => return,
                b'}' => depth -= 1,
                _ => {}
            }
        }
    }

    fn take_rule(&mut self) -> Option<Rule> {
        let selectors = self.take_selectors()?;
        self.skip_ws_and_comments();
        if self.peek() != Some(b'{') {
            return None;
        }
        self.pos += 1;
        let decls = self.take_decls_until(Some(b'}'));
        if self.peek() == Some(b'}') {
            self.pos += 1;
        }
        Some(Rule { selectors, decls })
    }

    fn take_selectors(&mut self) -> Option<Vec<Selector>> {
        let mut out = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let Some(sel) = self.take_selector() else {
                break;
            };
            out.push(sel);
            self.skip_ws_and_comments();
            if self.peek() == Some(b',') {
                self.pos += 1;
                continue;
            }
            break;
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn take_selector(&mut self) -> Option<Selector> {
        if self.peek() == Some(b'*') {
            self.pos += 1;
            return Some(Selector::Star);
        }
        let name = self.take_ident()?;
        TagName::from_ascii(name.as_bytes()).map(Selector::Tag)
    }

    fn take_decls_until(&mut self, end: Option<u8>) -> Vec<Decl> {
        let mut decls = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.eof() {
                break;
            }
            if let Some(e) = end
                && self.peek() == Some(e)
            {
                break;
            }
            let Some(decl) = self.take_decl() else {
                if self.peek() == Some(b';') {
                    self.pos += 1;
                    continue;
                }
                break;
            };
            decls.push(decl);
            self.skip_ws_and_comments();
            if self.peek() == Some(b';') {
                self.pos += 1;
            }
        }
        decls
    }

    fn take_decl(&mut self) -> Option<Decl> {
        let name = self.take_ident()?;
        self.skip_ws_and_comments();
        if self.peek() != Some(b':') {
            return None;
        }
        self.pos += 1;
        let value = self.take_value_text();
        let prop = property_from_name(&name)?;
        let value = parse_value(prop, &value)?;
        Some(Decl { prop, value })
    }

    fn take_ident(&mut self) -> Option<String> {
        self.skip_ws_and_comments();
        let start = self.pos;
        let first = self.peek()?;
        if !first.is_ascii_alphabetic() && first != b'_' && first != b'-' {
            return None;
        }
        self.pos += 1;
        while matches!(self.peek(), Some(b) if b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            self.pos += 1;
        }
        let mut s = String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned();
        s.make_ascii_lowercase();
        Some(s)
    }

    fn take_value_text(&mut self) -> String {
        self.skip_ws_and_comments();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b';' || b == b'}' {
                break;
            }
            self.pos += 1;
        }
        let mut s = String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned();
        if let Some(i) = s.find("!important") {
            s.truncate(i);
        }
        s.truncate(s.trim_end().len());
        s
    }
}

fn property_from_name(name: &str) -> Option<Property> {
    match name {
        "color" => Some(Property::Color),
        "background-color" => Some(Property::BackgroundColor),
        "font-size" => Some(Property::FontSize),
        "display" => Some(Property::Display),
        "margin" => Some(Property::Margin),
        "padding" => Some(Property::Padding),
        "width" => Some(Property::Width),
        _ => None,
    }
}

fn parse_value(prop: Property, raw: &str) -> Option<Value> {
    let raw = raw.trim();
    match prop {
        Property::Color | Property::BackgroundColor => parse_color(raw).map(Value::Color),
        Property::Display => match raw {
            "block" => Some(Value::Display(DisplayKind::Block)),
            "inline" => Some(Value::Display(DisplayKind::Inline)),
            "none" => Some(Value::Display(DisplayKind::None)),
            _ => None,
        },
        Property::Width if raw.eq_ignore_ascii_case("auto") => Some(Value::WidthAuto),
        Property::Width | Property::FontSize => {
            parse_length(raw).map(|l| Value::Lengths(alloc::vec![l]))
        }
        Property::Margin | Property::Padding => {
            let mut lens = Vec::new();
            for part in raw.split_whitespace() {
                lens.push(parse_length(part)?);
            }
            if (1..=4).contains(&lens.len()) {
                Some(Value::Lengths(lens))
            } else {
                None
            }
        }
    }
}

pub(crate) fn parse_color(raw: &str) -> Option<u32> {
    let s = raw.trim();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    Some(match s {
        "black" => 0x0000_0000,
        "white" => 0x00FF_FFFF,
        "red" => 0x00FF_0000,
        "green" => 0x0000_8000,
        "lime" => 0x0000_FF00,
        "blue" => 0x0000_00FF,
        "yellow" => 0x00FF_FF00,
        "navy" => 0x0000_0080,
        "maroon" => 0x0080_0000,
        "purple" => 0x0080_0080,
        "teal" => 0x0000_8080,
        "silver" => 0x00C0_C0C0,
        "gray" | "grey" => 0x0080_8080,
        "orange" => 0x00FF_A500,
        "transparent" => return None,
        _ => return None,
    })
}

fn parse_hex_color(hex: &str) -> Option<u32> {
    let b = hex.as_bytes();
    match b.len() {
        3 => {
            let r = hex_nibble(b[0])?;
            let g = hex_nibble(b[1])?;
            let bl = hex_nibble(b[2])?;
            Some(((r * 0x11) << 16) | ((g * 0x11) << 8) | (bl * 0x11))
        }
        6 => {
            let r = (hex_nibble(b[0])? << 4) | hex_nibble(b[1])?;
            let g = (hex_nibble(b[2])? << 4) | hex_nibble(b[3])?;
            let bl = (hex_nibble(b[4])? << 4) | hex_nibble(b[5])?;
            Some((r << 16) | (g << 8) | bl)
        }
        _ => None,
    }
}

fn hex_nibble(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32),
        _ => None,
    }
}

pub(crate) fn parse_length(raw: &str) -> Option<Length> {
    let s = raw.trim();
    if s == "0" {
        return Some(Length::Px(0));
    }
    if let Some(n) = s.strip_suffix("px") {
        return n.trim().parse::<i32>().ok().map(Length::Px);
    }
    if let Some(n) = s.strip_suffix("em") {
        return n.trim().parse::<i32>().ok().map(Length::Em);
    }
    None
}

impl Length {
    pub(crate) fn to_px(self, em_base: i32) -> i32 {
        match self {
            Self::Px(v) => v,
            Self::Em(v) => v.saturating_mul(em_base),
        }
    }
}
