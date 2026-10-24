use crate::document::Document;

#[must_use]
pub struct Page {
    document: Document,
}

impl Page {
    #[expect(
        clippy::new_without_default,
        reason = "Default would be a second constructor for the same empty page"
    )]
    pub fn new() -> Self {
        Self {
            document: Document::empty(),
        }
    }

    pub fn root_child_count(&self) -> usize {
        self.document.root_child_count()
    }
}

#[cfg(test)]
mod tests {
    use super::Page;

    #[test]
    fn new_page_owns_an_empty_document() {
        let page = Page::new();
        assert_eq!(page.root_child_count(), 0);
    }
}
