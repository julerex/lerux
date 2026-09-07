//! Subset HTML tokenizer and tree builder for lerux `web-content` (Phase 74).
//!
//! `#![no_std]` + `alloc`. Not html5ever, not Ladybird's tokenizer, not spec-complete.
//! Supported elements: `html`, `head`, `body`, `title`, `p`, `h1`–`h3`, `a`, `div`,
//! `span`, `ul`/`ol`/`li`, `pre`, `code`, `strong`, `em`, `img` (void), `style` (raw text).

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod token;

use alloc::{string::String, vec::Vec};

use token::{tokenize, Token};

/// Committed smoke fixture (`support/browser/fixture.html`).
pub const SMOKE_FIXTURE: &str = include_str!("../../../../support/browser/fixture.html");

/// Node index into [`Document::nodes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(pub usize);

/// Subset tag names (ASCII, case-insensitive at parse).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagName {
    Html,
    Head,
    Body,
    Title,
    P,
    H1,
    H2,
    H3,
    A,
    Div,
    Span,
    Ul,
    Ol,
    Li,
    Pre,
    Code,
    Strong,
    Em,
    Img,
    Style,
}

impl TagName {
    /// ASCII tag token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Head => "head",
            Self::Body => "body",
            Self::Title => "title",
            Self::P => "p",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::A => "a",
            Self::Div => "div",
            Self::Span => "span",
            Self::Ul => "ul",
            Self::Ol => "ol",
            Self::Li => "li",
            Self::Pre => "pre",
            Self::Code => "code",
            Self::Strong => "strong",
            Self::Em => "em",
            Self::Img => "img",
            Self::Style => "style",
        }
    }

    /// ASCII tag token, case-insensitive.
    pub fn from_ascii(name: &[u8]) -> Option<Self> {
        let mut lower = [0u8; 16];
        if name.is_empty() || name.len() > lower.len() {
            return None;
        }
        for (i, b) in name.iter().enumerate() {
            lower[i] = b.to_ascii_lowercase();
        }
        match &lower[..name.len()] {
            b"html" => Some(Self::Html),
            b"head" => Some(Self::Head),
            b"body" => Some(Self::Body),
            b"title" => Some(Self::Title),
            b"p" => Some(Self::P),
            b"h1" => Some(Self::H1),
            b"h2" => Some(Self::H2),
            b"h3" => Some(Self::H3),
            b"a" => Some(Self::A),
            b"div" => Some(Self::Div),
            b"span" => Some(Self::Span),
            b"ul" => Some(Self::Ul),
            b"ol" => Some(Self::Ol),
            b"li" => Some(Self::Li),
            b"pre" => Some(Self::Pre),
            b"code" => Some(Self::Code),
            b"strong" => Some(Self::Strong),
            b"em" => Some(Self::Em),
            b"img" => Some(Self::Img),
            b"style" => Some(Self::Style),
            _ => None,
        }
    }

    /// `img` has no end tag / children.
    pub const fn is_void(self) -> bool {
        matches!(self, Self::Img)
    }

    /// Tokenizer treats contents as text (no nested tags).
    pub const fn is_raw_text(self) -> bool {
        matches!(self, Self::Style)
    }

    pub(crate) fn preserves_whitespace(self) -> bool {
        matches!(self, Self::Pre | Self::Style | Self::Code | Self::Title)
    }

    pub(crate) fn end_tag_bytes(self) -> &'static [u8] {
        match self {
            Self::Style => b"</style",
            _ => b"</",
        }
    }
}

/// Element attribute (`name` is lowercase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub name: String,
    pub value: String,
}

/// Kind of a tree node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Element { name: TagName, attrs: Vec<Attr> },
    Text(String),
}

/// One DOM node. Children are indices, not pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

impl Node {
    /// Attribute value if this is an element and `name` is present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        let NodeKind::Element { attrs, .. } = &self.kind else {
            return None;
        };
        attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    pub fn tag(&self) -> Option<TagName> {
        match self.kind {
            NodeKind::Element { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// Walkable document produced by [`parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    nodes: Vec<Node>,
}

impl Document {
    fn new() -> Self {
        Self {
            nodes: alloc::vec![Node {
                kind: NodeKind::Document,
                parent: None,
                children: Vec::new(),
            }],
        }
    }

    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Total nodes including the document node.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    /// Concatenated descendant text (tree order).
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.collect_text(id, &mut out);
        out
    }

    fn collect_text(&self, id: NodeId, out: &mut String) {
        match &self.get(id).kind {
            NodeKind::Text(s) => out.push_str(s),
            _ => {
                for child in &self.get(id).children {
                    self.collect_text(*child, out);
                }
            }
        }
    }

    /// First element with this tag name, tree order.
    pub fn first_element(&self, name: TagName) -> Option<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .find_map(|(i, n)| match n.kind {
                NodeKind::Element { name: n, .. } if n == name => Some(NodeId(i)),
                _ => None,
            })
    }

    fn append(&mut self, parent: NodeId, kind: NodeKind) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            kind,
            parent: Some(parent),
            children: Vec::new(),
        });
        self.nodes[parent.0].children.push(id);
        id
    }
}

