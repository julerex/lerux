pub(crate) mod layout;
pub(crate) mod paint;
pub(crate) mod painted_page;
pub(crate) mod style;

/// Parse, lay out, and paint `html` into `bitmap`.
pub(crate) fn render(html: &str, bitmap: &mut crate::paint::Bitmap<'_>) {
    let doc = lerux_html::parse(html);
    #[expect(
        clippy::cast_possible_wrap,
        reason = "a bitmap side above i32::MAX wraps; layout keeps that coordinate"
    )]
    let tree = layout::layout_document(&doc, bitmap.width() as i32, bitmap.height() as i32);
    paint::paint(&tree, bitmap);
}
