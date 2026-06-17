#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(all(target_os = "redox", feature = "std"))]
extern crate daemon;
extern crate lerux_filesystem;
#[cfg(all(target_os = "redox", feature = "std"))]
extern crate syscall;
#[cfg(feature = "std")]
extern crate uuid;

#[cfg(feature = "std")]
use std::env;
#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::{self, Read, Write};
#[cfg(feature = "std")]
use std::os::unix::io::{FromRawFd, RawFd};
#[cfg(feature = "std")]
use std::process;

#[cfg(all(target_os = "redox", feature = "std"))]
use std::{mem::MaybeUninit, ptr::addr_of_mut, sync::atomic::Ordering};

use lerux_filesystem::FileSystem;
#[cfg(feature = "std")]
use lerux_filesystem::{DiskCache, DiskFile};
#[cfg(feature = "std")]
use termion::input::TermRead;
#[cfg(feature = "std")]
use uuid::Uuid;

#[cfg(all(target_os = "redox", feature = "std"))]
extern "C" fn unmount_handler(_s: usize) {
    lerux_filesystem::IS_UMT.store(1, Ordering::SeqCst);
}

#[cfg(all(target_os = "redox", feature = "std"))]
//set up a signal handler on redox, this implements unmounting. I have no idea what sa_flags is
//for, so I put 2. I don't think 0,0 is a valid sa_mask. I don't know what i'm doing here. When u
//send it a sigkill, it shuts off the filesystem
fn setsig() {
    // TODO: High-level wrapper like the nix crate?
    unsafe {
        let mut action = MaybeUninit::<libc::sigaction>::uninit();

        assert_eq!(
            libc::sigemptyset(addr_of_mut!((*action.as_mut_ptr()).sa_mask)),
            0
        );
        addr_of_mut!((*action.as_mut_ptr()).sa_flags).write(0);
        addr_of_mut!((*action.as_mut_ptr()).sa_sigaction).write(unmount_handler as usize);

        assert_eq!(
            libc::sigaction(libc::SIGTERM, action.as_ptr(), core::ptr::null_mut()),
            0
        );
    }
}

#[cfg(not(target_os = "redox"))]
// on linux, this is implemented properly, so no need for this unscrupulous nonsense!
fn setsig() {}

#[cfg(feature = "std")]
fn fork() -> isize {
    unsafe { libc::fork() as isize }
}

#[cfg(feature = "std")]
fn pipe(pipes: &mut [i32; 2]) -> isize {
    unsafe { libc::pipe(pipes.as_mut_ptr()) as isize }
}

#[cfg(not(target_os = "redox"))]
fn capability_mode() {}

#[cfg(not(target_os = "redox"))]
fn bootloader_password() -> Option<Vec<u8>> {
    None
}

#[cfg(all(target_os = "redox", feature = "std"))]
fn capability_mode() {
    libredox::call::setrens(0, 0).expect("redoxfs: failed to enter null namespace");
}

#[cfg(all(target_os = "redox", not(feature = "std")))]
fn capability_mode() {
    // no_std/runtime stub: full namespace transition will use redox-rt equivalents.
    // For the in-RAM smoke under the flag this is a no-op for now (mount still succeeds).
}

#[cfg(all(target_os = "redox", feature = "std"))]
fn bootloader_password() -> Option<Vec<u8>> {
    use libredox::call::MmapArgs;

    let addr_env = env::var_os("REDOXFS_PASSWORD_ADDR")?;
    let size_env = env::var_os("REDOXFS_PASSWORD_SIZE")?;

    let addr = usize::from_str_radix(
        addr_env.to_str().expect("REDOXFS_PASSWORD_ADDR not valid"),
        16,
    )
    .expect("failed to parse REDOXFS_PASSWORD_ADDR");

    let size = usize::from_str_radix(
        size_env.to_str().expect("REDOXFS_PASSWORD_SIZE not valid"),
        16,
    )
    .expect("failed to parse REDOXFS_PASSWORD_SIZE");

    let mut password = Vec::with_capacity(size);
    unsafe {
        let aligned_size = size.next_multiple_of(syscall::PAGE_SIZE);

        let fd = libredox::Fd::open("memory:physical", libredox::flag::O_CLOEXEC, 0)
            .expect("failed to open physical memory file");

        let password_map = libredox::call::mmap(MmapArgs {
            addr: core::ptr::null_mut(),
            length: aligned_size,
            prot: libredox::flag::PROT_READ,
            flags: libredox::flag::MAP_SHARED,
            fd: fd.raw(),
            offset: addr as u64,
        })
        .expect("failed to map REDOXFS_PASSWORD")
        .cast::<u8>();

        for i in 0..size {
            password.push(password_map.add(i).read());
        }

        let _ = libredox::call::munmap(password_map.cast(), aligned_size);
    }
    Some(password)
}

