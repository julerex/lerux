use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let src = scan::read("userspace/pds/sddf-shell/src/main.rs")?;
    let maths = scan::one(&src, "maths_repl")?;
    let menu = scan::one(&src, "app_menu")?;
    let source = scan::one(&src, "cmd_source")?;
    let quit = scan::between(maths, "text == b\"quit\"", "lerux_calc::eval")?;
    let browse = menu
        .find("browse()")
        .ok_or("app_menu does not call browse")?;
    let arm_end = menu[browse..]
        .find("maths_repl")
        .ok_or("app_menu has no maths arm")?;
    let arm = &menu[browse..browse + arm_end];
    let nested = scan::between(source, "SOURCING.swap", "source_file")?;
    Ok(vec![
        ("QuitReturns", quit.contains("return")),
        (
            "BrowserReturns",
            !arm.contains("break") && !arm.contains("return"),
        ),
        ("NestedRejected", nested.contains("return")),
    ])
}

pub fn behavior() -> Result<(), String> {
    expect(flags()?)
}

pub fn expect(flags: Vec<(&'static str, bool)>) -> Result<(), String> {
    for (name, value) in flags {
        if !value {
            return Err(message(name));
        }
    }
    Ok(())
}

fn message(name: &str) -> String {
    match name {
        "QuitReturns" => "maths_repl does not return on quit".to_string(),
        "BrowserReturns" => "app_menu does not return to the menu after browse".to_string(),
        "NestedRejected" => {
            "cmd_source does not return when a source is already running".to_string()
        }
        _ => format!("{name} is false"),
    }
}
