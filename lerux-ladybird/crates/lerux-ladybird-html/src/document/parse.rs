use alloc::{borrow::ToOwned, vec::Vec};

use super::{
    tokenizer::{DataToken, Tokenizer},
    Document, InsertionMode, NodeData, NodeId, ParentLink, Tag,
};

pub(super) fn build(html: &str) -> Document {
    TreeBuilder {
        tokenizer: Tokenizer::new(html),
        document: Document::empty(),
        open: Vec::new(),
        after_doctype: false,
    }
    .finish()
}

struct TreeBuilder<'a> {
    tokenizer: Tokenizer<'a>,
    document: Document,
    open: Vec<NodeId>,
    after_doctype: bool,
}

impl<'a> TreeBuilder<'a> {
    fn mode(&self) -> InsertionMode {
        if let Some(known) = self.nearest_known() {
            return known.mode;
        }
        if self.after_doctype {
            InsertionMode::Html
        } else {
            InsertionMode::Doctype
        }
    }

    fn nearest_known(&self) -> Option<&'static super::Known> {
        self.open.iter().rev().find_map(|&id| self.tag(id).known())
    }

    fn finish(mut self) -> Document {
        loop {
            match self.mode() {
                InsertionMode::Title => {
                    let end_tag = self.nearest_known().expect("title is open").name;
                    let raw = self.tokenizer.take_raw_text(end_tag);
                    let text = raw.text;
                    let closed = raw.closed;
                    self.append_text(text);
                    if !closed {
                        break;
                    }
                    self.open.pop();
                }
                InsertionMode::Doctype => {
                    let Some(token) = self.tokenizer.next_data() else {
                        break;
                    };
                    if !matches!(token, DataToken::Doctype) {
                        self.consume_ordinary(token);
                    }
                    self.after_doctype = true;
                }
                InsertionMode::Html
                | InsertionMode::Head
                | InsertionMode::Body
                | InsertionMode::Paragraph => {
                    let Some(token) = self.tokenizer.next_data() else {
                        break;
                    };
                    self.consume_ordinary(token);
                }
            }
        }
        self.document
    }

    fn consume_ordinary(&mut self, token: DataToken<'_>) {
        match token {
            DataToken::Doctype => {}
            DataToken::StartTag { name } => {
                let Some(tag) = Tag::from_name(name) else {
                    return;
                };
                let id = self.append_node(NodeData::Element {
                    tag,
                    children: Vec::new(),
                });
                self.open.push(id);
            }
            DataToken::EndTag { name } => {
                let Some(id) = self.open.last().copied() else {
                    return;
                };
                if self.tag(id).name().eq_ignore_ascii_case(name) {
                    self.open.pop();
                }
            }
            DataToken::Characters(data) => {
                if self.drops_space(data) {
                    return;
                }
                self.append_text(data);
            }
        }
    }

    fn drops_space(&self, data: &str) -> bool {
        matches!(
            self.mode(),
            InsertionMode::Doctype | InsertionMode::Html | InsertionMode::Head
        ) && data.bytes().all(|byte| byte.is_ascii_whitespace())
    }

    fn append_node(&mut self, data: NodeData) -> NodeId {
        let parent = match self.open.last().copied() {
            Some(id) => ParentLink::Node(id),
            None => ParentLink::Document,
        };
        let id = NodeId(self.document.nodes.len());
        self.document.nodes.push(super::NodeRecord { parent, data });
        match parent {
            ParentLink::Document => self.document.children.push(id),
            ParentLink::Node(parent_id) => self.push_child(parent_id, id),
        }
        id
    }

    fn push_child(&mut self, parent_id: NodeId, child: NodeId) {
        let NodeData::Element { children, .. } = &mut self.document.nodes[parent_id.0].data else {
            unreachable!("open parent is an element");
        };
        children.push(child);
    }

    fn append_text(&mut self, data: &str) {
        if data.is_empty() {
            return;
        }
        self.append_node(NodeData::Text {
            data: data.to_owned(),
        });
    }

    fn tag(&self, id: NodeId) -> &Tag {
        let NodeData::Element { tag, .. } = &self.document.nodes[id.0].data else {
            unreachable!("open node is an element");
        };
        tag
    }
}
