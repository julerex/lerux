mod device;
mod fat;
mod gpu;
mod html;
mod keys;
mod net;
mod scan;
mod sd;
mod shell;
mod tls;
mod wifi;

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!("usage: implicit probe|check ID");
        return ExitCode::from(2);
    };
    let Some(id) = args.next() else {
        eprintln!("usage: implicit probe|check ID");
        return ExitCode::from(2);
    };
    match cmd.as_str() {
        "probe" => match flags(&id) {
            Ok(lines) => {
                for (key, value) in lines {
                    let text = if value { "true" } else { "false" };
                    println!("{key}={text}");
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error={err}");
                ExitCode::from(2)
            }
        },
        "check" => match behavior(&id) {
            Ok(()) => ExitCode::SUCCESS,
            Err(Fail::Property(err)) => {
                eprintln!("verdict=fail {err}");
                ExitCode::from(1)
            }
            Err(Fail::Shape(err)) => {
                eprintln!("error={err}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("usage: implicit probe|check ID");
            ExitCode::from(2)
        }
    }
}

enum Fail {
    Property(String),
    Shape(String),
}

fn flags(id: &str) -> Result<Vec<(&'static str, bool)>, String> {
    match id {
        "shell" => shell::flags(),
        "wifi" => wifi::flags(),
        "net" => net::flags(),
        "gpu" => gpu::flags(),
        "fat" => fat::flags(),
        "keys" => keys::flags(),
        "device" => device::flags(),
        "sd" => sd::flags(),
        "html" => html::flags(),
        "tls" => tls::flags(),
        _ => Err(format!("unknown machine {id}")),
    }
}

fn behavior(id: &str) -> Result<(), Fail> {
    let run = match id {
        "shell" => shell::behavior(),
        "wifi" => wifi::behavior(),
        "net" => net::behavior(),
        "gpu" => gpu::behavior(),
        "fat" => fat::behavior(),
        "keys" => keys::behavior(),
        "device" => device::behavior(),
        "sd" => sd::behavior(),
        "html" => html::behavior(),
        "tls" => tls::behavior(),
        _ => Err(format!("unknown machine {id}")),
    };
    run.map_err(classify)
}

fn classify(err: String) -> Fail {
    if err.starts_with("shape:")
        || err.starts_with("no fn ")
        || err.starts_with("cannot read ")
        || err.starts_with("unbalanced ")
        || err.starts_with("missing `")
        || err.starts_with("unknown machine ")
        || err.contains(" fn ")
    {
        Fail::Shape(err)
    } else {
        Fail::Property(err)
    }
}
