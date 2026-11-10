//! Cold exact style for one document.
//!
//! A later incremental pass has to match this result, so this pass styles
//! every element from its parent and the full rule list. It has no
//! invalidation graph, no selector buckets, and no `:has()`.
//!
//! Origins are user-agent, then author. Specificity decides inside one
//! origin. Source order breaks a tie. Custom properties stay unresolved
//! text, including when a longhand's winning value still names `var()`.

use alloc::{boxed::Box, vec::Vec};
use lerux_ladybird_html::{Children, Document, Node};

use crate::{
    Combinator, CompoundSelector, Declaration, Property, Selector, SimpleSelector, StyleRule,
    StyleSheet, Value,
};

/// One element the cascade can see.
///
/// `chain` in [`compute_style`] lists these from the document root to the
/// element. The HTML subset stores a tag and a parent, not a class or an id,
/// so [`style_document`] leaves `classes` empty and `id` unset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyleElement<'a> {
    name: &'a str,
    classes: &'a [&'a str],
    id: Option<&'a str>,
}

impl<'a> StyleElement<'a> {
    #[must_use]
    pub const fn new(name: &'a str, classes: &'a [&'a str], id: Option<&'a str>) -> Self {
        Self { name, classes, id }
    }
}

/// One element and the computed style of its element children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyledElement {
    name: Box<str>,
    style: ComputedStyle,
    children: Vec<StyledElement>,
}

impl StyledElement {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn style(&self) -> &ComputedStyle {
        &self.style
    }

    #[must_use]
    pub fn children(&self) -> &[StyledElement] {
        &self.children
    }
}

/// Computed values a layout pass can read, plus unresolved custom properties.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputedStyle {
    color: Color,
    background_color: Color,
    font_size: FontSize,
    display: Display,
    margin: BoxEdges,
    padding: BoxEdges,
    width: Width,
    custom_properties: Vec<CustomProperty>,
}

impl ComputedStyle {
    #[must_use]
    pub const fn color(&self) -> &Color {
        &self.color
    }

    #[must_use]
    pub const fn background_color(&self) -> &Color {
        &self.background_color
    }

    #[must_use]
    pub const fn font_size(&self) -> &FontSize {
        &self.font_size
    }

    #[must_use]
    pub const fn display(&self) -> &Display {
        &self.display
    }

    #[must_use]
    pub const fn margin(&self) -> &BoxEdges {
        &self.margin
    }

    #[must_use]
    pub const fn padding(&self) -> &BoxEdges {
        &self.padding
    }

    #[must_use]
    pub const fn width(&self) -> &Width {
        &self.width
    }

    #[must_use]
    pub fn custom_properties(&self) -> &[CustomProperty] {
        &self.custom_properties
    }

    fn initial() -> Self {
        Self {
            color: Color::Rgba(Rgba::new(0, 0, 0, 0xff)),
            background_color: Color::Transparent,
            font_size: FontSize::Px(INITIAL_FONT_PX),
            display: Display::Inline,
            margin: BoxEdges::Edges(Edges::ZERO),
            padding: BoxEdges::Edges(Edges::ZERO),
            width: Width::Auto,
            custom_properties: Vec::new(),
        }
    }
}

/// An 8-bit color. Hex digits are expanded before they are stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// A computed color. `var()` and any other text that is not a color stay unresolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Color {
    Rgba(Rgba),
    Transparent,
    Unresolved(Box<str>),
}

/// `display` after the cascade. The initial value is `inline`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Display {
    Inline,
    Block,
    None,
    Unresolved(Box<str>),
}

/// A computed font size in integer pixels, or unresolved source text.
///
/// `em` and `%` are resolved against the parent font size. A fractional
/// length stays unresolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FontSize {
    Px(i32),
    Unresolved(Box<str>),
}

/// Margin or padding. A shorthand computes every side, or the whole value stays unresolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoxEdges {
    Edges(Edges),
    Unresolved(Box<str>),
}

/// The four sides of a box. `em` is already an integer pixel count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edges {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl Edges {
    pub const ZERO: Self = Self {
        top: Length::Px(0),
        right: Length::Px(0),
        bottom: Length::Px(0),
        left: Length::Px(0),
    };
}

