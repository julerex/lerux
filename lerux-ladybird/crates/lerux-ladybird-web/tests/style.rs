use lerux_ladybird_html::Document;
use lerux_ladybird_web::{
    compute_style, style_document, BoxEdges, Color, Display, Edges, FontSize, Length, Rgba,
    StyleElement, StyleSheet, StyledElement, Width,
};

fn sheet(input: &str) -> StyleSheet {
    StyleSheet::parse(input).expect("style sheet")
}

fn element<'a>(name: &'a str, classes: &'a [&'a str]) -> StyleElement<'a> {
    StyleElement::new(name, classes, None)
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgba(Rgba::new(r, g, b, 0xff))
}

fn edges(top: i32, right: i32, bottom: i32, left: i32) -> BoxEdges {
    BoxEdges::Edges(Edges {
        top: Length::Px(top),
        right: Length::Px(right),
        bottom: Length::Px(bottom),
        left: Length::Px(left),
    })
}

fn find<'a>(element: &'a StyledElement, name: &str) -> &'a StyledElement {
    find_in(element, name).unwrap_or_else(|| panic!("missing {name}"))
}

fn find_in<'a>(element: &'a StyledElement, name: &str) -> Option<&'a StyledElement> {
    if element.name() == name {
        return Some(element);
    }
    element
        .children()
        .iter()
        .find_map(|child| find_in(child, name))
}

#[test]
fn author_class_overrides_user_agent_color_and_head_is_hidden() {
    // `.note` is earlier and more specific than `body`, so blue beats green.
    // Green on an unmarked body beats the user-agent black at equal specificity.
    let author = sheet(".note { color: #0000ff } body { color: #00ff00 }");
    let marked = compute_style(&[element("body", &["note"])], &author);
    assert_eq!(marked.color(), &rgb(0x00, 0x00, 0xff));
    assert_eq!(marked.display(), &Display::Block);
    assert_eq!(marked.margin(), &edges(8, 8, 8, 8));

    let plain = compute_style(&[element("body", &[])], &author);
    assert_eq!(plain.color(), &rgb(0x00, 0xff, 0x00));

    let document = Document::parse(include_str!("../../../../support/browser/fixture.html"));
    let styled = style_document(&document, &sheet(""));
    let html = find(&styled[0], "html");
    assert_eq!(find(html, "head").style().display(), &Display::None);
    assert_eq!(find(html, "title").style().display(), &Display::None);
}

#[test]
fn user_agent_hides_style_and_sets_the_fixture_margins() {
    let hidden = style_document(&Document::parse("<style></style>"), &sheet(""));
    assert_eq!(hidden[0].name(), "style");
    assert_eq!(hidden[0].style().display(), &Display::None);

    let document = Document::parse(include_str!("../../../../support/browser/fixture.html"));
    let styled = style_document(&document, &sheet(""));
    let html = find(&styled[0], "html");
    let body = find(html, "body");
    let paragraph = find(html, "p");
    assert_eq!(html.style().display(), &Display::Block);
    assert_eq!(body.style().display(), &Display::Block);
    assert_eq!(body.style().margin(), &edges(8, 8, 8, 8));
    assert_eq!(body.style().background_color(), &rgb(0xff, 0xff, 0xff));
    assert_eq!(paragraph.style().display(), &Display::Block);
    assert_eq!(paragraph.style().margin(), &edges(16, 0, 16, 0));
    assert_eq!(paragraph.style().color(), &rgb(0, 0, 0));
    assert_eq!(paragraph.style().background_color(), &Color::Transparent);
}

#[test]
fn source_order_breaks_an_equal_specificity_tie() {
    let author = sheet(".a { color: #111111 } .b { color: #222222 }");
    let style = compute_style(&[element("p", &["a", "b"])], &author);
    assert_eq!(style.color(), &rgb(0x22, 0x22, 0x22));
}

#[test]
fn an_id_beats_a_later_class() {
    let author = sheet("#main { color: #111111 } .note { color: #222222 }");
    let style = compute_style(&[StyleElement::new("p", &["note"], Some("main"))], &author);
    assert_eq!(style.color(), &rgb(0x11, 0x11, 0x11));
}

#[test]
fn a_later_declaration_in_the_same_rule_wins() {
    let style = compute_style(
        &[element("p", &[])],
        &sheet("p { color: #111111; color: #222222 }"),
    );
    assert_eq!(style.color(), &rgb(0x22, 0x22, 0x22));
}

#[test]
fn class_match_keeps_case_and_type_match_folds_it() {
    let author = sheet("P.Note { color: #0000ff }");
    let matched = compute_style(&[element("p", &["Note"])], &author);
    assert_eq!(matched.color(), &rgb(0x00, 0x00, 0xff));
    let missed = compute_style(&[element("p", &["note"])], &author);
    assert_eq!(missed.color(), &rgb(0, 0, 0));
}

