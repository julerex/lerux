use lerux_ladybird_html::Document;

/// Hosts one document.
pub struct Page {
    document: Document,
}

impl Page {
    /// A page whose document is empty.
    #[expect(
        clippy::new_without_default,
        reason = "Default would be a second constructor for the same page"
    )]
    pub const fn new() -> Self {
        Self {
            document: Document::empty(),
        }
    }

    pub const fn document(&self) -> &Document {
        &self.document
    }
}
