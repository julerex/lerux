#![no_std]
#![no_main]

use core::{
    cell::UnsafeCell,
    mem::{size_of, MaybeUninit},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_posix::{Files, Open, Stat};
use lerux_sddf::{
    fs_client_config_t,
    fs_image::{
        self, FS_COMMAND_QUEUE_VADDR, FS_COMPLETION_QUEUE_VADDR, FS_REGION_SIZE, FS_SHARE_VADDR,
        FS_SHELL_SERVER_CHANNEL,
    },
    fs_queue_t, serial_cancel_consumer_signal, serial_client_config_t, serial_dequeue,
    serial_enqueue, serial_handle_from_connection,
    serial_image::{self, CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL},
    serial_queue_handle_t, serial_request_consumer_signal, serial_require_consumer_signal,
    LIONS_FS_MAGIC, SDDF_SERIAL_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const TX_CH: usize = CLIENT_TX_CHANNEL as usize;
const RX_CH: usize = CLIENT_RX_CHANNEL as usize;
const FS_CH: usize = FS_SHELL_SERVER_CHANNEL as usize;
const STACK_SIZE: usize = 0x4000;
const LINE_MAX: usize = 80;
const NAME_MAX: usize = 12;
const LIST_MAX: usize = 32;
const S_IFMT: u64 = 0o170000;
const S_IFDIR: u64 = 0o040000;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

struct Io {
    tx: serial_queue_handle_t,
    rx: serial_queue_handle_t,
}

struct IoCell(UnsafeCell<MaybeUninit<Io>>);

// SAFETY: init writes this cell before the worker starts. The root does not read it again.
unsafe impl Sync for IoCell {}

static IO: IoCell = IoCell(UnsafeCell::new(MaybeUninit::uninit()));

struct FilesCell(UnsafeCell<MaybeUninit<Files>>);

// SAFETY: the worker is the only user. The table is several kilobytes, so it stays off the stack.
unsafe impl Sync for FilesCell {}

static FILES: FilesCell = FilesCell(UnsafeCell::new(MaybeUninit::uninit()));

struct HandlerImpl;

#[derive(Clone, Copy)]
struct Name {
    bytes: [u8; NAME_MAX],
    len: u8,
}

extern "C" fn worker() {
    if !bind_files() {
        return;
    }
    say(b"lerux shell ready\n");
    let mut line = [0u8; LINE_MAX];
    loop {
        let len = read_line(&mut line);
        dispatch(&line[..len]);
    }
}

fn bind_files() -> bool {
    let commands = FS_COMMAND_QUEUE_VADDR as *mut fs_queue_t;
    let completions = FS_COMPLETION_QUEUE_VADDR as *mut fs_queue_t;
    let share = FS_SHARE_VADDR as *mut u8;
    // SAFETY: the template maps these three regions. This worker is the only client.
    let files = unsafe {
        Files::bind(
            commands,
            completions,
            share,
            FS_REGION_SIZE as usize,
            wake_fs,
            core::ptr::null_mut(),
        )
    };
    let Ok(files) = files else {
        return false;
    };
    // SAFETY: init spawned one worker, and the root does not read this cell.
    unsafe { (*FILES.0.get()).write(files) };
    true
}

fn wake_fs(_: *mut ()) {
    Channel::new(FS_CH).notify();
    microkit_cothread_wait_on_channel(FS_CH);
}

fn files() -> &'static mut Files {
    // SAFETY: `bind_files` wrote this cell, and only the worker calls this.
    unsafe { (*FILES.0.get()).assume_init_mut() }
}

fn io() -> &'static Io {
    // SAFETY: init wrote this cell before spawning the worker.
    unsafe { (*IO.0.get()).assume_init_ref() }
}

fn say(bytes: &[u8]) {
    let tx = &io().tx;
    for &byte in bytes {
        // SAFETY: `tx` is this client's mapped transmit queue. This worker produces it.
        while unsafe { serial_enqueue(tx, byte) } != 0 {
            unsafe { serial_request_consumer_signal(tx) };
            microkit_cothread_wait_on_channel(TX_CH);
        }
    }
    Channel::new(TX_CH).notify();
}

fn read_byte() -> u8 {
    let rx = &io().rx;
    loop {
        let mut byte = 0;
        // SAFETY: `rx` is this client's mapped receive queue. This worker consumes it.
        if unsafe { serial_dequeue(rx, &mut byte) } == 0 {
            if unsafe { serial_require_consumer_signal(rx) } {
                unsafe { serial_cancel_consumer_signal(rx) };
                Channel::new(RX_CH).notify();
            }
            return byte;
        }
        microkit_cothread_wait_on_channel(RX_CH);
    }
}