/// One computed length. Percentages stay percentages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Length {
    Px(i32),
    Percent(i32),
    Auto,
}

/// A computed width. The initial value is `auto`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Width {
    Auto,
    Px(i32),
    Percent(i32),
    Unresolved(Box<str>),
}

/// A custom property that was not substituted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomProperty {
    name: Box<str>,
    value: Box<str>,
}

impl CustomProperty {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

const INITIAL_FONT_PX: i32 = 16;

/// `body` is 8px on every side and `p` is 16px on the vertical sides, so the
/// fixture paragraph's line box starts at y=24. `head`, `title`, and `style`
/// are hidden.
const USER_AGENT: &str = "\
html { display: block; color: #000000; background-color: #ffffff; }
head, title, style { display: none; }
body { display: block; margin: 8px; color: #000000; background-color: #ffffff; }
p { display: block; margin: 16px 0; }
";

const NO_CLASSES: &[&str] = &[];

/// Style `chain`'s last element.
///
/// `chain` runs from the document root to that element. An empty chain is the
/// initial style: black text, a transparent background, a 16px font, inline
/// display, zero margin and padding, and `width: auto`.
#[must_use]
pub fn compute_style(chain: &[StyleElement<'_>], author: &StyleSheet) -> ComputedStyle {
    let ua = user_agent();
    compute_chain(chain, &ua, author)
}

/// Style every element in `document`, parent before child.
///
/// Text nodes are skipped. Class and id selectors do not match these elements.
#[must_use]
pub fn style_document(document: &Document, author: &StyleSheet) -> Vec<StyledElement> {
    let ua = user_agent();
    let mut ancestors = Vec::new();
    style_nodes(
        document.children(),
        &mut ancestors,
        &ComputedStyle::initial(),
        &ua,
        author,
    )
}

fn user_agent() -> StyleSheet {
    StyleSheet::parse(USER_AGENT).expect("user-agent sheet parses")
}

fn compute_chain(
    chain: &[StyleElement<'_>],
    ua: &StyleSheet,
    author: &StyleSheet,
) -> ComputedStyle {
    let Some((element, parents)) = chain.split_last() else {
        return ComputedStyle::initial();
    };
    let parent = compute_chain(parents, ua, author);
    compute_one(element, parents, &parent, ua, author)
}

fn compute_one(
    element: &StyleElement<'_>,
    ancestors: &[StyleElement<'_>],
    parent: &ComputedStyle,
    ua: &StyleSheet,
    author: &StyleSheet,
) -> ComputedStyle {
    apply(parent, &winners(element, ancestors, ua, author))
}

fn style_nodes<'doc>(
    nodes: Children<'doc>,
    ancestors: &mut Vec<StyleElement<'doc>>,
    parent: &ComputedStyle,
    ua: &StyleSheet,
    author: &StyleSheet,
) -> Vec<StyledElement> {
    let mut styled = Vec::new();
    for node in nodes {
        let Node::Element(element) = node else {
            continue;
        };
        let current = StyleElement::new(element.name(), NO_CLASSES, None);
        let style = compute_one(&current, ancestors, parent, ua, author);
        ancestors.push(current);
        let children = style_nodes(element.children(), ancestors, &style, ua, author);
        ancestors.pop();
        styled.push(StyledElement {
            name: Box::from(element.name()),
            style,
            children,
        });
    }
    styled
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum Origin {
    UserAgent,
    Author,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct Rank {
    origin: Origin,
    specificity: (u32, u32, u32),
    order: u32,
}

struct Slot<'a> {
    rank: Rank,
    text: &'a str,
}

struct Found<'a> {
    color: Option<Slot<'a>>,
    background_color: Option<Slot<'a>>,
    font_size: Option<Slot<'a>>,
    display: Option<Slot<'a>>,
    margin: Option<Slot<'a>>,
    padding: Option<Slot<'a>>,
    width: Option<Slot<'a>>,
    custom: Vec<CustomHit<'a>>,
}

struct CustomHit<'a> {
    name: &'a str,
    rank: Rank,
    value: &'a str,
}

struct Winners<'a> {
    color: Option<&'a str>,
    background_color: Option<&'a str>,
    font_size: Option<&'a str>,
    display: Option<&'a str>,
    margin: Option<&'a str>,
    padding: Option<&'a str>,
    width: Option<&'a str>,
    custom: Vec<(&'a str, &'a str)>,
}

fn winners<'a>(
    element: &StyleElement<'_>,
    ancestors: &[StyleElement<'_>],
    ua: &'a StyleSheet,
    author: &'a StyleSheet,
) -> Winners<'a> {
    let mut found = Found::empty();
    collect_sheet(&mut found, Origin::UserAgent, ua, element, ancestors);
    collect_sheet(&mut found, Origin::Author, author, element, ancestors);
    found.finish()
}

impl<'a> Found<'a> {
    fn empty() -> Self {
        Self {
            color: None,
            background_color: None,
            font_size: None,
            display: None,
            margin: None,
            padding: None,
            width: None,
            custom: Vec::new(),
        }
    }

