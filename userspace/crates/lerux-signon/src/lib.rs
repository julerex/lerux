//! The text on the screen when the console accepts a line.
//!
//! The issue line uses the caller's describe string. The login line and the
//! prompt use the hostname `lerux`. The message of the day is the archiso
//! releng file, git blob `4d9eda1ede97f0fddcb959525b21306c8499c09e`.

#![no_std]

#[cfg(test)]
extern crate std;

/// `lerux login: root (automatic login)` and the blank line under it.
pub const LOGIN: &[u8] = b"lerux login: root (automatic login)\n\n";

/// Bash `PS1` for root on this host: `[root@lerux ~]# `.
pub const PROMPT: &[u8] = b"[root@lerux ~]# ";

/// archiso `configs/releng/airootfs/etc/motd`, escapes included.
pub static MOTD: &[u8] = include_bytes!("../../../../support/console/archiso-motd");

/// Issue line, login line, message of the day, then the prompt.
///
/// `describe` is `LERUX_GIT_DESCRIBE`. It is placed where Arch prints the kernel release.
pub fn for_each_preamble(describe: &[u8], mut emit: impl FnMut(&[u8])) {
    emit(b"Lerux ");
    emit(describe);
    emit(b" (tty1)\n\n");
    emit(LOGIN);
    emit(MOTD);
    emit(PROMPT);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    #[test]
    fn preamble_ends_at_the_prompt() {
        let mut buf = std::vec::Vec::new();
        for_each_preamble(b"test", |part| buf.extend_from_slice(part));
        assert!(buf.starts_with(b"Lerux test (tty1)\n\n"));
        assert!(buf.windows(LOGIN.len()).any(|window| window == LOGIN));
        assert!(buf.ends_with(PROMPT));
        assert!(buf.windows(MOTD.len()).any(|window| window == MOTD));
    }

    #[test]
    fn motd_is_the_archiso_releng_blob() {
        let mut child = Command::new("git")
            .args(["hash-object", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("git hash-object");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(MOTD)
            .expect("write motd");
        let out = child.wait_with_output().expect("git hash-object");
        assert!(out.status.success());
        let hash = std::str::from_utf8(&out.stdout).expect("hash text");
        assert_eq!(hash.trim(), "4d9eda1ede97f0fddcb959525b21306c8499c09e");
    }
}
