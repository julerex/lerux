use lerux_ladybird_html::{Children, Document, Element, Node, Parent};

fn element<'a>(document: &'a Document, name: &str) -> Element<'a> {
    fn walk<'a>(children: Children<'a>, name: &str) -> Option<Element<'a>> {
        for node in children {
            let Node::Element(element) = node else {
                continue;
            };
            if element.name() == name {
                return Some(element);
            }
            if let Some(found) = walk(element.children(), name) {
                return Some(found);
            }
        }
        None
    }
    walk(document.children(), name).unwrap_or_else(|| panic!("element {name}"))
}

fn text_child<'a>(element: &Element<'a>) -> &'a str {
    let Some(Node::Text(text)) = element.children().next() else {
        panic!("text");
    };
    text.data()
}

#[test]
fn fixture_title_is_lerux() {
    let document = Document::parse(include_str!("../../../../support/browser/fixture.html"));
    assert_eq!(document.title_text(), "lerux");
}

#[test]
fn fixture_paragraph_is_lerux_http_fixture() {
    let document = Document::parse(include_str!("../../../../support/browser/fixture.html"));
    let paragraph = element(&document, "p");
    assert_eq!(paragraph.name(), "p");
    assert_eq!(text_child(&paragraph), "lerux-http-fixture");
}

#[test]
fn unknown_div_stays_an_element() {
    let document = Document::parse("<div></div>");
    let Some(Node::Element(div)) = document.children().next() else {
        panic!("div element");
    };
    assert_eq!(div.name(), "div");
    assert!(matches!(div.parent(), Parent::Document));
    assert!(div.children().next().is_none());
    assert!(document.children().nth(1).is_none());
}

#[test]
fn title_is_raw_text() {
    let document = Document::parse("<title>a<b>c</title>");
    assert_eq!(document.title_text(), "a<b>c");
    let Some(Node::Element(title)) = document.children().next() else {
        panic!("title element");
    };
    assert_eq!(title.name(), "title");
    let Some(Node::Text(text)) = title.children().next() else {
        panic!("title text");
    };
    assert_eq!(text.data(), "a<b>c");
    assert!(matches!(
        text.parent(),
        Parent::Element(parent) if parent.name() == "title"
    ));
    assert!(title.children().nth(1).is_none());
    assert!(!has_element(&document, "b"));
}

#[test]
fn newline_inside_html_is_not_a_child() {
    let document = Document::parse("<html>\n<head></head></html>");
    let Some(Node::Element(html)) = document.children().next() else {
        panic!("html element");
    };
    assert_eq!(html.name(), "html");
    assert!(document.children().nth(1).is_none());
    let mut children = html.children();
    let Some(Node::Element(head)) = children.next() else {
        panic!("head element");
    };
    assert_eq!(head.name(), "head");
    assert!(children.next().is_none());
}

#[test]
fn newline_inside_body_stays_before_the_paragraph() {
    let document = Document::parse("<body>\n<p>x</p></body>");
    let Some(Node::Element(body)) = document.children().next() else {
        panic!("body element");
    };
    assert_eq!(body.name(), "body");
    let mut children = body.children();
    let Some(Node::Text(newline)) = children.next() else {
        panic!("newline");
    };
    assert_eq!(newline.data(), "\n");
    assert!(matches!(
        newline.parent(),
        Parent::Element(parent) if parent.name() == "body"
    ));
    let Some(Node::Element(paragraph)) = children.next() else {
        panic!("paragraph");
    };
    assert_eq!(paragraph.name(), "p");
    assert_eq!(text_child(&paragraph), "x");
    assert!(children.next().is_none());
}

#[test]
fn empty_tag_name_adds_no_element() {
    let document = Document::parse("<>");
    assert!(document.children().next().is_none());
    assert_eq!(document.title_text(), "");
}

#[test]
fn empty_document_has_no_title_text() {
    assert_eq!(Document::empty().title_text(), "");
    assert!(Document::empty().children().next().is_none());
}

#[test]
fn whitespace_inside_head_is_not_a_child() {
    let document = Document::parse("<head> \t\n\r\x0c</head>");
    let head = element(&document, "head");
    assert!(head.children().next().is_none());
}

#[test]
fn paragraph_keeps_a_newline() {
    let document = Document::parse("<p>\n</p>");
    let paragraph = element(&document, "p");
    assert_eq!(text_child(&paragraph), "\n");
}

#[test]
fn title_keeps_its_whitespace() {
    let document = Document::parse("<title> \t\n</title>");
    assert_eq!(document.title_text(), " \t\n");
}

