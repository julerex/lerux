use alloc::vec::Vec;

use crate::css::{Declaration, Property, Value};

/// Computed values for the accepted properties. Lengths are integer CSS pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComputedStyle {
    color: Color,
    background_color: Color,
    font_size: u32,
    display: Display,
    margin: Edges<i32>,
    padding: Edges<u32>,
    width: Width,
}

impl ComputedStyle {
    pub(super) const INITIAL: Self = Self {
        color: Color::BLACK,
        background_color: Color::TRANSPARENT,
        font_size: 16,
        display: Display::Inline,
        margin: Edges::all(0),
        padding: Edges::all(0),
        width: Width::Auto,
    };

    pub(super) fn inherit(&self) -> Self {
        Self {
            color: self.color,
            font_size: self.font_size,
            ..Self::INITIAL
        }
    }

    pub(super) fn apply(&mut self, declaration: &Declaration) {
        let Value::Specified(text) = declaration.value() else {
            return;
        };
        match declaration.property() {
            Property::Color => set_parsed(&mut self.color, Color::parse(text)),
            Property::BackgroundColor => set_parsed(&mut self.background_color, Color::parse(text)),
            Property::FontSize => set_parsed(&mut self.font_size, non_negative_px(text)),
            Property::Display => set_parsed(&mut self.display, Display::parse(text)),
            Property::Margin => set_parsed(&mut self.margin, Edges::parse(text, px)),
            Property::Padding => set_parsed(&mut self.padding, Edges::parse(text, non_negative_px)),
            Property::Width => set_parsed(&mut self.width, Width::parse(text)),
            Property::Custom(_) | Property::Unknown(_) => {}
        }
    }

    #[must_use]
    pub fn color(&self) -> Color {
        self.color
    }

    #[must_use]
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    #[must_use]
    pub fn font_size(&self) -> u32 {
        self.font_size
    }

    #[must_use]
    pub fn display(&self) -> Display {
        self.display
    }

    #[must_use]
    pub fn margin(&self) -> Edges<i32> {
        self.margin
    }

    #[must_use]
    pub fn padding(&self) -> Edges<u32> {
        self.padding
    }

    #[must_use]
    pub fn width(&self) -> Width {
        self.width
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub const BLACK: Self = Self::opaque(0, 0, 0);
    pub const TRANSPARENT: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    };

    #[must_use]
    pub const fn opaque(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 0xff,
        }
    }

    /// `#rgb` or `#rrggbb`.
    fn parse(text: &str) -> Option<Self> {
        match *text.strip_prefix('#')?.as_bytes() {
            [r, g, b] => Some(Self::opaque(
                hex_byte(r, r)?,
                hex_byte(g, g)?,
                hex_byte(b, b)?,
            )),
            [r1, r2, g1, g2, b1, b2] => Some(Self::opaque(
                hex_byte(r1, r2)?,
                hex_byte(g1, g2)?,
                hex_byte(b1, b2)?,
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Display {
    Inline,
    Block,
    None,
}

impl Display {
    fn parse(text: &str) -> Option<Self> {
        [
            ("inline", Self::Inline),
            ("block", Self::Block),
            ("none", Self::None),
        ]
        .into_iter()
        .find_map(|(keyword, display)| text.eq_ignore_ascii_case(keyword).then_some(display))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    const fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    fn parse(text: &str, length: fn(&str) -> Option<T>) -> Option<Self> {
        let lengths: Vec<T> = text
            .split_ascii_whitespace()
            .map(length)
            .collect::<Option<_>>()?;
        match *lengths.as_slice() {
            [all] => Some(Self::all(all)),
            [vertical, horizontal] => Some(Self {
                top: vertical,
                right: horizontal,
                bottom: vertical,
                left: horizontal,
            }),
            [top, horizontal, bottom] => Some(Self {
                top,
                right: horizontal,
                bottom,
                left: horizontal,
            }),
            [top, right, bottom, left] => Some(Self {
                top,
                right,
                bottom,
                left,
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Width {
    Auto,
    Px(u32),
}

impl Width {
    fn parse(text: &str) -> Option<Self> {
        if text.eq_ignore_ascii_case("auto") {
            return Some(Self::Auto);
        }
        non_negative_px(text).map(Self::Px)
    }
}

fn set_parsed<T>(slot: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *slot = value;
    }
}

/// An integer followed by `px`, or a unitless zero.
fn px(text: &str) -> Option<i32> {
    if text.parse::<i32>() == Ok(0) {
        return Some(0);
    }
    let (number, unit) = text.split_at_checked(text.len().checked_sub(2)?)?;
    if !unit.eq_ignore_ascii_case("px") {
        return None;
    }
    number.parse().ok()
}

fn non_negative_px(text: &str) -> Option<u32> {
    u32::try_from(px(text)?).ok()
}

fn hex_byte(high: u8, low: u8) -> Option<u8> {
    Some(hex_digit(high)? * 16 + hex_digit(low)?)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