/// Parse `input` into a walkable tree. Never fails: unknown tags are skipped.
pub fn parse(input: &str) -> Document {
    let tokens = tokenize(input);
    let mut doc = Document::new();
    let mut open = alloc::vec![doc.root()];
    for token in tokens {
        match token {
            Token::Start {
                name,
                attrs,
                self_closing,
            } => {
                let parent = *open.last().unwrap_or(&doc.root());
                let id = doc.append(parent, NodeKind::Element { name, attrs });
                if !self_closing && !name.is_void() {
                    open.push(id);
                }
            }
            Token::End { name } => {
                if let Some(i) = open.iter().rposition(|id| doc.get(*id).tag() == Some(name)) {
                    open.truncate(i);
                    if open.is_empty() {
                        open.push(doc.root());
                    }
                }
            }
            Token::Text(text) => {
                let parent = *open.last().unwrap_or(&doc.root());
                if keep_text(&doc, parent, &text) {
                    if let Some(last) = doc.get(parent).children.last().copied()
                        && let NodeKind::Text(existing) = &mut doc.nodes[last.0].kind
                    {
                        existing.push_str(&text);
                    } else {
                        doc.append(parent, NodeKind::Text(text));
                    }
                }
            }
        }
    }
    doc
}

fn keep_text(doc: &Document, parent: NodeId, text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if let Some(tag) = doc.get(parent).tag()
        && tag.preserves_whitespace()
    {
        return true;
    }
    !text
        .as_bytes()
        .iter()
        .all(|&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use token::tokenize;

    fn browser_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../support/browser")
    }

    fn read_fixture(name: &str) -> String {
        std::fs::read_to_string(browser_dir().join(name)).expect("fixture")
    }

    #[test]
    fn smoke_fixture_on_disk_matches_include_str() {
        assert_eq!(read_fixture("fixture.html"), SMOKE_FIXTURE);
    }

    #[test]
    fn parse_should_count_eight_nodes_on_smoke_fixture() {
        let doc = parse(SMOKE_FIXTURE);
        assert_eq!(doc.node_count(), 8);
    }

    #[test]
    fn parse_should_expose_title_and_paragraph_text() {
        let doc = parse(SMOKE_FIXTURE);
        let title = doc.first_element(TagName::Title).expect("title");
        assert_eq!(doc.text_content(title), "lerux");
        let p = doc.first_element(TagName::P).expect("p");
        assert_eq!(doc.text_content(p), "lerux-http-fixture");
    }

    #[test]
    fn parse_should_walk_subset_fixture_tags() {
        let src = read_fixture("subset.html");
        let doc = parse(&src);
        for name in [
            TagName::Html,
            TagName::Head,
            TagName::Body,
            TagName::Title,
            TagName::Style,
            TagName::H1,
            TagName::H2,
            TagName::H3,
            TagName::Div,
            TagName::Span,
            TagName::P,
            TagName::Strong,
            TagName::Em,
            TagName::A,
            TagName::Ul,
            TagName::Ol,
            TagName::Li,
            TagName::Pre,
            TagName::Code,
            TagName::Img,
        ] {
            assert!(
                doc.first_element(name).is_some(),
                "missing <{}>",
                name.as_str()
            );
        }
        let style = doc.first_element(TagName::Style).unwrap();
        assert!(doc.text_content(style).contains("color: red"));
        let a = doc.first_element(TagName::A).unwrap();
        assert_eq!(doc.get(a).attr("href"), Some("https://host/"));
        let img = doc.first_element(TagName::Img).unwrap();
        assert_eq!(doc.get(img).attr("src"), Some("x.png"));
        assert_eq!(doc.get(img).attr("alt"), Some("pic"));
        assert!(doc.get(img).children.is_empty());
        let pre = doc.first_element(TagName::Pre).unwrap();
        assert!(doc.text_content(pre).contains("keep\nspace"));
    }

    #[test]
    fn tokenize_should_skip_doctype_and_comments() {
        let tokens = tokenize("<!DOCTYPE html><!-- hi --><p>x</p>");
        assert!(matches!(
            &tokens[0],
            Token::Start {
                name: TagName::P,
                ..
            }
        ));
    }

    #[test]
    fn parse_should_ignore_unknown_tags_but_keep_inner_text() {
        let doc = parse("<p><unknown>z</unknown></p>");
        let p = doc.first_element(TagName::P).unwrap();
        assert_eq!(doc.text_content(p), "z");
        assert_eq!(
            doc.nodes
                .iter()
                .filter(|n| matches!(n.kind, NodeKind::Element { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn parse_should_not_panic_on_unmatched_end_tag() {
        let doc = parse("</p><div>ok</div>");
        let div = doc.first_element(TagName::Div).unwrap();
        assert_eq!(doc.text_content(div), "ok");
    }

    #[test]
    fn parse_should_treat_img_as_void() {
        let doc = parse("<p><img src=a>after</p>");
        let p = doc.first_element(TagName::P).unwrap();
        assert_eq!(doc.text_content(p), "after");
        assert_eq!(doc.get(p).children.len(), 2);
    }
}