    fn finish(self) -> Winners<'a> {
        Winners {
            color: self.color.map(|slot| slot.text),
            background_color: self.background_color.map(|slot| slot.text),
            font_size: self.font_size.map(|slot| slot.text),
            display: self.display.map(|slot| slot.text),
            margin: self.margin.map(|slot| slot.text),
            padding: self.padding.map(|slot| slot.text),
            width: self.width.map(|slot| slot.text),
            custom: self
                .custom
                .into_iter()
                .map(|hit| (hit.name, hit.value))
                .collect(),
        }
    }
}

fn collect_sheet<'a>(
    found: &mut Found<'a>,
    origin: Origin,
    sheet: &'a StyleSheet,
    element: &StyleElement<'_>,
    ancestors: &[StyleElement<'_>],
) {
    let mut order = 0u32;
    for rule in sheet.rules() {
        let specificity = matching_specificity(rule, element, ancestors);
        for declaration in rule.declarations() {
            if let Some(specificity) = specificity {
                note(
                    found,
                    Rank {
                        origin,
                        specificity,
                        order,
                    },
                    declaration,
                );
            }
            order = order.saturating_add(1);
        }
    }
}

fn note<'a>(found: &mut Found<'a>, rank: Rank, declaration: &'a Declaration) {
    let text = value_text(declaration.value());
    match declaration.property() {
        Property::Color => offer(&mut found.color, rank, text),
        Property::BackgroundColor => offer(&mut found.background_color, rank, text),
        Property::FontSize => offer(&mut found.font_size, rank, text),
        Property::Display => offer(&mut found.display, rank, text),
        Property::Margin => offer(&mut found.margin, rank, text),
        Property::Padding => offer(&mut found.padding, rank, text),
        Property::Width => offer(&mut found.width, rank, text),
        Property::Custom(name) => offer_custom(&mut found.custom, name, rank, text),
        Property::Unknown(_) => {}
    }
}

fn offer<'a>(slot: &mut Option<Slot<'a>>, rank: Rank, text: &'a str) {
    if slot.as_ref().is_none_or(|current| rank > current.rank) {
        *slot = Some(Slot { rank, text });
    }
}

fn offer_custom<'a>(custom: &mut Vec<CustomHit<'a>>, name: &'a str, rank: Rank, value: &'a str) {
    if let Some(hit) = custom.iter_mut().find(|hit| hit.name == name) {
        if rank > hit.rank {
            hit.rank = rank;
            hit.value = value;
        }
        return;
    }
    custom.push(CustomHit { name, rank, value });
}

fn value_text(value: &Value) -> &str {
    match value {
        Value::Specified(text) | Value::Unresolved(text) => text,
    }
}

