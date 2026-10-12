//! File and network tools for one [`agent_tool_t`].
//!
//! The worker calls [`bind`] once, then [`run`] for each notification. A tool
//! that fails returns false and leaves the caller to mark the page. Browse
//! returns after `grok` has written the result, so this module does not
//! replace those bytes.

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

use sel4_microkit::Channel;

use lerux_cothread::microkit_cothread_wait_on_channel;
use lerux_posix::{Errno, Fd, Files, Open, Stat};
use lerux_sddf::{
    agent_bytes, fs_image, fs_queue_t, AGENT_BROWSE_CHANNEL, AGENT_FS_CHANNEL, AGENT_KIND_BROWSE,
    AGENT_KIND_EDIT, AGENT_KIND_LIST_DIR, AGENT_KIND_READ, AGENT_KIND_SEARCH, AGENT_KIND_WEB_FETCH,
    AGENT_KIND_WRITE, AGENT_RESULT_CAP, AGENT_STATE_BROWSE, AGENT_STATE_BROWSE_DONE,
};

use crate::{net, tool_page};

const NAME_MAX: usize = 12;
const LIST_MAX: usize = 32;
const FILE_MAX: usize = 512;
const S_IFMT: u64 = 0o170000;
const S_IFDIR: u64 = 0o040000;

struct FilesCell(UnsafeCell<MaybeUninit<Files>>);

// SAFETY: the worker binds this cell once and is the only caller afterwards.
unsafe impl Sync for FilesCell {}

static FILES: FilesCell = FilesCell(UnsafeCell::new(MaybeUninit::uninit()));
static BOUND: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
struct Name {
    bytes: [u8; NAME_MAX],
    len: u8,
}

/// Bind the filesystem client. The root has already initialised the network queues.
pub fn bind() {
    let commands = fs_image::FS_COMMAND_QUEUE_VADDR as *mut fs_queue_t;
    let completions = fs_image::FS_COMPLETION_QUEUE_VADDR as *mut fs_queue_t;
    let share = fs_image::FS_SHARE_VADDR as *mut u8;
    // SAFETY: the template maps these three regions. This worker is the only client.
    let files = unsafe {
        Files::bind(
            commands,
            completions,
            share,
            fs_image::FS_REGION_SIZE as usize,
            wake_fs,
            core::ptr::null_mut(),
        )
    };
    let Ok(files) = files else {
        return;
    };
    // SAFETY: one worker calls `bind` before `run`.
    unsafe { (*FILES.0.get()).write(files) };
    BOUND.store(true, Ordering::Relaxed);
}

/// Run the tool that is already on the shared page.
pub fn run() -> bool {
    let tool = tool_page();
    let kind = tool.kind;
    let Some(name) = agent_bytes(&tool.name, tool.name_len) else {
        return false;
    };
    let Some(arg) = agent_bytes(&tool.arg, tool.arg_len) else {
        return false;
    };
    let mut name_buf = [0u8; lerux_sddf::AGENT_NAME_CAP];
    let mut arg_buf = [0u8; lerux_sddf::AGENT_ARG_CAP];
    let name_len = name.len();
    let arg_len = arg.len();
    name_buf[..name_len].copy_from_slice(name);
    arg_buf[..arg_len].copy_from_slice(arg);
    let name = &name_buf[..name_len];
    let arg = &arg_buf[..arg_len];
    match kind {
        AGENT_KIND_READ => read(name),
        AGENT_KIND_WRITE => write(name, arg),
        AGENT_KIND_EDIT => edit(name, arg),
        AGENT_KIND_LIST_DIR => list_dir(name),
        AGENT_KIND_SEARCH => search(name, arg),
        AGENT_KIND_WEB_FETCH => web_fetch(name),
        AGENT_KIND_BROWSE => browse(),
        _ => false,
    }
}

fn wake_fs(_: *mut ()) {
    Channel::new(AGENT_FS_CHANNEL as usize).notify();
    microkit_cothread_wait_on_channel(AGENT_FS_CHANNEL as usize);
}

fn files() -> Option<&'static mut Files> {
    if !BOUND.load(Ordering::Relaxed) {
        return None;
    }
    // SAFETY: `bind` wrote this cell, and only the worker calls this.
    Some(unsafe { (*FILES.0.get()).assume_init_mut() })
}

