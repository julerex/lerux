use alloc::{boxed::Box, string::String, vec::Vec};
use core::str::SplitAsciiWhitespace;

mod parse;
mod tokenizer;

struct Known {
    name: &'static str,
    mode: InsertionMode,
}

#[derive(Clone, Copy)]
enum InsertionMode {
    Doctype,
    Html,
    Head,
    Title,
    Body,
    Paragraph,
}

const KNOWN: &[Known] = &[
    Known {
        name: "html",
        mode: InsertionMode::Html,
    },
    Known {
        name: "head",
        mode: InsertionMode::Head,
    },
    Known {
        name: "title",
        mode: InsertionMode::Title,
    },
    Known {
        name: "body",
        mode: InsertionMode::Body,
    },
    Known {
        name: "p",
        mode: InsertionMode::Paragraph,
    },
];

#[derive(Clone, Copy)]
struct NodeId(usize);

#[derive(Clone, Copy)]
enum ParentLink {
    Document,
    Node(NodeId),
}

enum NodeData {
    Element {
        tag: Tag,
        id: Option<Box<str>>,
        class: Option<Box<str>>,
        href: Option<Box<str>>,
        src: Option<Box<str>>,
        children: Vec<NodeId>,
    },
    Text {
        data: String,
    },
}

struct NodeRecord {
    parent: ParentLink,
    data: NodeData,
}

enum Tag {
    Known(usize),
    Other(Box<str>),
}

impl Tag {
    fn from_name(name: &str) -> Option<Self> {
        if name.is_empty() {
            return None;
        }
        if let Some(index) = KNOWN
            .iter()
            .position(|known| known.name.eq_ignore_ascii_case(name))
        {
            return Some(Self::Known(index));
        }
        let mut lowered = String::from(name);
        lowered.make_ascii_lowercase();
        Some(Self::Other(lowered.into_boxed_str()))
    }

    fn name(&self) -> &str {
        match self {
            Self::Known(index) => KNOWN[*index].name,
            Self::Other(name) => name,
        }
    }

    fn known(&self) -> Option<&'static Known> {
        match self {
            Self::Known(index) => Some(&KNOWN[*index]),
            Self::Other(_) => None,
        }
    }
}

pub struct Document {
    nodes: Vec<NodeRecord>,
    children: Vec<NodeId>,
}

impl Document {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn parse(html: &str) -> Self {
        parse::build(html)
    }

    /// Text of the first `title` element in tree order.
    ///
    /// A missing title, or a title with no text child, returns `""`.
    #[must_use]
    pub fn title_text(&self) -> &str {
        let mut index = 0;
        while index < self.children.len() {
            let id = self.children[index];
            if let Some(text) = self.title_in(id) {
                return text;
            }
            index += 1;
        }
        ""
    }

