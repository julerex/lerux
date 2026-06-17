//! Host tool: populate an existing redoxfs image with the rustc smoke stub.
extern crate redoxfs;

use std::{env, fs, process, time};

use redoxfs::{DiskFile, FileSystem, Node, TreePtr};

fn usage() -> ! {
    eprintln!("redoxfs-populate DISK STUB_ELF");
    process::exit(1);
}

fn main() {
    env_logger::init();

    let mut args = env::args().skip(1);
    let Some(disk_path) = args.next() else {
        usage();
    };
    let Some(stub_path) = args.next() else {
        usage();
    };
    if args.next().is_some() {
        usage();
    }

    let stub_bytes = match fs::read(&stub_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("redoxfs-populate: failed to read stub {}: {}", stub_path, err);
            process::exit(1);
        }
    };

    let disk = match DiskFile::open(&disk_path) {
        Ok(disk) => disk,
        Err(err) => {
            eprintln!("redoxfs-populate: failed to open image {}: {}", disk_path, err);
            process::exit(1);
        }
    };

    let mut filesystem = match FileSystem::open(disk, None, None, true) {
        Ok(fs) => fs,
        Err(err) => {
            eprintln!("redoxfs-populate: failed to open filesystem on {}: {}", disk_path, err);
            process::exit(1);
        }
    };

    let ctime = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap();

    let root = TreePtr::<Node>::root();
    let bin_dir = match filesystem.tx(|tx| {
        tx.create_node(
            root,
            "bin",
            Node::MODE_DIR | 0o755,
            ctime.as_secs(),
            ctime.subsec_nanos(),
        )
    }) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("redoxfs-populate: failed to create bin dir: {}", err);
            process::exit(1);
        }
    };

    let rustc_node = match filesystem.tx(|tx| {
        tx.create_node(
            bin_dir.ptr(),
            "rustc",
            Node::MODE_FILE | 0o755,
            ctime.as_secs(),
            ctime.subsec_nanos(),
        )
    }) {
        Ok(node) => node,
        Err(err) => {
            eprintln!("redoxfs-populate: failed to create rustc node: {}", err);
            process::exit(1);
        }
    };

    if let Err(err) = filesystem.tx(|tx| {
        tx.write_node(
            rustc_node.ptr(),
            0,
            &stub_bytes,
            ctime.as_secs(),
            ctime.subsec_nanos(),
        )
    }) {
        eprintln!("redoxfs-populate: failed to write rustc stub: {}", err);
        process::exit(1);
    }

    if let Err(err) = filesystem.tx(|tx| tx.sync(true)) {
        eprintln!("redoxfs-populate: failed to sync: {}", err);
        process::exit(1);
    }

    eprintln!(
        "redoxfs-populate: wrote {} bytes to {}/bin/rustc",
        stub_bytes.len(),
        disk_path
    );
}
