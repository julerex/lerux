//! Phase 47: single-writer advisory lock for hardware serial smoke.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};

/// Held exclusive lock for one hardware board (dropped on exit).
pub struct BoardLock {
    path: PathBuf,
}

impl BoardLock {
    /// Acquire `{lock_dir}/{board}.lock` with exclusive create.
    ///
    /// Env overrides:
    /// - `LERUX_HW_LOCK_DIR` — directory for lock files (default: `{tmp}/lerux-hw-locks`)
    /// - `LERUX_HW_LOCK_WAIT_SECS` — how long to wait for a busy lock (default 300)
    pub fn acquire(board: &str) -> Result<Self> {
        Self::acquire_in(lock_dir()?, board, wait_secs())
    }

    /// Same as [`acquire`](Self::acquire) with explicit directory (for tests).
    pub fn acquire_in(dir: PathBuf, board: &str, wait: u64) -> Result<Self> {
        fs::create_dir_all(&dir).with_context(|| format!("create lock dir {}", dir.display()))?;
        let path = dir.join(format!("{board}.lock"));
        let deadline = Instant::now() + Duration::from_secs(wait);

        loop {
            match try_create_lock(&path) {
                Ok(()) => {
                    println!("==> hardware lock acquired: {}", path.display());
                    return Ok(Self { path });
                }
                Err(_) if path.exists() => {
                    if Instant::now() >= deadline {
                        let holder = fs::read_to_string(&path).unwrap_or_default();
                        bail!(
                            "timed out after {wait}s waiting for hardware lock {} (holder: {})",
                            path.display(),
                            holder.trim()
                        );
                    }
                    // Stale lock: dead PID → steal.
                    if is_stale(&path)? {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(e) => return Err(e).context(format!("create lock {}", path.display())),
            }
        }
    }
}

impl Drop for BoardLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn lock_dir() -> Result<PathBuf> {
    if let Ok(d) = std::env::var("LERUX_HW_LOCK_DIR") {
        return Ok(PathBuf::from(d));
    }
    Ok(std::env::temp_dir().join("lerux-hw-locks"))
}

fn wait_secs() -> u64 {
    std::env::var("LERUX_HW_LOCK_WAIT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300)
}

fn try_create_lock(path: &Path) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    writeln!(f, "pid={} board_lock", process::id())?;
    f.sync_all()?;
    Ok(())
}

fn is_stale(path: &Path) -> Result<bool> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let Some(pid_str) = text.split_whitespace().find_map(|t| t.strip_prefix("pid=")) else {
        return Ok(true);
    };
    let Ok(pid) = pid_str.parse::<i32>() else {
        return Ok(true);
    };
    if pid <= 0 {
        return Ok(true);
    }
    // Linux: /proc/<pid> exists while the process is alive.
    #[cfg(target_os = "linux")]
    {
        Ok(!Path::new(&format!("/proc/{pid}")).exists())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn exclusive_lock() {
        let dir = tempdir().unwrap();
        let board = "test_board_lock_unit";
        let a = BoardLock::acquire_in(dir.path().to_path_buf(), board, 1).unwrap();
        let path = dir.path().join(format!("{board}.lock"));
        assert!(path.is_file());
        assert!(try_create_lock(&path).is_err());
        drop(a);
        let b = BoardLock::acquire_in(dir.path().to_path_buf(), board, 1).unwrap();
        drop(b);
        assert!(!path.exists());
    }
}
