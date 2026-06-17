//! CLI integration tests for host redoxfs tools.
use assert_cmd::Command;
use lerux_filesystem::{DiskFile, FileSystem, Node, TreePtr};
use std::fs;
use std::io::Write;

fn temp_image() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.img");
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(&vec![0u8; 64 * 1024 * 1024]).unwrap();
    (dir, path)
}

#[test]
fn mkfs_creates_openable_image() {
    let (_dir, path) = temp_image();
    Command::cargo_bin("redoxfs-mkfs")
        .unwrap()
        .arg(&path)
        .assert()
        .success();
    let disk = DiskFile::open(&path).unwrap();
    assert!(FileSystem::open(disk, None, None, true).is_ok());
}

#[test]
fn populate_adds_rustc_stub_to_image() {
    let (_dir, path) = temp_image();
    Command::cargo_bin("redoxfs-mkfs")
        .unwrap()
        .arg(&path)
        .assert()
        .success();

    let stub_dir = tempfile::tempdir().unwrap();
    let stub_path = stub_dir.path().join("rustc");
    fs::write(&stub_path, b"#!/bin/sh\necho stub").unwrap();

    Command::cargo_bin("redoxfs-populate")
        .unwrap()
        .arg(&path)
        .arg(&stub_path)
        .assert()
        .success();

    let disk = DiskFile::open(&path).unwrap();
    let mut fs = FileSystem::open(disk, None, None, true).unwrap();
    let root = TreePtr::<Node>::root();
    let bin_dir = fs.tx(|tx| tx.find_node(root, "bin")).unwrap();
    let rustc = fs.tx(|tx| tx.find_node(bin_dir.ptr(), "rustc")).unwrap();
    assert_eq!(rustc.data().size(), b"#!/bin/sh\necho stub".len() as u64);
}

#[test]
fn mkfs_fails_without_disk_argument() {
    Command::cargo_bin("redoxfs-mkfs")
        .unwrap()
        .assert()
        .failure();
}