#[test]
fn leading_whitespace_adds_no_text() {
    let document = Document::parse("\n<html></html>");
    let Some(Node::Element(html)) = document.children().next() else {
        panic!("html element");
    };
    assert_eq!(html.name(), "html");
    assert!(document.children().nth(1).is_none());
    assert!(html.children().next().is_none());
}

#[test]
fn uppercase_name_is_lowercase() {
    let document = Document::parse("<DIV></DIV>");
    let div = element(&document, "div");
    assert_eq!(div.name(), "div");
    assert!(matches!(div.parent(), Parent::Document));
}

fn has_element(document: &Document, name: &str) -> bool {
    fn walk(children: Children<'_>, name: &str) -> bool {
        for node in children {
            let Node::Element(element) = node else {
                continue;
            };
            if element.name() == name || walk(element.children(), name) {
                return true;
            }
        }
        false
    }
    walk(document.children(), name)
}

#[test]
fn fixture_root_is_the_html_element() {
    let document = Document::parse(include_str!("../../../../support/browser/fixture.html"));
    let Some(Node::Element(html)) = document.children().next() else {
        panic!("html element");
    };
    assert_eq!(html.name(), "html");
    assert!(matches!(html.parent(), Parent::Document));
    assert!(document.children().nth(1).is_none());
}

#[test]
fn paragraph_is_not_wrapped() {
    let document = Document::parse("<p>x</p>");
    let Some(Node::Element(paragraph)) = document.children().next() else {
        panic!("paragraph");
    };
    assert_eq!(paragraph.name(), "p");
    assert!(matches!(paragraph.parent(), Parent::Document));
    assert_eq!(text_child(&paragraph), "x");
    assert!(document.children().nth(1).is_none());
}

#[test]
fn end_tag_pops_only_the_matching_top() {
    let document = Document::parse("<div><span></div><p>x</p>");
    let div = element(&document, "div");
    let mut children = div.children();
    let Some(Node::Element(span)) = children.next() else {
        panic!("span");
    };
    assert_eq!(span.name(), "span");
    assert!(children.next().is_none());
    let Some(Node::Element(paragraph)) = span.children().next() else {
        panic!("paragraph");
    };
    assert_eq!(paragraph.name(), "p");
    assert_eq!(text_child(&paragraph), "x");
}

#[test]
fn end_tag_makes_the_next_element_a_sibling() {
    let document = Document::parse("<div></div><p>x</p>");
    let mut children = document.children();
    let Some(Node::Element(div)) = children.next() else {
        panic!("div element");
    };
    assert_eq!(div.name(), "div");
    assert!(div.children().next().is_none());
    let Some(Node::Element(paragraph)) = children.next() else {
        panic!("paragraph");
    };
    assert_eq!(paragraph.name(), "p");
    assert!(matches!(paragraph.parent(), Parent::Document));
    assert_eq!(text_child(&paragraph), "x");
    assert!(children.next().is_none());
}

#[test]
fn head_keeps_text_that_is_not_only_whitespace() {
    let document = Document::parse("<head> x</head>");
    assert_eq!(text_child(&element(&document, "head")), " x");
}

#[test]
fn attributes_are_not_nodes() {
    let document = Document::parse("<p id=a>x</p>");
    let paragraph = element(&document, "p");
    assert_eq!(text_child(&paragraph), "x");
    assert!(paragraph.children().nth(1).is_none());
}

#[test]
fn character_reference_stays_the_source_bytes() {
    let document = Document::parse("<p>&amp;</p>");
    assert_eq!(text_child(&element(&document, "p")), "&amp;");
}

#[test]
fn title_end_tag_is_case_insensitive() {
    let document = Document::parse("<TITLE>lerux</TiTlE>");
    assert_eq!(document.title_text(), "lerux");
    let title = element(&document, "title");
    assert_eq!(title.name(), "title");
    assert_eq!(text_child(&title), "lerux");
}

#[test]
fn raw_text_keeps_a_longer_end_tag_name() {
    let document = Document::parse("<title>a</titleextra>b</title>");
    assert_eq!(document.title_text(), "a</titleextra>b");
}

#[test]
fn first_title_without_text_is_empty() {
    let document = Document::parse("<title></title><title>lerux</title>");
    assert_eq!(document.title_text(), "");
}

#[test]
fn whitespace_under_an_unknown_element_follows_the_open_mode() {
    let in_head = Document::parse("<head><div>\n</div></head>");
    assert!(element(&in_head, "div").children().next().is_none());

    let in_body = Document::parse("<body><div>\n</div></body>");
    assert_eq!(text_child(&element(&in_body, "div")), "\n");
}
