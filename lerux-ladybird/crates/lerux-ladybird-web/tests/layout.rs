use lerux_ladybird_html::Document;
use lerux_ladybird_web::{LayoutBox, StyleSheet, StyledDocument};

const FIXTURE: &str = include_str!("../../../../support/browser/fixture.html");

fn names<'s, 'd>(boxes: &'s [LayoutBox<'s, 'd>], found: &mut Vec<&'d str>) {
    for layout_box in boxes {
        match layout_box {
            LayoutBox::Block(block) => {
                found.push(block.element().name());
                names(block.children(), found);
            }
            LayoutBox::Anonymous(anonymous) => names(anonymous.children(), found),
            LayoutBox::Line(_) => {}
        }
    }
}

fn line_text<'d>(boxes: &[LayoutBox<'_, 'd>]) -> &'d str {
    let [LayoutBox::Line(line)] = boxes else {
        panic!("line, got {boxes:?}");
    };
    line.text()
}

#[test]
fn fixture_paragraph_line_sits_under_the_body_margin() {
    let document = Document::parse(FIXTURE);
    let author = StyleSheet::parse("").expect("author sheet");
    let styled = StyledDocument::new(&document, &author);
    let tree = styled.layout();
    assert_eq!(tree.frame().width, 320);
    assert_eq!(tree.frame().height, 64);

    let [LayoutBox::Block(html)] = tree.children() else {
        panic!("html, got {:?}", tree.children());
    };
    assert_eq!(html.element().name(), "html");
    let [LayoutBox::Block(body)] = html.children() else {
        panic!("body, got {:?}", html.children());
    };
    assert_eq!(body.element().name(), "body");
    let [LayoutBox::Block(paragraph)] = body.children() else {
        panic!("paragraph, got {:?}", body.children());
    };
    assert_eq!(paragraph.element().name(), "p");
    let [LayoutBox::Line(line)] = paragraph.children() else {
        panic!("line, got {:?}", paragraph.children());
    };
    assert_eq!(line.text(), "lerux-http-fixture");
    assert_eq!(line.rect().x, 8);
    assert_eq!(line.rect().y, 24);
    assert_eq!(line.rect().width, 144);
    assert_eq!(line.rect().height, 8);
}

#[test]
fn head_and_title_generate_no_box() {
    let document = Document::parse(FIXTURE);
    let author = StyleSheet::parse("").expect("author sheet");
    let styled = StyledDocument::new(&document, &author);
    let tree = styled.layout();
    let mut found = Vec::new();
    names(tree.children(), &mut found);
    assert_eq!(found, ["html", "body", "p"]);
}

#[test]
fn sibling_vertical_margins_collapse_to_the_larger() {
    let html = "<div class=a>a</div><div class=b>b</div>";
    let author = ".a { display: block; margin: 10px 0 } .b { display: block; margin: 30px 0 }";
    let document = Document::parse(html);
    let sheet = StyleSheet::parse(author).expect("author sheet");
    let styled = StyledDocument::new(&document, &sheet);
    let tree = styled.layout();
    let [LayoutBox::Block(first), LayoutBox::Block(second)] = tree.children() else {
        panic!("two blocks, got {:?}", tree.children());
    };
    assert_eq!(first.rect().y, 10);
    assert_eq!(first.rect().height, 8);
    assert_eq!(second.rect().height, 8);
    assert_eq!(second.rect().y - (first.rect().y + first.rect().height), 30);
}

#[test]
fn mixed_children_wrap_each_text_run_in_an_anonymous_block() {
    let html = "<div class=mix>before<p>child</p>after</div>";
    let document = Document::parse(html);
    let author = StyleSheet::parse(".mix { display: block }").expect("author sheet");
    let styled = StyledDocument::new(&document, &author);
    let tree = styled.layout();
    let [LayoutBox::Block(mix)] = tree.children() else {
        panic!("mix, got {:?}", tree.children());
    };
    let [LayoutBox::Anonymous(before), LayoutBox::Block(paragraph), LayoutBox::Anonymous(after)] =
        mix.children()
    else {
        panic!("text, block, text, got {:?}", mix.children());
    };
    assert_eq!(line_text(before.children()), "before");
    assert_eq!(paragraph.element().name(), "p");
    assert_eq!(line_text(paragraph.children()), "child");
    assert_eq!(line_text(after.children()), "after");
}