fn read_line(buf: &mut [u8]) -> usize {
    let mut len = 0;
    let mut overflow = false;
    loop {
        let byte = read_byte();
        if byte == b'\n' {
            continue;
        }
        if byte == b'\r' {
            return if overflow { 0 } else { len };
        }
        if len == buf.len() {
            overflow = true;
            continue;
        }
        buf[len] = byte;
        len += 1;
    }
}

fn dispatch(line: &[u8]) {
    let Some((cmd, rest)) = split_first(line) else {
        say(b"err\n");
        return;
    };
    match cmd {
        b"mkdir" => cmd_mkdir(rest),
        b"write" => cmd_write(rest),
        b"cat" => cmd_cat(rest),
        b"ls" => cmd_ls(rest),
        b"mv" => cmd_mv(rest),
        b"rm" => cmd_rm(rest),
        b"stat" => cmd_stat(rest),
        _ => say(b"err\n"),
    }
}

fn cmd_mkdir(rest: &[u8]) {
    let Some(name) = one_name(rest) else {
        say(b"mkdir err\n");
        return;
    };
    reply(files().mkdir(name).is_ok(), b"mkdir ok\n", b"mkdir err\n");
}

fn cmd_write(rest: &[u8]) {
    let Some((name, payload)) = split_first(rest) else {
        say(b"write err\n");
        return;
    };
    if name.is_empty() || payload.is_empty() {
        say(b"write err\n");
        return;
    }
    let fd = match files().open(name, Open::CreateWrite) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"write err\n");
            return;
        }
    };
    let wrote = files().write(fd, payload);
    let _ = files().close(fd);
    let ok = matches!(wrote, Ok(n) if n == payload.len());
    reply(ok, b"write ok\n", b"write err\n");
}

fn cmd_cat(rest: &[u8]) {
    let Some(name) = one_name(rest) else {
        say(b"cat err\n");
        return;
    };
    let fd = match files().open(name, Open::Read) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"cat err\n");
            return;
        }
    };
    let mut buf = [0u8; 512];
    let read = files().read(fd, &mut buf);
    let _ = files().close(fd);
    let Ok(n) = read else {
        say(b"cat err\n");
        return;
    };
    let mut out = [0u8; 513];
    out[..n].copy_from_slice(&buf[..n]);
    out[n] = b'\n';
    say(&out[..=n]);
}

fn cmd_ls(rest: &[u8]) {
    if rest.is_empty() {
        list_dir(b".");
        return;
    }
    let Some(name) = one_name(rest) else {
        say(b"ls err\n");
        return;
    };
    list_dir(name);
}

fn cmd_mv(rest: &[u8]) {
    let Some((from, tail)) = split_first(rest) else {
        say(b"mv err\n");
        return;
    };
    let Some(to) = one_name(tail) else {
        say(b"mv err\n");
        return;
    };
    reply(files().rename(from, to).is_ok(), b"mv ok\n", b"mv err\n");
}

fn cmd_rm(rest: &[u8]) {
    let Some(name) = one_name(rest) else {
        say(b"rm err\n");
        return;
    };
    reply(files().unlink(name).is_ok(), b"rm ok\n", b"rm err\n");
}

fn cmd_stat(rest: &[u8]) {
    let Some(name) = one_name(rest) else {
        say(b"stat err\n");
        return;
    };
    match files().stat(name) {
        Ok(stat) if is_dir(&stat) => say(b"stat dir\n"),
        Ok(stat) => say_size(stat.size),
        Err(_) => say(b"stat err\n"),
    }
}

fn list_dir(path: &[u8]) {
    let dir = match files().open_dir(path) {
        Ok(dir) => dir,
        Err(_) => {
            say(b"ls err\n");
            return;
        }
    };
    let mut names = [Name {
        bytes: [0; NAME_MAX],
        len: 0,
    }; LIST_MAX];
    let mut count = 0;
    let mut buf = [0u8; NAME_MAX];
    loop {
        match files().read_dir(dir, &mut buf) {
            Ok(None) => break,
            Ok(Some(0)) => {}
            Ok(Some(len)) => {
                if !insert(&mut names, &mut count, &buf[..len]) {
                    let _ = files().close_dir(dir);
                    say(b"ls err\n");
                    return;
                }
            }
            Err(_) => {
                let _ = files().close_dir(dir);
                say(b"ls err\n");
                return;
            }
        }
    }
    let _ = files().close_dir(dir);
    for name in &names[..count] {
        say_name(name);
    }
    say(b"ls ok\n");
}

