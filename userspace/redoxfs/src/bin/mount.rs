#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "std")]
extern crate libc;
extern crate redoxfs;
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

use redoxfs::FileSystem;
#[cfg(feature = "std")]
use redoxfs::{DiskCache, DiskFile};
#[cfg(feature = "std")]
use termion::input::TermRead;
#[cfg(feature = "std")]
use uuid::Uuid;

#[cfg(all(target_os = "redox", feature = "std"))]
extern "C" fn unmount_handler(_s: usize) {
    redoxfs::IS_UMT.store(1, Ordering::SeqCst);
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
    eprintln!("redoxfs [--no-daemon|-d] [--uuid] [disk or uuid] [mountpoint] [block in hex]");
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
                match redoxfs::FileSystem::open(disk, password_opt.as_deref(), block_opt, true) {
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
        match redoxfs::mount(filesystem, mountpoint, |mounted_path| {
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
    let mut disk_id: Option<DiskId> = None;
    let mut mountpoint: Option<String> = None;
    let mut block_opt: Option<u64> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-daemon" | "-d" => daemonise = false,

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

            disk if disk_id.is_none() => disk_id = Some(DiskId::Path(disk.to_owned())),

            mnt if disk_id.is_some() && mountpoint.is_none() => mountpoint = Some(mnt.to_owned()),

            opts if mountpoint.is_some() => match u64::from_str_radix(opts, 16) {
                Ok(block) => block_opt = Some(block),
                Err(err) => print_err_exit(format!("invalid block '{}': {}", opts, err)),
            },

            _ => print_usage_exit(),
        }
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

/// In-RAM smoke path (used for the rustc-hosting smoke to provide a writable /data fs
/// and deliver the rustc stub via the initfs oneshot).
/// Now available without `std` (DiskMemory and FileSystem::create are un-gated in the lib).
/// std-only blocks inside (e.g. the pre-mount rustc copy using std::fs) remain cfg-gated.
#[cfg(target_os = "redox")]
fn memory_daemon(mountpoint: &str, daemonise: bool) -> ! {
    #[cfg(feature = "std")]
    use std::time;

    #[cfg(feature = "std")]
    let (ctime_secs, ctime_nanos) = {
        let ctime = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap();
        (ctime.as_secs(), ctime.subsec_nanos())
    };
    #[cfg(not(feature = "std"))]
    let (ctime_secs, ctime_nanos) = (0u64, 0u32);

    let disk = redoxfs::DiskMemory::new(64 * 1024 * 1024);

    let filesystem = match FileSystem::create(
        disk,
        None,
        ctime_secs,
        ctime_nanos,
    ) {
        Ok(fs) => fs,
        Err(err) => {
            #[cfg(feature = "std")]
            log::error!("redoxfs memory: failed to create in-RAM filesystem: {}", err);
            #[cfg(feature = "std")]
            process::exit(1);
            #[cfg(not(feature = "std"))]
            loop {}
        }
    };

    #[cfg(feature = "std")]
    eprintln!("redoxfs mounted");

    #[cfg(feature = "std")]
    {
        let _ = std::fs::create_dir_all("/data/bin");
        if std::fs::copy("/scheme/initfs/bin/rustc", "/data/bin/rustc").is_ok() {
            let _ = std::process::Command::new("/data/bin/rustc").arg("--version").status();
            let _ = std::process::Command::new("/data/bin/rustc").status();
        } else {
            let _ = std::process::Command::new("/scheme/initfs/bin/rustc").arg("--version").status();
            let _ = std::process::Command::new("/scheme/initfs/bin/rustc").status();
        }
    }

    match redoxfs::mount(filesystem, mountpoint, |mounted_path| {
        capability_mode();

        #[cfg(feature = "std")]
        log::info!(
            "mounted filesystem (memory) to {}",
            mounted_path.display()
        );
    }) {
        Ok(()) => {
            #[cfg(feature = "std")]
            if !daemonise {
                process::exit(0);
            }
            loop {
                #[cfg(feature = "std")]
                std::thread::park();
                #[cfg(not(feature = "std"))]
                core::hint::spin_loop();
            }
        }
        Err(err) => {
            #[cfg(feature = "std")]
            log::error!("redoxfs memory: failed to mount to {}: {}", mountpoint, err);
            #[cfg(feature = "std")]
            process::exit(1);
            #[cfg(not(feature = "std"))]
            loop {}
        }
    }
}

#[cfg(all(target_os = "redox", not(feature = "std")))]
fn main() {
    // no_std + runtime entrypoint for the daemon under RUNTIME_REDOXFS flag.
    // The memory_daemon (in-RAM smoke fs for rustc copy + markers) is currently
    // behind feature="std" because DiskMemory/create/mount helpers in the lib are.
    // Once those are available no_std (additive lift of their cfgs + any internal std uses),
    // call memory_daemon("/data", true); here.
    // For now this keeps the bin compilable as no_std (lib already is); full smoke under
    // pure no_std daemon will follow after lib adjustments and redox-rt wiring.
    // Placeholder to satisfy bin main + allow build/link of no_std path.
    loop {
        core::hint::spin_loop();
    }
}
