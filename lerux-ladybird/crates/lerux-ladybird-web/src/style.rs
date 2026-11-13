//! Cold style for one document.
//!
//! Each element is matched against the user-agent sheet and one author sheet.
//! The matched rules sort by origin, then specificity, then source order, and
//! their declarations apply in that order, so the last one wins. `color` and
//! `font-size` start from the parent's computed value. The other properties
//! start from their initial value.
//!
//! An unresolved value takes no part in the cascade, and neither does a value
//! this subset cannot parse. Custom properties are not computed.

use alloc::vec::Vec;

use lerux_ladybird_html::{Children, Document, Element, Node, Text};

use crate::css::{Declaration, StyleSheet};
use matching::Specificity;

mod computed;
mod matching;

pub use computed::{Color, ComputedStyle, Display, Edges, Width};

const USER_AGENT_SHEET: &str = "
html { display: block; color: #000000; background-color: #ffffff; }
head, title, style { display: none; }
body { display: block; margin: 8px; color: #000000; background-color: #ffffff; }
p { display: block; margin: 16px 0; }
a { display: inline; color: #0000ff; }
iframe { display: block; }
";

/// The nodes of one document in tree order, each element with its computed style.
pub struct StyledDocument<'a> {
    children: Vec<StyledNode<'a>>,
}

impl<'a> StyledDocument<'a> {
    /// Style every element of `document` with the user-agent sheet and `author`.
    #[must_use]
    pub fn new(document: &'a Document, author: &StyleSheet) -> Self {
        let cascade = Cascade::new(author);
        Self {
            children: cascade.style_children(document.children(), &ComputedStyle::INITIAL),
        }
    }

    #[must_use]
    pub fn children(&self) -> &[StyledNode<'a>] {
        &self.children
    }
}

pub enum StyledNode<'a> {
    Element(StyledElement<'a>),
    Text(Text<'a>),
}

/// One element, its computed style, and its child nodes in tree order.
pub struct StyledElement<'a> {
    element: Element<'a>,
    style: ComputedStyle,
    children: Vec<StyledNode<'a>>,
}

impl<'a> StyledElement<'a> {
    #[must_use]
    pub fn element(&self) -> &Element<'a> {
        &self.element
    }

    #[must_use]
    pub fn style(&self) -> &ComputedStyle {
        &self.style
    }

    #[must_use]
    pub fn children(&self) -> &[StyledNode<'a>] {
        &self.children
    }
}

/// Variant order is the cascade order. A later origin wins.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Origin {
    UserAgent,
    Author,
}

/// Field order is the cascade order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Priority {
    origin: Origin,
    specificity: Specificity,
    source_order: usize,
}

struct MatchedRule<'s> {
    priority: Priority,
    declarations: &'s [Declaration],
}

struct Cascade<'s> {
    user_agent: StyleSheet,
    author: &'s StyleSheet,
}

impl<'s> Cascade<'s> {
    fn new(author: &'s StyleSheet) -> Self {
        Self {
            user_agent: StyleSheet::parse(USER_AGENT_SHEET).expect("user-agent sheet parses"),
            author,
        }
    }

    fn style_children<'a>(
        &self,
        children: Children<'a>,
        parent: &ComputedStyle,
    ) -> Vec<StyledNode<'a>> {
        children
            .map(|node| match node {
                Node::Element(element) => StyledNode::Element(self.style_element(element, parent)),
                Node::Text(text) => StyledNode::Text(text),
            })
            .collect()
    }

    fn style_element<'a>(&self, element: Element<'a>, parent: &ComputedStyle) -> StyledElement<'a> {
        let mut style = parent.inherit();
        for rule in self.matched_rules(&element) {
            for declaration in rule.declarations {
                style.apply(declaration);
            }
        }
        let children = self.style_children(element.children(), &style);
        StyledElement {
            element,
            style,
            children,
        }
    }

    fn matched_rules(&self, element: &Element<'_>) -> Vec<MatchedRule<'_>> {
        let sheets = [
            (Origin::UserAgent, &self.user_agent),
            (Origin::Author, self.author),
        ];
        let mut matched = Vec::new();
        for (origin, sheet) in sheets {
            for (source_order, rule) in sheet.rules().iter().enumerate() {
                let Some(specificity) = matching::match_rule(rule, element) else {
                    continue;
                };
                matched.push(MatchedRule {
                    priority: Priority {
                        origin,
                        specificity,
                        source_order,
                    },
                    declarations: rule.declarations(),
                });
            }
        }
        matched.sort_by_key(|rule| rule.priority);
        matched
    }
}