    #[must_use]
    pub fn children(&self) -> Children<'_> {
        Children {
            document: self,
            ids: &self.children,
            index: 0,
        }
    }

    fn title_in(&self, id: NodeId) -> Option<&str> {
        if !self.is_element(id) {
            return None;
        }
        if self.is_title(id) {
            let mut index = 0;
            while let Some(child) = self.child_at(id, index) {
                if let Some(text) = self.text_of(child) {
                    return Some(text);
                }
                index += 1;
            }
            return Some("");
        }
        let mut index = 0;
        while let Some(child) = self.child_at(id, index) {
            if let Some(text) = self.title_in(child) {
                return Some(text);
            }
            index += 1;
        }
        None
    }

    fn is_element(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].data, NodeData::Element { .. })
    }

    fn is_title(&self, id: NodeId) -> bool {
        self.tag(id).name() == "title"
    }

    fn child_at(&self, id: NodeId, index: usize) -> Option<NodeId> {
        self.child_ids(id).get(index).copied()
    }

    fn text_of(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id.0].data {
            NodeData::Text { data } => Some(data),
            NodeData::Element { .. } => None,
        }
    }

    fn tag(&self, id: NodeId) -> &Tag {
        let NodeData::Element { tag, .. } = &self.nodes[id.0].data else {
            unreachable!("element id");
        };
        tag
    }

    fn child_ids(&self, id: NodeId) -> &[NodeId] {
        let NodeData::Element { children, .. } = &self.nodes[id.0].data else {
            unreachable!("element id");
        };
        children
    }

    fn id_attribute(&self, node: NodeId) -> Option<&str> {
        let NodeData::Element { id, .. } = &self.nodes[node.0].data else {
            unreachable!("element id");
        };
        id.as_deref()
    }

    fn class_attribute(&self, node: NodeId) -> Option<&str> {
        let NodeData::Element { class, .. } = &self.nodes[node.0].data else {
            unreachable!("element id");
        };
        class.as_deref()
    }

    fn href_attribute(&self, node: NodeId) -> Option<&str> {
        let NodeData::Element { href, .. } = &self.nodes[node.0].data else {
            unreachable!("element id");
        };
        href.as_deref()
    }

    fn src_attribute(&self, node: NodeId) -> Option<&str> {
        let NodeData::Element { src, .. } = &self.nodes[node.0].data else {
            unreachable!("element id");
        };
        src.as_deref()
    }

    fn text_data(&self, id: NodeId) -> &str {
        let NodeData::Text { data } = &self.nodes[id.0].data else {
            unreachable!("text id");
        };
        data
    }

    fn parent_of(&self, id: NodeId) -> Parent<'_> {
        match self.nodes[id.0].parent {
            ParentLink::Document => Parent::Document,
            ParentLink::Node(parent) => Parent::Element(Element {
                document: self,
                node: parent,
            }),
        }
    }

    fn node(&self, node: NodeId) -> Node<'_> {
        match &self.nodes[node.0].data {
            NodeData::Element { .. } => Node::Element(Element {
                document: self,
                node,
            }),
            NodeData::Text { .. } => Node::Text(Text {
                document: self,
                node,
            }),
        }
    }
}

pub struct Element<'a> {
    document: &'a Document,
    node: NodeId,
}

impl<'a> Element<'a> {
    /// Lowercase tag name.
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.document.tag(self.node).name()
    }

    /// The `id` attribute value as the start tag wrote it.
    #[must_use]
    pub fn id(&self) -> Option<&'a str> {
        self.document.id_attribute(self.node)
    }

    /// The `class` attribute split on ASCII whitespace. Each token keeps its case.
    #[must_use]
    pub fn classes(&self) -> SplitAsciiWhitespace<'a> {
        self.document
            .class_attribute(self.node)
            .unwrap_or_default()
            .split_ascii_whitespace()
    }

    /// The `href` attribute as the start tag wrote it.
    #[must_use]
    pub fn href(&self) -> Option<&'a str> {
        self.document.href_attribute(self.node)
    }

    /// The `src` attribute as the start tag wrote it.
    #[must_use]
    pub fn src(&self) -> Option<&'a str> {
        self.document.src_attribute(self.node)
    }

    #[must_use]
    pub fn parent(&self) -> Parent<'a> {
        self.document.parent_of(self.node)
    }

    #[must_use]
    pub fn children(&self) -> Children<'a> {
        Children {
            document: self.document,
            ids: self.document.child_ids(self.node),
            index: 0,
        }
    }
}

pub struct Text<'a> {
    document: &'a Document,
    node: NodeId,
}

impl<'a> Text<'a> {
    #[must_use]
    pub fn data(&self) -> &'a str {
        self.document.text_data(self.node)
    }

    #[must_use]
    pub fn parent(&self) -> Parent<'a> {
        self.document.parent_of(self.node)
    }
}

pub enum Node<'a> {
    Element(Element<'a>),
    Text(Text<'a>),
}

pub enum Parent<'a> {
    Document,
    Element(Element<'a>),
}

pub struct Children<'a> {
    document: &'a Document,
    ids: &'a [NodeId],
    index: usize,
}

impl<'a> Iterator for Children<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = *self.ids.get(self.index)?;
        self.index += 1;
        Some(self.document.node(id))
    }
}
