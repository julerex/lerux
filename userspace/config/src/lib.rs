#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use libredox::Fd;
use libredox::flag::O_RDONLY;

pub fn config_for_dirs(dirs: &[&str]) -> Result<Vec<String>, syscall::Error> {
    let mut entries = BTreeMap::new();

    for dir in dirs {
        let Ok(fd) = Fd::open(*dir, O_RDONLY, 0) else {
            continue;
        };
        let mut buf = [0u8; 4096];
        let n = fd.read(&mut buf)?;
        let text = core::str::from_utf8(&buf[..n]).map_err(|_| syscall::Error::new(syscall::EINVAL))?;
        for line in text.lines() {
            let name = line.trim();
            if name.is_empty() || name.starts_with('#') {
                continue;
            }
            let path = if dir.ends_with('/') {
                format!("{dir}{name}")
            } else {
                format!("{dir}/{name}")
            };
            entries.insert(name.to_string(), path);
        }
    }

    Ok(entries.into_values().collect())
}

pub fn config_for_initfs(name: &str) -> Result<Vec<String>, syscall::Error> {
    config_for_dirs(&[
        &alloc::format!("/scheme/initfs/lib/{name}.d"),
        &alloc::format!("/scheme/initfs/etc/{name}.d"),
    ])
}