#[test]
fn id_match_keeps_case() {
    let author = sheet("p#Main { color: #0000ff }");
    let matched = compute_style(&[StyleElement::new("p", &[], Some("Main"))], &author);
    assert_eq!(matched.color(), &rgb(0x00, 0x00, 0xff));
    let missed = compute_style(&[StyleElement::new("p", &[], Some("main"))], &author);
    assert_eq!(missed.color(), &rgb(0, 0, 0));
}

#[test]
fn child_combinator_does_not_match_a_grandchild() {
    let author = sheet("div p { color: #00ff00 } div > p { color: #0000ff }");
    let div = element("div", &[]);
    let paragraph = element("p", &[]);
    let grandchild = compute_style(&[div, element("span", &[]), paragraph], &author);
    assert_eq!(grandchild.color(), &rgb(0x00, 0xff, 0x00));
    let child = compute_style(&[div, paragraph], &author);
    assert_eq!(child.color(), &rgb(0x00, 0x00, 0xff));
}

#[test]
fn custom_properties_stay_unresolved() {
    let author = sheet("p { --Accent: #654ff0; color: var(--Accent); }");
    let style = compute_style(&[element("p", &[])], &author);
    assert_eq!(style.color(), &Color::Unresolved("var(--Accent)".into()));
    assert_eq!(style.custom_properties().len(), 1);
    assert_eq!(style.custom_properties()[0].name(), "--Accent");
    assert_eq!(style.custom_properties()[0].value(), "#654ff0");
}

#[test]
fn a_custom_property_inherits_until_the_element_sets_it() {
    let author = sheet("body { --Accent: #111111 } p { --Accent: #222222 }");
    let body = element("body", &[]);
    let paragraph = compute_style(&[body, element("p", &[])], &author);
    assert_eq!(paragraph.custom_properties()[0].value(), "#222222");
    let span = compute_style(&[body, element("span", &[])], &author);
    assert_eq!(span.custom_properties()[0].value(), "#111111");
}

#[test]
fn em_margin_uses_the_element_font_size() {
    let author = sheet("p { font-size: 20px; margin: 1em 0; width: 50%; }");
    let style = compute_style(&[element("p", &[])], &author);
    assert_eq!(style.font_size(), &FontSize::Px(20));
    assert_eq!(style.margin(), &edges(20, 0, 20, 0));
    assert_eq!(style.width(), &Width::Percent(50));
}

#[test]
fn short_hex_folds_case_and_expands() {
    let author = sheet("p { color: #FfF; background-color: #0000ff80 }");
    let style = compute_style(&[element("p", &[])], &author);
    assert_eq!(style.color(), &rgb(0xff, 0xff, 0xff));
    assert_eq!(
        style.background_color(),
        &Color::Rgba(Rgba::new(0x00, 0x00, 0xff, 0x80))
    );
}

#[test]
fn display_keyword_folds_ascii_case() {
    let style = compute_style(&[element("p", &[])], &sheet("p { display: None }"));
    assert_eq!(style.display(), &Display::None);
}

#[test]
fn margin_auto_computes_and_padding_auto_does_not() {
    let style = compute_style(
        &[element("p", &[])],
        &sheet("p { margin: auto; padding: auto; }"),
    );
    let auto = Length::Auto;
    assert_eq!(
        style.margin(),
        &BoxEdges::Edges(Edges {
            top: auto,
            right: auto,
            bottom: auto,
            left: auto,
        })
    );
    assert_eq!(style.padding(), &BoxEdges::Unresolved("auto".into()));
}

#[test]
fn an_author_type_rule_overrides_the_inherited_color() {
    let document = Document::parse("<p>x</p>");
    let styled = style_document(&document, &sheet("p { color: #010203 }"));
    assert_eq!(styled[0].name(), "p");
    assert_eq!(styled[0].style().color(), &rgb(0x01, 0x02, 0x03));
    assert_eq!(styled[0].style().display(), &Display::Block);
}

#[test]
fn color_inherits_and_background_does_not() {
    let parent = compute_style(&[element("body", &[])], &sheet(""));
    let child = compute_style(&[element("body", &[]), element("span", &[])], &sheet(""));
    assert_eq!(child.color(), parent.color());
    assert_eq!(child.font_size(), &FontSize::Px(16));
    assert_eq!(parent.background_color(), &rgb(0xff, 0xff, 0xff));
    assert_eq!(child.background_color(), &Color::Transparent);
    assert_eq!(child.display(), &Display::Inline);
    assert_eq!(parent.display(), &Display::Block);
}