#[cfg(feature = "std")]
fn print_err_exit(err: impl AsRef<str>) -> ! {
    eprintln!("redoxfs: {}", err.as_ref());
    usage();
    process::exit(1)
}

#[cfg(feature = "std")]
fn print_usage_exit() -> ! {
    usage();
    process::exit(1)
}

#[cfg(feature = "std")]
fn usage() {
    eprintln!("redoxfs [--no-daemon|-d] [--memory MOUNTPOINT] [--disk-file PATH MOUNTPOINT] [--uuid] [disk or uuid] [mountpoint] [block in hex]");
}

#[cfg(feature = "std")]
enum DiskId {
    Path(String),
    Uuid(Uuid),
}

#[cfg(feature = "std")]
fn filesystem_by_path(
    path: &str,
    block_opt: Option<u64>,
    log_errors: bool,
) -> Option<(String, FileSystem<DiskCache<DiskFile>>)> {
    log::debug!("opening {}", path);
    let attempts = 10;
    for attempt in 0..=attempts {
        let password_opt = if attempt > 0 {
            eprint!("redoxfs: password: ");

            let password = io::stdin()
                .read_passwd(&mut io::stderr())
                .unwrap()
                .unwrap_or_default();

            eprintln!();

            if password.is_empty() {
                eprintln!("redoxfs: empty password, giving up");

                // Password is empty, exit loop
                break;
            }

            Some(password.into_bytes())
        } else {
            bootloader_password()
        };

        match DiskFile::open(path).map(DiskCache::new) {
            Ok(disk) => {
                match lerux_filesystem::FileSystem::open(disk, password_opt.as_deref(), block_opt, true) {
                    Ok(filesystem) => {
                        log::debug!(
                            "opened filesystem on {} with uuid {}",
                            path,
                            Uuid::from_bytes(filesystem.header.uuid()).hyphenated()
                        );

                        return Some((path.to_string(), filesystem));
                    }
                    Err(err) => match err.errno {
                        syscall::ENOKEY => {
                            if password_opt.is_some() {
                                eprintln!("redoxfs: incorrect password ({}/{})", attempt, attempts);
                            }
                        }
                        _ => {
                            if log_errors {
                                log::error!("failed to open filesystem {}: {}", path, err);
                            }
                            break;
                        }
                    },
                }
            }
            Err(err) => {
                if log_errors {
                    log::error!("failed to open image {}: {}", path, err);
                }
                break;
            }
        }
    }
    None
}

#[cfg(all(not(target_os = "redox"), feature = "std"))]
fn filesystem_by_uuid(
    _uuid: &Uuid,
    _block_opt: Option<u64>,
) -> Option<(String, FileSystem<DiskCache<DiskFile>>)> {
    None
}

