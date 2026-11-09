use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let src = scan::read("userspace/pds/sddf-fatfs/src/main.rs")?;
    let opens = ["create_file", "open_read", "open_nested_read"];
    let mut second = true;
    for name in opens {
        let body = scan::one(&src, name)?;
        second &= body.contains("file_busy()");
    }
    let dir = scan::one(&src, "handle_dir_open")?;
    second &= dir.contains("dir.is_some()");
    let load = scan::one(&src, "load_block")?;
    Ok(vec![
        ("SecondRefused", second),
        ("FlushesFirst", load.contains("flush_block()")),
    ])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        if !value {
            return Err(match name {
                "SecondRefused" => "a second open is no longer refused".to_string(),
                "FlushesFirst" => "load_block reads another block without flushing".to_string(),
                _ => format!("{name} is false"),
            });
        }
    }
    Ok(())
}
