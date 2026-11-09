use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    let src = scan::read("userspace/pds/emmc2-driver/src/emmc2.rs")?;
    let identify = scan::one(&src, "identify_card")?;
    let read = scan::one(&src, "read_sector")?;
    let write = scan::one(&src, "write_sector")?;
    Ok(vec![
        ("OrderOk", command_order(identify)),
        (
            "ReadChecksInit",
            read.contains("!self.initialized") && write.contains("!self.initialized"),
        ),
    ])
}

pub fn behavior() -> Result<(), String> {
    for (name, value) in flags()? {
        if !value {
            return Err(match name {
                "OrderOk" => {
                    "identify_card no longer selects the card before setting the block length"
                        .to_string()
                }
                "ReadChecksInit" => {
                    "a sector transfer runs before the card is identified".to_string()
                }
                _ => format!("{name} is false"),
            });
        }
    }
    Ok(())
}

fn command_order(body: &str) -> bool {
    let need = [0u32, 8, 2, 3, 7, 16];
    let mut at = 0;
    let mut rest = body;
    while let Some(found) = rest.find("send_cmd(") {
        rest = &rest[found + "send_cmd(".len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(number) = digits.parse::<u32>() else {
            continue;
        };
        if at < need.len() && number == need[at] {
            at += 1;
        }
    }
    at == need.len()
}
