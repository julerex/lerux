//! Minimal bootstrap "rustc" stand-in for the lerux rustc-hosting smoke.
//! Built with the same hybrid x86_64-unknown-redox + relibc sysroot cross setup
//! as the rest of early userspace. When staged into initfs and copied onto
//! a mounted redoxfs (/data), exec'ing it produces the RUSTC_SUCCESS_MARKERS.
//!
//! Markers (must appear on stdout/serial):
//!   - "rustc --version" (from --version mode)
//!   - "lerux-bootstrap-compiled" (from default/compile action)

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let exe = Path::new(&args[0]).file_name().and_then(|s| s.to_str()).unwrap_or("rustc");

    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("{} 1.80.0-lerux-bootstrap (x86_64-unknown-redox) (lerux 2026-06)", exe);
        println!("rustc --version");
        return;
    }

    // Default/"compile" action for the rustc-hosting smoke oneshot.
    println!("rustc --version");
    println!("lerux-bootstrap-compiled");

    let _ = fs::write("lerux-bootstrap-compiled.marker", "lerux-bootstrap-compiled\n");
}
