use lerux_ladybird_html::Document;
use lerux_ladybird_web::Page;

#[test]
fn page_owns_one_empty_document() {
    let page = Page::new();
    let _root: &Document = page.document();
}
