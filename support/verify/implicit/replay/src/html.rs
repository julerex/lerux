use lerux_ladybird_html::Document;

use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let src = scan::read("lerux-ladybird/crates/lerux-ladybird-html/src/document/parse.rs")?;
    let finish = scan::one(&src, "finish")?;
    let arm = scan::between(finish, "InsertionMode::Title", "InsertionMode::Doctype")?;
    Ok(vec![(
        "TakesRaw",
        arm.contains("take_raw_text") && arm.contains("open.pop()"),
    )])
}

pub fn behavior() -> Result<(), String> {
    let src = scan::read("lerux-ladybird/crates/lerux-ladybird-html/src/document/parse.rs")?;
    let finish = scan::one(&src, "finish")?;
    let arm = scan::between(finish, "InsertionMode::Title", "InsertionMode::Doctype")?;
    if !arm.contains("take_raw_text") {
        return Err("the title arm no longer takes the title as raw text".to_string());
    }
    if !arm.contains("open.pop()") {
        return Err("the title arm no longer closes the title element".to_string());
    }
    let hi = Document::parse("<title>Hi</title>");
    if hi.title_text() != "Hi" {
        return Err("the title text was not Hi".to_string());
    }
    let raw = Document::parse("<title>a<b>c</title>");
    if raw.title_text() != "a<b>c" {
        return Err("the title did not keep a<b>c as text".to_string());
    }
    Ok(())
}