fn insert(names: &mut [Name], count: &mut usize, bytes: &[u8]) -> bool {
    if *count == names.len() || bytes.len() > NAME_MAX {
        return false;
    }
    let mut entry = Name {
        bytes: [0; NAME_MAX],
        len: bytes.len() as u8,
    };
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

fn say_name(name: &Name) {
    let mut line = [0u8; NAME_MAX + 1];
    let len = usize::from(name.len);
    line[..len].copy_from_slice(&name.bytes[..len]);
    line[len] = b'\n';
    say(&line[..=len]);
}

fn say_size(size: u64) {
    let mut digits = [0u8; 20];
    let mut value = size;
    let mut index = digits.len();
    loop {
        index -= 1;
        digits[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let mut line = [0u8; 32];
    let prefix = b"stat size ";
    line[..prefix.len()].copy_from_slice(prefix);
    let len = digits.len() - index;
    line[prefix.len()..prefix.len() + len].copy_from_slice(&digits[index..]);
    let end = prefix.len() + len;
    line[end] = b'\n';
    say(&line[..=end]);
}

fn is_dir(stat: &Stat) -> bool {
    stat.mode & S_IFMT == S_IFDIR
}

fn reply(ok: bool, success: &'static [u8], failure: &'static [u8]) {
    if ok {
        say(success);
    } else {
        say(failure);
    }
}

fn one_name(rest: &[u8]) -> Option<&[u8]> {
    let (name, extra) = split_first(rest)?;
    if name.is_empty() || !extra.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn split_first(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let line = trim(line);
    if line.is_empty() {
        return None;
    }
    let Some(space) = line.iter().position(|byte| *byte == b' ') else {
        return Some((line, b""));
    };
    let (head, tail) = line.split_at(space);
    Some((head, trim(&tail[1..])))
}

fn trim(line: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = line.len();
    while start < end && line[start] == b' ' {
        start += 1;
    }
    while end > start && (line[end - 1] == b' ' || line[end - 1] == b'\r' || line[end - 1] == b'\n')
    {
        end -= 1;
    }
    &line[start..end]
}

#[protection_domain]
fn init() -> HandlerImpl {
    let serial_len = size_of::<serial_client_config_t>();
    let fs_len = size_of::<fs_client_config_t>();
    assert_eq!(CONFIG.len(), serial_len + fs_len);
    // SAFETY: the build script wrote a `serial_client_config_t` and then an `fs_client_config_t`.
    let serial = unsafe {
        serial_image::serial_config_from_bytes::<serial_client_config_t>(&CONFIG[..serial_len])
    };
    let fs = unsafe { fs_image::fs_config_from_bytes::<fs_client_config_t>(&CONFIG[serial_len..]) };
    assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(fs.magic, LIONS_FS_MAGIC);
    assert_eq!(serial.tx.id, CLIENT_TX_CHANNEL);
    assert_eq!(serial.rx.id, CLIENT_RX_CHANNEL);
    assert_eq!(fs.server.id, FS_SHELL_SERVER_CHANNEL);
    // SAFETY: the template maps this client's transmit and receive regions.
    let tx = unsafe { serial_handle_from_connection(&serial.tx) };
    let rx = unsafe { serial_handle_from_connection(&serial.rx) };
    unsafe { (*IO.0.get()).write(Io { tx, rx }) };
    // SAFETY: `STACKS` is exclusive to this init and the spawned worker.
    let stacks = unsafe { &mut *STACKS.0.get() };
    let ptrs = core::array::from_fn(|index| stacks[index].0.as_mut_ptr());
    // SAFETY: the static stacks outlive the protection domain and do not overlap.
    unsafe { microkit_cothread_init(STACK_SIZE, &ptrs) };
    let handle = microkit_cothread_spawn(worker, core::ptr::null_mut());
    assert_ne!(handle, NULL_HANDLE, "worker cothread slot is free");
    microkit_cothread_yield();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(Channel::new(TX_CH)) {
            microkit_cothread_recv_ntfn(TX_CH);
        }
        if channels.contains(Channel::new(RX_CH)) {
            microkit_cothread_recv_ntfn(RX_CH);
        }
        if channels.contains(Channel::new(FS_CH)) {
            microkit_cothread_recv_ntfn(FS_CH);
        }
        Ok(())
    }
}