fn read(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut buf = [0u8; AGENT_RESULT_CAP + 1];
    let Some(n) = read_file(name, &mut buf) else {
        return false;
    };
    if n > AGENT_RESULT_CAP {
        return false;
    }
    store(&buf[..n])
}

fn write(name: &[u8], text: &[u8]) -> bool {
    if name.is_empty() || text.is_empty() || text.len() > FILE_MAX {
        return false;
    }
    replace_file(name, text)
}

fn edit(name: &[u8], arg: &[u8]) -> bool {
    let Some((from, to)) = split_pipe(arg) else {
        return false;
    };
    if name.is_empty() || from.is_empty() {
        return false;
    }
    let mut src = [0u8; FILE_MAX];
    let Some(n) = read_file(name, &mut src) else {
        return false;
    };
    let mut next = [0u8; FILE_MAX];
    let Some(wrote) = replace_first(&src[..n], from, to, &mut next) else {
        return false;
    };
    if wrote > AGENT_RESULT_CAP || !replace_file(name, &next[..wrote]) {
        return false;
    }
    store(&next[..wrote])
}

fn list_dir(path: &[u8]) -> bool {
    let Some((names, count)) = collect_names(path) else {
        return false;
    };
    let mut out = [0u8; AGENT_RESULT_CAP];
    let Some(n) = join_names(&names, count, &mut out) else {
        return false;
    };
    store(&out[..n])
}

fn search(dir: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return false;
    }
    let Some((names, count)) = collect_names(dir) else {
        return false;
    };
    let mut matched = [Name {
        bytes: [0; NAME_MAX],
        len: 0,
    }; LIST_MAX];
    let mut found = 0usize;
    for name in &names[..count] {
        let bytes = &name.bytes[..usize::from(name.len)];
        let mut path = [0u8; NAME_MAX * 2 + 1];
        let Some(path_len) = child_path(dir, bytes, &mut path) else {
            return false;
        };
        let directory = match files().and_then(|files| files.stat(&path[..path_len]).ok()) {
            Some(stat) if is_dir(&stat) => true,
            Some(_) => false,
            None => return false,
        };
        if directory {
            continue;
        }
        let mut body = [0u8; FILE_MAX];
        let Some(n) = read_file(&path[..path_len], &mut body) else {
            return false;
        };
        if body[..n]
            .windows(needle.len())
            .any(|window| window == needle)
            && !insert(&mut matched, &mut found, bytes)
        {
            return false;
        }
    }
    let mut out = [0u8; AGENT_RESULT_CAP];
    let Some(n) = join_names(&matched, found, &mut out) else {
        return false;
    };
    store(&out[..n])
}

fn web_fetch(path: &[u8]) -> bool {
    let mut out = [0u8; AGENT_RESULT_CAP + 1];
    let Ok(n) = net::web_fetch(path, &mut out) else {
        return false;
    };
    if n > AGENT_RESULT_CAP {
        return false;
    }
    store(&out[..n])
}

fn browse() -> bool {
    let tool = tool_page();
    tool.state = AGENT_STATE_BROWSE;
    Channel::new(AGENT_BROWSE_CHANNEL as usize).notify();
    microkit_cothread_wait_on_channel(AGENT_BROWSE_CHANNEL as usize);
    let tool = tool_page();
    tool.state == AGENT_STATE_BROWSE_DONE && tool.result_len > 0
}

fn store(bytes: &[u8]) -> bool {
    if bytes.len() > AGENT_RESULT_CAP {
        return false;
    }
    let tool = tool_page();
    tool.result[..bytes.len()].copy_from_slice(bytes);
    tool.result_len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    true
}

fn read_file(name: &[u8], buf: &mut [u8]) -> Option<usize> {
    let fd = files()?.open(name, Open::Read).ok()?;
    let read = files().and_then(|files| files.read(fd, buf).ok());
    close_file(fd);
    read
}

fn replace_file(name: &[u8], bytes: &[u8]) -> bool {
    let removed = files().and_then(|files| match files.unlink(name) {
        Ok(()) => Some(()),
        Err(Errno(code)) if code == Errno::ENOENT => Some(()),
        Err(_) => None,
    });
    if removed.is_none() {
        return false;
    }
    let Some(fd) = files().and_then(|files| files.open(name, Open::CreateWrite).ok()) else {
        return false;
    };
    let wrote = files().and_then(|files| files.write(fd, bytes).ok());
    close_file(fd);
    wrote == Some(bytes.len())
}