#[cfg(all(target_os = "redox", feature = "std"))]
fn filesystem_by_uuid(
    uuid: &Uuid,
    block_opt: Option<u64>,
) -> Option<(String, FileSystem<DiskCache<DiskFile>>)> {
    use std::fs;

    use redox_path::RedoxPath;

    match fs::read_dir("/scheme") {
        Ok(entries) => {
            for entry_res in entries {
                if let Ok(entry) = entry_res {
                    if let Some(disk) = entry.path().to_str() {
                        if RedoxPath::from_absolute(disk)
                            .unwrap_or(RedoxPath::from_absolute("/")?)
                            .is_scheme_category("disk")
                        {
                            log::debug!("found scheme {}", disk);
                            match fs::read_dir(disk) {
                                Ok(entries) => {
                                    for entry_res in entries {
                                        if let Ok(entry) = entry_res {
                                            if let Ok(path) =
                                                entry.path().into_os_string().into_string()
                                            {
                                                log::debug!("found path {}", path);
                                                if let Some((path, filesystem)) =
                                                    filesystem_by_path(&path, block_opt, false)
                                                {
                                                    if &filesystem.header.uuid() == uuid.as_bytes()
                                                    {
                                                        log::debug!(
                                                            "filesystem on {} matches uuid {}",
                                                            path,
                                                            uuid.hyphenated()
                                                        );
                                                        return Some((path, filesystem));
                                                    } else {
                                                        log::debug!(
                                                            "filesystem on {} does not match uuid {}",
                                                            path,
                                                            uuid.hyphenated()
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::debug!("failed to list '{}': {}", disk, err);
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(err) => {
            log::error!("failed to list schemes: {}", err);
        }
    }

    None
}

#[cfg(feature = "std")]
fn daemon(
    disk_id: &DiskId,
    mountpoint: &str,
    block_opt: Option<u64>,
    mut write: Option<File>,
) -> ! {
    setsig();

    let filesystem_opt = match *disk_id {
        DiskId::Path(ref path) => filesystem_by_path(path, block_opt, true),
        DiskId::Uuid(ref uuid) => filesystem_by_uuid(uuid, block_opt),
    };

    if let Some((path, filesystem)) = filesystem_opt {
        match lerux_filesystem::mount(filesystem, mountpoint, |mounted_path| {
            capability_mode();

            log::info!(
                "mounted filesystem on {} to {}",
                path,
                mounted_path.display()
            );

            if let Some(ref mut write) = write {
                let _ = write.write(&[0]);
            }
        }) {
            Ok(()) => {
                process::exit(0);
            }
            Err(err) => {
                log::error!("failed to mount {} to {}: {}", path, mountpoint, err);
            }
        }
    }

    match *disk_id {
        DiskId::Path(ref path) => {
            log::error!("not able to mount path {}", path);
        }
        DiskId::Uuid(ref uuid) => {
            log::error!("not able to mount uuid {}", uuid.hyphenated());
        }
    }

    if let Some(ref mut write) = write {
        let _ = write.write(&[1]);
    }

    process::exit(1);
}

#[cfg(feature = "std")]
fn main() {
    env_logger::init();

    let mut args = env::args().skip(1);

    let mut daemonise = true;
    let mut memory_mode = false;
    let mut disk_file_path: Option<String> = None;
    let mut disk_id: Option<DiskId> = None;
    let mut mountpoint: Option<String> = None;
    let mut block_opt: Option<u64> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-daemon" | "-d" => daemonise = false,

            "--disk-file" => {
                #[cfg(not(target_os = "redox"))]
                print_err_exit("--disk-file is only supported on Redox");
                #[cfg(target_os = "redox")]
                {
                    let path = match args.next() {
                        Some(path) => path,
                        None => print_err_exit("no disk file path provided for --disk-file"),
                    };
                    let mnt = match args.next() {
                        Some(mnt) => mnt,
                        None => print_err_exit("no mountpoint provided for --disk-file"),
                    };
                    disk_file_path = Some(path);
                    mountpoint = Some(mnt);
                }
            }

            "--memory" => {
                #[cfg(not(target_os = "redox"))]
                print_err_exit("--memory is only supported on Redox");
                #[cfg(target_os = "redox")]
                {
                    memory_mode = true;
                }
            }

            "--uuid" if disk_id.is_none() => {
                disk_id = Some(DiskId::Uuid(
                    match args.next().as_deref().map(Uuid::parse_str) {
                        Some(Ok(uuid)) => uuid,
                        Some(Err(err)) => {
                            print_err_exit(format!("invalid uuid '{}': {}", arg, err))
                        }
                        None => print_err_exit("no uuid provided"),
                    },
                ));
            }

            disk if disk_id.is_none() && !memory_mode && disk_file_path.is_none() => {
                disk_id = Some(DiskId::Path(disk.to_owned()));
            }

            mnt if (disk_id.is_some() || memory_mode || disk_file_path.is_some()) && mountpoint.is_none() => {
                mountpoint = Some(mnt.to_owned());
            }

            opts if mountpoint.is_some() => match u64::from_str_radix(opts, 16) {
                Ok(block) => block_opt = Some(block),
                Err(err) => print_err_exit(format!("invalid block '{}': {}", opts, err)),
            },

            _ => print_usage_exit(),
        }
    }

    #[cfg(target_os = "redox")]
    if let Some(disk_path) = disk_file_path {
        let Some(mountpoint) = mountpoint else {
            print_err_exit("no mountpoint provided for --disk-file");
        };
        if disk_id.is_some() || block_opt.is_some() || memory_mode {
            print_err_exit("--disk-file does not accept disk, memory, or block arguments");
        }
        if daemonise {
            daemon::SchemeDaemon::new(|scheme_daemon| {
                disk_file_daemon(&disk_path, &mountpoint, scheme_daemon)
            });
        } else {
            eprintln!("redoxfs: --disk-file requires daemon mode under init");
            process::exit(1);
        }
    }

    #[cfg(target_os = "redox")]
    if memory_mode {
        let Some(mountpoint) = mountpoint else {
            print_err_exit("no mountpoint provided for --memory");
        };
        if disk_id.is_some() || block_opt.is_some() {
            print_err_exit("--memory does not accept disk or block arguments");
        }
        if daemonise {
            daemon::SchemeDaemon::new(|scheme_daemon| memory_daemon(&mountpoint, scheme_daemon));
        } else {
            eprintln!("redoxfs: --memory requires daemon mode under init");
            process::exit(1);
        }
    }

    #[cfg(not(target_os = "redox"))]
    if memory_mode {
        print_err_exit("--memory is only supported on Redox");
    }

    #[cfg(not(target_os = "redox"))]
    if disk_file_path.is_some() {
        print_err_exit("--disk-file is only supported on Redox");
    }

    let Some(disk_id) = disk_id else {
        print_err_exit("no disk provided");
    };

    let Some(mountpoint) = mountpoint else {
        print_err_exit("no mountpoint provided");
    };

    if daemonise {
        let mut pipes = [0; 2];
        if pipe(&mut pipes) == 0 {
            let mut read = unsafe { File::from_raw_fd(pipes[0] as RawFd) };
            let write = unsafe { File::from_raw_fd(pipes[1] as RawFd) };

            let pid = fork();
            if pid == 0 {
                drop(read);

                daemon(&disk_id, &mountpoint, block_opt, Some(write));
            } else if pid > 0 {
                drop(write);

                let mut res = [0];
                read.read_exact(&mut res).unwrap();

                process::exit(res[0] as i32);
            } else {
                panic!("redoxfs: failed to fork");
            }
        } else {
            panic!("redoxfs: failed to create pipe");
        }
    } else {
        log::info!("running in foreground");
        daemon(&disk_id, &mountpoint, block_opt, None);
    }
}

/// In-RAM smoke path: DiskMemory backend, init scheme registration, rustc stub delivery.
#[cfg(all(target_os = "redox", feature = "std"))]
fn populate_rustc_stub(
    filesystem: &mut FileSystem<lerux_filesystem::DiskMemory>,
    ctime_secs: u64,
    ctime_nanos: u32,
) {
    use lerux_filesystem::{Node, TreePtr};

    let rustc_bytes = match std::fs::read("/scheme/initfs/bin/rustc") {
        Ok(bytes) => bytes,
        Err(err) => {
            log::error!("redoxfs memory: failed to read rustc stub: {}", err);
            return;
        }
    };

    let root = TreePtr::<Node>::root();
    let bin_dir = match filesystem.tx(|tx| {
        tx.create_node(
            root,
            "bin",
            Node::MODE_DIR | 0o755,
            ctime_secs,
            ctime_nanos,
        )
    }) {
        Ok(dir) => dir,
        Err(err) => {
            log::error!("redoxfs memory: failed to create bin dir: {}", err);
            return;
        }
    };

    let rustc_node = match filesystem.tx(|tx| {
        tx.create_node(
            bin_dir.ptr(),
            "rustc",
            Node::MODE_FILE | 0o755,
            ctime_secs,
            ctime_nanos,
        )
    }) {
        Ok(node) => node,
        Err(err) => {
            log::error!("redoxfs memory: failed to create rustc node: {}", err);
            return;
        }
    };

    if let Err(err) = filesystem.tx(|tx| {
        tx.write_node(
            rustc_node.ptr(),
            0,
            &rustc_bytes,
            ctime_secs,
            ctime_nanos,
        )
    }) {
        log::error!("redoxfs memory: failed to write rustc stub: {}", err);
        return;
    }

    if let Err(err) = filesystem.tx(|tx| tx.sync(true)) {
        log::error!("redoxfs memory: failed to sync rustc stub: {}", err);
    }
}

/// In-RAM smoke path: DiskMemory backend, init scheme registration, rustc stub delivery.
#[cfg(all(target_os = "redox", feature = "std"))]
fn memory_daemon(mountpoint: &str, scheme_daemon: daemon::SchemeDaemon) -> ! {
    use std::time;

    let (ctime_secs, ctime_nanos) = {
        let ctime = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap();
        (ctime.as_secs(), ctime.subsec_nanos())
    };

    let disk = lerux_filesystem::DiskMemory::new(64 * 1024 * 1024);

    let mut filesystem = match FileSystem::create(disk, None, ctime_secs, ctime_nanos) {
        Ok(fs) => fs,
        Err(err) => {
            log::error!("redoxfs memory: failed to create in-RAM filesystem: {}", err);
            process::exit(1);
        }
    };

    populate_rustc_stub(&mut filesystem, ctime_secs, ctime_nanos);

    if let Err(err) = lerux_filesystem::mount_via_init(filesystem, mountpoint, scheme_daemon, |_mounted_path| {
        eprintln!("redoxfs mounted");
        capability_mode();
        log::info!("mounted filesystem (memory) to /scheme/{}", mountpoint);
    }) {
        log::error!("redoxfs memory: failed to mount to {}: {}", mountpoint, err);
        process::exit(1);
    }

    process::exit(0);
}

/// File-backed smoke path: open a disk image (e.g. staged in initfs) via DiskFile.
#[cfg(all(target_os = "redox", feature = "std"))]
fn disk_file_daemon(disk_path: &str, mountpoint: &str, scheme_daemon: daemon::SchemeDaemon) -> ! {
    // Initfs files are read-only; copy to the logging ramfs for writable block I/O.
    let open_path = if disk_path.starts_with("/scheme/initfs/") {
        let writable = "/scheme/logging/rustc-disk.img";
        match std::fs::copy(disk_path, writable) {
            Ok(_) => writable,
            Err(err) => {
                log::error!(
                    "redoxfs disk: failed to copy {} to {}: {}",
                    disk_path,
                    writable,
                    err
                );
                process::exit(1);
            }
        }
    } else {
        disk_path
    };

    let disk = match DiskFile::open(open_path).map(DiskCache::new) {
        Ok(disk) => disk,
        Err(err) => {
            log::error!("redoxfs disk: failed to open image {}: {}", open_path, err);
            process::exit(1);
        }
    };

    let filesystem = match FileSystem::open(disk, None, None, true) {
        Ok(fs) => fs,
        Err(err) => {
            log::error!("redoxfs disk: failed to open filesystem on {}: {}", open_path, err);
            process::exit(1);
        }
    };

    if let Err(err) = lerux_filesystem::mount_via_init(filesystem, mountpoint, scheme_daemon, |_mounted_path| {
        eprintln!("redoxfs mounted");
        capability_mode();
        log::info!("mounted filesystem (disk file {}) to /scheme/{}", disk_path, mountpoint);
    }) {
        log::error!("redoxfs disk: failed to mount {} at {}: {}", disk_path, mountpoint, err);
        process::exit(1);
    }

    process::exit(0);
}

#[cfg(all(target_os = "redox", not(feature = "std")))]
fn main() {
    use lerux_filesystem::DiskMemory;

    daemon::SchemeDaemon::new(|scheme_daemon| {
        let disk = DiskMemory::new(64 * 1024 * 1024);
        let filesystem = match FileSystem::create(disk, None, 0, 0) {
            Ok(fs) => fs,
            Err(_) => loop {},
        };

        if lerux_filesystem::mount_via_init(filesystem, "data", scheme_daemon, |_mounted_path| {
            capability_mode();
        })
        .is_err()
        {
            loop {}
        }
        loop {}
    });
}
