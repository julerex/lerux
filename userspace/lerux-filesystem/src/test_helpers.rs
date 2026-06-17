//! Shared helpers for host integration tests (`std` + `test` only).
use crate::{DiskMemory, DiskSparse, FileSystem};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, time};

static IMAGE_SEQ: AtomicUsize = AtomicUsize::new(0);

/// Create a small in-RAM filesystem and run `callback` on it.
pub fn with_memory_fs<F, T>(size: u64, callback: F) -> T
where
    F: FnOnce(FileSystem<DiskMemory>) -> T,
{
    let disk = DiskMemory::new(size);
    let ctime = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap();
    let fs = FileSystem::create(disk, None, ctime.as_secs(), ctime.subsec_nanos()).unwrap();
    callback(fs)
}

/// Create a sparse-file-backed filesystem, run `callback`, then remove the temp image.
pub fn with_sparse_fs<F, T>(callback: F) -> T
where
    F: FnOnce(FileSystem<DiskSparse>) -> T,
{
    let disk_path = format!("/tmp/lerux-redoxfs-test-{}.img", IMAGE_SEQ.fetch_add(1, Ordering::Relaxed));
    let res = {
        let disk = DiskSparse::create(&disk_path, 64 * 1024 * 1024).unwrap();
        let ctime = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap();
        let fs = FileSystem::create(disk, None, ctime.as_secs(), ctime.subsec_nanos()).unwrap();
        callback(fs)
    };
    let _ = fs::remove_file(&disk_path);
    res
}

/// Path to a fresh temp sparse image (caller must remove).
pub fn temp_image_path() -> String {
    format!(
        "/tmp/lerux-redoxfs-golden-{}.img",
        IMAGE_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}
