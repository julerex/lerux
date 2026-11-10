use lerux_ladybird_html::Document;
use lerux_ladybird_web::{
    Color, ComputedStyle, Display, Edges, StyleSheet, StyledDocument, StyledElement, StyledNode,
    Width,
};

const FIXTURE: &str = include_str!("../../../../support/browser/fixture.html");
const RED: Color = Color::opaque(0xff, 0, 0);
const BLUE: Color = Color::opaque(0, 0, 0xff);
const WHITE: Color = Color::opaque(0xff, 0xff, 0xff);

fn find<'s, 'a>(nodes: &'s [StyledNode<'a>], name: &str) -> Option<&'s StyledElement<'a>> {
    nodes.iter().find_map(|node| {
        let StyledNode::Element(styled) = node else {
            return None;
        };
        if styled.element().name() == name {
            Some(styled)
        } else {
            find(styled.children(), name)
        }
    })
}

fn styled<'s, 'a>(document: &'s StyledDocument<'a>, name: &str) -> &'s StyledElement<'a> {
    find(document.children(), name).unwrap_or_else(|| panic!("element {name}"))
}

fn style_of(html: &str, author: &str, name: &str) -> ComputedStyle {
    let document = Document::parse(html);
    let author = StyleSheet::parse(author).expect("author sheet");
    *styled(&StyledDocument::new(&document, &author), name).style()
}

fn all<T: Copy>(value: T) -> Edges<T> {
    Edges {
        top: value,
        right: value,
        bottom: value,
        left: value,
    }
}

#[test]
fn author_class_rule_overrides_the_user_agent_color() {
    let html = "<html><body class=note><p>x</p></body></html>";
    let author = ".note { color: #ff0000 }";
    assert_eq!(style_of(html, author, "body").color(), RED);
    assert_eq!(style_of(html, author, "html").color(), Color::BLACK);
    assert_eq!(style_of(html, "", "body").color(), Color::BLACK);
}

#[test]
fn head_and_title_compute_display_none() {
    assert_eq!(style_of(FIXTURE, "", "head").display(), Display::None);
    assert_eq!(style_of(FIXTURE, "", "title").display(), Display::None);
    assert_eq!(style_of(FIXTURE, "", "html").display(), Display::Block);
}

#[test]
fn user_agent_body_color_beats_an_inherited_color() {
    let html = "<html><body><p>x</p></body></html>";
    let author = "html { color: #0000ff }";
    assert_eq!(style_of(html, author, "html").color(), BLUE);
    assert_eq!(style_of(html, author, "body").color(), Color::BLACK);
    assert_eq!(style_of(html, author, "p").color(), Color::BLACK);
}

#[test]
fn fixture_gets_the_user_agent_boxes() {
    let body = style_of(FIXTURE, "", "body");
    assert_eq!(body.display(), Display::Block);
    assert_eq!(body.margin(), all(8));
    assert_eq!(body.background_color(), WHITE);

    let paragraph = style_of(FIXTURE, "", "p");
    assert_eq!(paragraph.display(), Display::Block);
    assert_eq!(
        paragraph.margin(),
        Edges {
            top: 16,
            right: 0,
            bottom: 16,
            left: 0,
        }
    );
    assert_eq!(paragraph.padding(), all(0));
    assert_eq!(paragraph.width(), Width::Auto);
    assert_eq!(paragraph.font_size(), 16);
    assert_eq!(paragraph.color(), Color::BLACK);
    assert_eq!(paragraph.background_color(), Color::TRANSPARENT);
}

#[test]
fn element_outside_the_user_agent_sheet_is_inline() {
    assert_eq!(
        style_of("<div>x</div>", "", "div").display(),
        Display::Inline
    );
}

#[test]
fn text_stays_in_tree_order_under_its_element() {
    let document = Document::parse(FIXTURE);
    let author = StyleSheet::parse("").expect("empty sheet");
    let styled_document = StyledDocument::new(&document, &author);
    let paragraph = styled(&styled_document, "p");
    let [StyledNode::Text(text)] = paragraph.children() else {
        panic!("one text child");
    };
    assert_eq!(text.data(), "lerux-http-fixture");
}