fn close_file(fd: Fd) {
    if let Some(files) = files() {
        let _ = files.close(fd);
    }
}

fn collect_names(path: &[u8]) -> Option<([Name; LIST_MAX], usize)> {
    let dir = files()?.open_dir(path).ok()?;
    let mut names = [Name {
        bytes: [0; NAME_MAX],
        len: 0,
    }; LIST_MAX];
    let mut count = 0usize;
    let mut buf = [0u8; NAME_MAX];
    let mut failed = false;
    loop {
        let read = match files() {
            Some(files) => files.read_dir(dir, &mut buf),
            None => {
                failed = true;
                break;
            }
        };
        match read {
            Ok(None) => break,
            Ok(Some(0)) => {}
            Ok(Some(len)) => {
                if !insert(&mut names, &mut count, &buf[..len]) {
                    failed = true;
                    break;
                }
            }
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    close_dir(dir);
    if failed {
        None
    } else {
        Some((names, count))
    }
}

fn close_dir(dir: Fd) {
    if let Some(files) = files() {
        let _ = files.close_dir(dir);
    }
}

fn insert(names: &mut [Name], count: &mut usize, bytes: &[u8]) -> bool {
    if *count == names.len() || bytes.len() > NAME_MAX {
        return false;
    }
    let mut entry = Name {
        bytes: [0; NAME_MAX],
        len: u8::try_from(bytes.len()).unwrap_or(u8::MAX),
    };
    if usize::from(entry.len) != bytes.len() {
        return false;
    }
    entry.bytes[..bytes.len()].copy_from_slice(bytes);
    let mut index = *count;
    while index > 0 && before(&entry, &names[index - 1]) {
        names[index] = names[index - 1];
        index -= 1;
    }
    names[index] = entry;
    *count += 1;
    true
}

fn before(left: &Name, right: &Name) -> bool {
    left.bytes[..usize::from(left.len)] < right.bytes[..usize::from(right.len)]
}

fn join_names(names: &[Name], count: usize, out: &mut [u8]) -> Option<usize> {
    let mut len = 0usize;
    for (index, name) in names[..count].iter().enumerate() {
        let n = usize::from(name.len);
        let gap = usize::from(index > 0);
        if len + gap + n > out.len() {
            return None;
        }
        if gap == 1 {
            out[len] = b'\n';
            len += 1;
        }
        out[len..len + n].copy_from_slice(&name.bytes[..n]);
        len += n;
    }
    Some(len)
}

fn child_path(dir: &[u8], name: &[u8], out: &mut [u8]) -> Option<usize> {
    if dir.is_empty() || dir == b"." || dir == b"/" {
        if name.len() > out.len() {
            return None;
        }
        out[..name.len()].copy_from_slice(name);
        return Some(name.len());
    }
    if dir.len() + 1 + name.len() > out.len() {
        return None;
    }
    out[..dir.len()].copy_from_slice(dir);
    out[dir.len()] = b'/';
    let start = dir.len() + 1;
    out[start..start + name.len()].copy_from_slice(name);
    Some(start + name.len())
}

fn split_pipe(arg: &[u8]) -> Option<(&[u8], &[u8])> {
    let at = arg.iter().position(|byte| *byte == b'|')?;
    Some((&arg[..at], &arg[at + 1..]))
}

fn replace_first(src: &[u8], from: &[u8], to: &[u8], out: &mut [u8]) -> Option<usize> {
    if from.is_empty() {
        return None;
    }
    let at = src.windows(from.len()).position(|window| window == from)?;
    let len = src.len() - from.len() + to.len();
    if len > out.len() {
        return None;
    }
    out[..at].copy_from_slice(&src[..at]);
    out[at..at + to.len()].copy_from_slice(to);
    out[at + to.len()..len].copy_from_slice(&src[at + from.len()..]);
    Some(len)
}

fn is_dir(stat: &Stat) -> bool {
    stat.mode & S_IFMT == S_IFDIR
}