fn matching_specificity(
    rule: &StyleRule,
    element: &StyleElement<'_>,
    ancestors: &[StyleElement<'_>],
) -> Option<(u32, u32, u32)> {
    let mut best = None;
    for selector in rule.selectors() {
        if selector_matches(selector, element, ancestors) {
            let spec = specificity(selector);
            best = Some(best.map_or(spec, |current: (u32, u32, u32)| current.max(spec)));
        }
    }
    best
}

fn specificity(selector: &Selector) -> (u32, u32, u32) {
    let mut ids = 0u32;
    let mut classes = 0u32;
    let mut types = 0u32;
    for compound in selector.compounds() {
        for simple in compound.simples() {
            match simple {
                SimpleSelector::Id(_) => ids += 1,
                SimpleSelector::Class(_) => classes += 1,
                SimpleSelector::Type(_) => types += 1,
            }
        }
    }
    (ids, classes, types)
}

fn selector_matches(
    selector: &Selector,
    element: &StyleElement<'_>,
    ancestors: &[StyleElement<'_>],
) -> bool {
    let compounds = selector.compounds();
    let Some(subject) = compounds.last() else {
        return false;
    };
    if !compound_matches(subject, element) {
        return false;
    }
    let mut right = compounds.len() - 1;
    let mut cursor = ancestors.len();
    while right > 0 {
        let combinator = compounds[right].combinator();
        right -= 1;
        if !step(combinator, &compounds[right], ancestors, &mut cursor) {
            return false;
        }
    }
    true
}

fn step(
    combinator: Combinator,
    compound: &CompoundSelector,
    ancestors: &[StyleElement<'_>],
    cursor: &mut usize,
) -> bool {
    match combinator {
        Combinator::None => false,
        Combinator::Child => child_matches(compound, ancestors, cursor),
        Combinator::Descendant => descendant_matches(compound, ancestors, cursor),
    }
}

fn child_matches(
    compound: &CompoundSelector,
    ancestors: &[StyleElement<'_>],
    cursor: &mut usize,
) -> bool {
    if *cursor == 0 {
        return false;
    }
    *cursor -= 1;
    compound_matches(compound, &ancestors[*cursor])
}

fn descendant_matches(
    compound: &CompoundSelector,
    ancestors: &[StyleElement<'_>],
    cursor: &mut usize,
) -> bool {
    while *cursor > 0 {
        *cursor -= 1;
        if compound_matches(compound, &ancestors[*cursor]) {
            return true;
        }
    }
    false
}

fn compound_matches(compound: &CompoundSelector, element: &StyleElement<'_>) -> bool {
    compound
        .simples()
        .iter()
        .all(|simple| simple_matches(simple, element))
}

fn simple_matches(simple: &SimpleSelector, element: &StyleElement<'_>) -> bool {
    match simple {
        SimpleSelector::Type(name) => element.name.eq_ignore_ascii_case(name),
        SimpleSelector::Class(name) => element.classes.iter().any(|class| *class == name.as_ref()),
        SimpleSelector::Id(name) => element.id == Some(name.as_ref()),
    }
}

fn apply(parent: &ComputedStyle, won: &Winners<'_>) -> ComputedStyle {
    let font_size = font_size(parent, won.font_size);
    let em_px = absolute_px(&font_size);
    ComputedStyle {
        color: color(parent, won.color),
        background_color: background(won.background_color),
        font_size,
        display: display(won.display),
        margin: box_edges(won.margin, em_px, AutoPolicy::Allow),
        padding: box_edges(won.padding, em_px, AutoPolicy::Reject),
        width: width(won.width, em_px),
        custom_properties: customs(&parent.custom_properties, &won.custom),
    }
}

fn color(parent: &ComputedStyle, value: Option<&str>) -> Color {
    match value {
        Some(text) => color_from(text),
        None => parent.color.clone(),
    }
}

fn background(value: Option<&str>) -> Color {
    match value {
        Some(text) => color_from(text),
        None => Color::Transparent,
    }
}

fn font_size(parent: &ComputedStyle, value: Option<&str>) -> FontSize {
    match value {
        Some(text) => font_size_from(text, absolute_px(&parent.font_size)),
        None => parent.font_size.clone(),
    }
}

fn display(value: Option<&str>) -> Display {
    match value {
        Some(text) => display_from(text),
        None => Display::Inline,
    }
}

fn absolute_px(font: &FontSize) -> Option<i32> {
    match font {
        FontSize::Px(px) => Some(*px),
        FontSize::Unresolved(_) => None,
    }
}

fn customs(parent: &[CustomProperty], winners: &[(&str, &str)]) -> Vec<CustomProperty> {
    let mut custom = parent.to_vec();
    for (name, value) in winners {
        let value = Box::from(*value);
        if let Some(existing) = custom.iter_mut().find(|item| item.name.as_ref() == *name) {
            existing.value = value;
        } else {
            custom.push(CustomProperty {
                name: Box::from(*name),
                value,
            });
        }
    }
    custom
}

#[derive(Clone, Copy)]
enum AutoPolicy {
    Allow,
    Reject,
}

fn color_from(value: &str) -> Color {
    if value.eq_ignore_ascii_case("transparent") {
        return Color::Transparent;
    }
    if let Some(rgba) = parse_hex(value) {
        return Color::Rgba(rgba);
    }
    Color::Unresolved(Box::from(value))
}

fn parse_hex(value: &str) -> Option<Rgba> {
    let digits = value.strip_prefix('#')?.as_bytes();
    match digits.len() {
        3 | 4 => parse_short_hex(digits),
        6 | 8 => parse_long_hex(digits),
        _ => None,
    }
}

fn parse_short_hex(digits: &[u8]) -> Option<Rgba> {
    let red = expand_nibble(hex_nibble(digits.first().copied()?)?);
    let green = expand_nibble(hex_nibble(digits.get(1).copied()?)?);
    let blue = expand_nibble(hex_nibble(digits.get(2).copied()?)?);
    let alpha = if digits.len() == 4 {
        expand_nibble(hex_nibble(digits.get(3).copied()?)?)
    } else {
        0xff
    };
    Some(Rgba::new(red, green, blue, alpha))
}

fn parse_long_hex(digits: &[u8]) -> Option<Rgba> {
    let red = hex_byte(digits, 0)?;
    let green = hex_byte(digits, 2)?;
    let blue = hex_byte(digits, 4)?;
    let alpha = if digits.len() == 8 {
        hex_byte(digits, 6)?
    } else {
        0xff
    };
    Some(Rgba::new(red, green, blue, alpha))
}

fn hex_byte(digits: &[u8], index: usize) -> Option<u8> {
    let high = hex_nibble(*digits.get(index)?)?;
    let low = hex_nibble(*digits.get(index + 1)?)?;
    Some((high << 4) | low)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn expand_nibble(nibble: u8) -> u8 {
    nibble * 17
}

fn display_from(value: &str) -> Display {
    if value.eq_ignore_ascii_case("none") {
        Display::None
    } else if value.eq_ignore_ascii_case("block") {
        Display::Block
    } else if value.eq_ignore_ascii_case("inline") {
        Display::Inline
    } else {
        Display::Unresolved(Box::from(value))
    }
}

fn font_size_from(value: &str, parent_px: Option<i32>) -> FontSize {
    let Some(token) = single_token(value) else {
        return FontSize::Unresolved(Box::from(value));
    };
    match length_token(token, parent_px, AutoPolicy::Reject) {
        Some(Length::Px(px)) if px >= 0 => FontSize::Px(px),
        Some(Length::Percent(percent)) if percent >= 0 => font_percent(value, parent_px, percent),
        Some(Length::Px(_) | Length::Percent(_) | Length::Auto) | None => {
            FontSize::Unresolved(Box::from(value))
        }
    }
}

fn font_percent(value: &str, parent_px: Option<i32>, percent: i32) -> FontSize {
    match percent_of(parent_px, percent) {
        Some(px) => FontSize::Px(px),
        None => FontSize::Unresolved(Box::from(value)),
    }
}

fn percent_of(base: Option<i32>, percent: i32) -> Option<i32> {
    base?.checked_mul(percent)?.checked_div(100)
}

fn box_edges(value: Option<&str>, em_px: Option<i32>, auto: AutoPolicy) -> BoxEdges {
    match value {
        Some(text) => resolve_edges(text, em_px, auto),
        None => BoxEdges::Edges(Edges::ZERO),
    }
}

fn resolve_edges(value: &str, em_px: Option<i32>, auto: AutoPolicy) -> BoxEdges {
    let Some(edges) = edges_from(value, em_px, auto) else {
        return BoxEdges::Unresolved(Box::from(value));
    };
    BoxEdges::Edges(edges)
}

fn edges_from(value: &str, em_px: Option<i32>, auto: AutoPolicy) -> Option<Edges> {
    let parsed = tokens(value)?;
    let sides = side_lengths(&parsed, em_px, auto)?;
    Some(Edges {
        top: sides[0],
        right: sides[1],
        bottom: sides[2],
        left: sides[3],
    })
}

fn side_lengths(parsed: &Tokens<'_>, em_px: Option<i32>, auto: AutoPolicy) -> Option<[Length; 4]> {
    let at = |index| length_token(parsed.parts[index], em_px, auto);
    match parsed.len {
        1 => {
            let all = at(0)?;
            Some([all, all, all, all])
        }
        2 => {
            let vertical = at(0)?;
            let horizontal = at(1)?;
            Some([vertical, horizontal, vertical, horizontal])
        }
        3 => {
            let top = at(0)?;
            let horizontal = at(1)?;
            let bottom = at(2)?;
            Some([top, horizontal, bottom, horizontal])
        }
        4 => Some([at(0)?, at(1)?, at(2)?, at(3)?]),
        _ => None,
    }
}

fn width(value: Option<&str>, em_px: Option<i32>) -> Width {
    match value {
        Some(text) => width_from(text, em_px),
        None => Width::Auto,
    }
}

fn width_from(value: &str, em_px: Option<i32>) -> Width {
    let Some(token) = single_token(value) else {
        return Width::Unresolved(Box::from(value));
    };
    match length_token(token, em_px, AutoPolicy::Allow) {
        Some(Length::Auto) => Width::Auto,
        Some(Length::Px(px)) => Width::Px(px),
        Some(Length::Percent(percent)) => Width::Percent(percent),
        None => Width::Unresolved(Box::from(value)),
    }
}

fn single_token(value: &str) -> Option<&str> {
    let parsed = tokens(value)?;
    if parsed.len == 1 {
        Some(parsed.parts[0])
    } else {
        None
    }
}

struct Tokens<'a> {
    parts: [&'a str; 4],
    len: usize,
}

fn tokens(value: &str) -> Option<Tokens<'_>> {
    let mut parts = [""; 4];
    let mut len = 0usize;
    for part in value.split(is_css_whitespace) {
        if part.is_empty() {
            continue;
        }
        if len == 4 {
            return None;
        }
        parts[len] = part;
        len += 1;
    }
    if len == 0 {
        None
    } else {
        Some(Tokens { parts, len })
    }
}

fn length_token(token: &str, em_px: Option<i32>, auto: AutoPolicy) -> Option<Length> {
    if token.eq_ignore_ascii_case("auto") {
        return match auto {
            AutoPolicy::Allow => Some(Length::Auto),
            AutoPolicy::Reject => None,
        };
    }
    let (number, unit) = split_unit(token);
    let number = parse_i32(number)?;
    match unit {
        "" => (number == 0).then_some(Length::Px(0)),
        "%" => Some(Length::Percent(number)),
        _ if unit.eq_ignore_ascii_case("px") => Some(Length::Px(number)),
        _ if unit.eq_ignore_ascii_case("em") => Some(Length::Px(number.checked_mul(em_px?)?)),
        _ => None,
    }
}

fn split_unit(token: &str) -> (&str, &str) {
    if let Some(number) = token.strip_suffix('%') {
        return (number, "%");
    }
    let bytes = token.as_bytes();
    let mut index = bytes.len();
    while index > 0 && bytes[index - 1].is_ascii_alphabetic() {
        index -= 1;
    }
    (&token[..index], &token[index..])
}

fn parse_i32(text: &str) -> Option<i32> {
    let (negative, digits) = match text.as_bytes().first().copied() {
        Some(b'+') => (false, &text[1..]),
        Some(b'-') => (true, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let magnitude = digits.parse::<i32>().ok()?;
    if negative {
        magnitude.checked_neg()
    } else {
        Some(magnitude)
    }
}

fn is_css_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}