#[test]
fn class_beats_a_later_type_rule() {
    let html = "<p class=note>x</p>";
    let author = ".note { color: #ff0000 } p { color: #0000ff }";
    assert_eq!(style_of(html, author, "p").color(), RED);
}

#[test]
fn one_id_beats_two_classes() {
    let html = "<p id=main class=\"a b\">x</p>";
    let author = "#main { color: #ff0000 } .a.b { color: #0000ff }";
    assert_eq!(style_of(html, author, "p").color(), RED);
}

#[test]
fn later_declaration_wins_a_specificity_tie() {
    let rules = "p { color: #ff0000 } p { color: #0000ff }";
    assert_eq!(style_of("<p>x</p>", rules, "p").color(), BLUE);
    let one_rule = "p { color: #0000ff; color: #ff0000 }";
    assert_eq!(style_of("<p>x</p>", one_rule, "p").color(), RED);
}

#[test]
fn author_type_rule_beats_the_user_agent_type_rule() {
    let body = style_of(FIXTURE, "body { margin: 0; display: inline }", "body");
    assert_eq!(body.margin(), all(0));
    assert_eq!(body.display(), Display::Inline);
}

#[test]
fn selector_list_uses_its_most_specific_match() {
    let html = "<p class=note>x</p>";
    let author = "p, .note { color: #ff0000 } p { color: #0000ff }";
    assert_eq!(style_of(html, author, "p").color(), RED);
}

#[test]
fn child_and_descendant_combinators_walk_the_ancestors() {
    let html = "<html><body><p class=x>t</p></body></html>";
    let author = "html > p { color: #ff0000 } body > p { margin: 1px } html .x { padding: 2px }";
    let paragraph = style_of(html, author, "p");
    assert_eq!(paragraph.color(), Color::BLACK);
    assert_eq!(paragraph.margin(), all(1));
    assert_eq!(paragraph.padding(), all(2));
}

#[test]
fn color_and_font_size_inherit_and_the_rest_reset() {
    let author =
        "body { color: #ff0000; font-size: 20px; background-color: #00ff00; padding: 4px; width: 100px }";
    let body = style_of(FIXTURE, author, "body");
    assert_eq!(body.padding(), all(4));
    assert_eq!(body.width(), Width::Px(100));

    let paragraph = style_of(FIXTURE, author, "p");
    assert_eq!(paragraph.color(), RED);
    assert_eq!(paragraph.font_size(), 20);
    assert_eq!(paragraph.background_color(), Color::TRANSPARENT);
    assert_eq!(paragraph.padding(), all(0));
    assert_eq!(paragraph.width(), Width::Auto);
}

#[test]
fn value_outside_the_subset_keeps_the_earlier_value() {
    let author = "body { color: red; margin: 1em; padding: 1.5px; width: 50%; display: flex }";
    let body = style_of(FIXTURE, author, "body");
    assert_eq!(body.color(), Color::BLACK);
    assert_eq!(body.margin(), all(8));
    assert_eq!(body.padding(), all(0));
    assert_eq!(body.width(), Width::Auto);
    assert_eq!(body.display(), Display::Block);
}

#[test]
fn unresolved_value_keeps_the_earlier_value() {
    let author = "body { --accent: #ff0000; color: var(--accent); background-color: VAR(--x) }";
    let body = style_of(FIXTURE, author, "body");
    assert_eq!(body.color(), Color::BLACK);
    assert_eq!(body.background_color(), WHITE);
}

#[test]
fn shorthand_and_keyword_forms() {
    let author = "p { margin: 1px 2px 3PX; padding: 1px 2px 3px 4px; display: BLOCK; color: #F0a }";
    let paragraph = style_of("<p>x</p>", author, "p");
    assert_eq!(
        paragraph.margin(),
        Edges {
            top: 1,
            right: 2,
            bottom: 3,
            left: 2,
        }
    );
    assert_eq!(
        paragraph.padding(),
        Edges {
            top: 1,
            right: 2,
            bottom: 3,
            left: 4,
        }
    );
    assert_eq!(paragraph.display(), Display::Block);
    assert_eq!(paragraph.color(), Color::opaque(0xff, 0, 0xaa));
}
