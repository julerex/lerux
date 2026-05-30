use std::fs::File;

use redox_log::{OutputBuilder, RedoxLogger};
use log::{debug, error, info, trace, warn};

fn main() {
    dbg!(RedoxLogger::new()
        .with_output(
            OutputBuilder::with_endpoint(
                File::create("file.log").expect("failed to open log file")
            )
            .with_filter(log::LevelFilter::Trace)
            .build()
        )
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Debug)
                .with_ansi_escape_codes()
                .build()
        )
        .with_process_name("simple".into())
        .enable().expect("failed to enable"));
    info!("Example started");
    debug!("example started with log file: {}", "file.log");
    trace!("useless comment");
    warn!("useless comment is useless");
    error!("deadlock");
    loop {}
}
