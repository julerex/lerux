use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let driver = scan::read("userspace/pds/sddf-gpu-driver/src/main.rs")?;
    let virt = scan::read("userspace/pds/sddf-gpu-virt/src/main.rs")?;
    let flush = scan::one(&driver, "flush")?;
    let responses = scan::one(&virt, "forward_responses")?;
    Ok(vec![
        (
            "FlushNeedsTransfer",
            flush.contains("resource.transfer") && flush.contains("GPU_RESP_ERR_INVALID_PARAMETER"),
        ),
        (
            "ClearsNote",
            responses.contains("note.live = false") && virt.contains("live: true"),
        ),
    ])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        if !value {
            return Err(match name {
                "FlushNeedsTransfer" => "flush can succeed without a staged transfer".to_string(),
                "ClearsNote" => "a display response leaves the note live".to_string(),
                _ => format!("{name} is false"),
            });
        }
    }
    Ok(())
}
