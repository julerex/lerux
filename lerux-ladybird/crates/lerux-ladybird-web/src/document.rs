use alloc::vec::Vec;

struct NodeId(usize);

impl NodeId {
    fn index(self) -> usize {
        self.0
    }
}

struct Node {
    children: Vec<NodeId>,
}

pub(crate) struct Document {
    nodes: Vec<Node>,
}

impl Document {
    pub(crate) fn empty() -> Self {
        Self {
            nodes: alloc::vec![Node {
                children: Vec::new(),
            }],
        }
    }

    pub(crate) fn root_child_count(&self) -> usize {
        let root = NodeId(0);
        self.nodes[root.index()].children.len()
    }
}
