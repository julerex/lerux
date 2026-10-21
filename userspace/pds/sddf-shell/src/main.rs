#![no_std]
#![no_main]

#[cfg(any(
    all(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-x86_64_generic_fetch_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-qemu_virt_aarch64_web_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-x86_64_generic_web_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    ),
))]
compile_error!(
    "the fetch image, the web image, the browser image, and the filesystem image do not share channel 3"
);

#[cfg(any(
    all(
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-qemu_virt_aarch64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-qemu_virt_aarch64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-x86_64_generic_web_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-x86_64_generic_web_sddf",
        feature = "board-qemu_virt_aarch64_fetch_sddf"
    ),
    all(
        feature = "board-x86_64_generic_web_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-x86_64_generic_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-x86_64_generic_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-x86_64_generic_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_aarch64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-qemu_virt_aarch64_browser_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-qemu_virt_riscv64_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_aarch64_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-qemu_virt_aarch64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-x86_64_generic_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-qemu_virt_aarch64_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-qemu_virt_riscv64_web_sddf"
    ),
    all(
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ),
))]
compile_error!("the fetch image, the web image, and the browser image are separate boards");

use core::{
    cell::UnsafeCell,
    mem::{size_of, MaybeUninit},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
use sel4_microkit::{get_mr, MessageInfo};

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

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
use lerux_sddf::{
    date_from_ns, sched_config_t, sched_from_bytes, timer_client_config_t, timer_config_from_bytes,
    FS_IMAGE_ROLES, SDDF_TIMER_GET_TIME, SDDF_TIMER_MAGIC, SHELL_TIMER_CHANNEL,
    TIMER_CLIENT_CHANNEL,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const TX_CH: usize = CLIENT_TX_CHANNEL as usize;
const RX_CH: usize = CLIENT_RX_CHANNEL as usize;
const FS_CH: usize = FS_SHELL_SERVER_CHANNEL as usize;
#[cfg(feature = "net-client")]
const STACK_SIZE: usize = 0x1_0000;
#[cfg(not(feature = "net-client"))]
const STACK_SIZE: usize = 0x4000;
const LINE_MAX: usize = 80;
const NAME_MAX: usize = 12;
const LIST_MAX: usize = 32;
const S_IFMT: u64 = 0o170000;
const S_IFDIR: u64 = 0o040000;

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
mod gpu;
#[cfg(feature = "net-client")]
mod net;
#[cfg(feature = "net-client")]
mod net_device;

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

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
const LOG_CAP: usize = 2048;

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
struct Log {
    bytes: [u8; LOG_CAP],
    len: usize,
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
struct LogCell(UnsafeCell<Log>);

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
// SAFETY: the worker is the only user. The root does not read the log.
unsafe impl Sync for LogCell {}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
static LOG: LogCell = LogCell(UnsafeCell::new(Log {
    bytes: [0; LOG_CAP],
    len: 0,
}));

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
struct SchedCell(UnsafeCell<MaybeUninit<sched_config_t>>);

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
// SAFETY: init writes this cell before the worker starts. The worker only reads it.
unsafe impl Sync for SchedCell {}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
static SCHED: SchedCell = SchedCell(UnsafeCell::new(MaybeUninit::uninit()));

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
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-x86_64_generic_web_sddf"
    ))]
    serve_seeded_file();
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))]
    browse();
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

#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
fn serve_seeded_file() {
    let mut body = [0u8; 64];
    let n = read_seed(&mut body);
    if n == 0 {
        say(b"web err\n");
        return;
    }
    let Ok(fd) = net::begin_listen() else {
        say(b"web err\n");
        return;
    };
    say(b"web listening\n");
    if net::serve_one(fd, &body[..n]).is_err() {
        say(b"web err\n");
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn browse() {
    let mut body = [0u8; 512];
    match net::fetch_body(&mut body) {
        Ok(n) => match page_title(&body[..n]) {
            Some(title) => say_title(title),
            None => say(b"title err\n"),
        },
        Err(()) => say(b"title err\n"),
    }
    if gpu::paint().is_ok() {
        say(b"pixel 4c5258ff\n");
    } else {
        say(b"pixel err\n");
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn page_title(body: &[u8]) -> Option<&[u8]> {
    let open = b"<title>";
    let close = b"</title>";
    let start = body.windows(open.len()).position(|window| window == open)? + open.len();
    let rest = body.get(start..)?;
    let end = rest
        .windows(close.len())
        .position(|window| window == close)?;
    let title = rest.get(..end)?;
    if title.is_empty() || title.len() > 32 {
        return None;
    }
    Some(title)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn say_title(title: &[u8]) {
    let mut line = [0u8; 40];
    let prefix = b"title ";
    line[..prefix.len()].copy_from_slice(prefix);
    let mut len = prefix.len();
    line[len..len + title.len()].copy_from_slice(title);
    len += title.len();
    line[len] = b'\n';
    say(&line[..=len]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
fn read_seed(buf: &mut [u8]) -> usize {
    let Ok(fd) = files().open(b"/config/hostname", Open::Read) else {
        return 0;
    };
    let read = files().read(fd, buf);
    let _ = files().close(fd);
    read.unwrap_or(0)
}

fn io() -> &'static Io {
    // SAFETY: init wrote this cell before spawning the worker.
    unsafe { (*IO.0.get()).assume_init_ref() }
}

fn say(bytes: &[u8]) {
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    ))]
    store_serial(bytes);
    emit(bytes);
}

fn emit(bytes: &[u8]) {
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
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"fetch" => cmd_fetch(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"calc" => cmd_calc(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"backup" => cmd_backup(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"source" => cmd_source(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"cert" => cmd_cert(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"edit" => cmd_edit(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fetch_sddf",
            feature = "board-qemu_virt_riscv64_fetch_sddf",
            feature = "board-x86_64_generic_fetch_sddf"
        ))]
        b"chat" => cmd_chat(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        b"config" => cmd_config(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        b"date" => cmd_date(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        b"ps" => cmd_table(rest, b"ps ok\n", b"ps err\n"),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        b"qos" => cmd_table(rest, b"qos ok\n", b"qos err\n"),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        b"dmesg" => cmd_dmesg(rest),
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        b"reboot" => cmd_reboot(rest),
        _ => say(b"err\n"),
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn cmd_config(rest: &[u8]) {
    let Some((verb, key)) = split_first(rest) else {
        say(b"config err\n");
        return;
    };
    if verb != b"get" || key != b"hostname" {
        say(b"config err\n");
        return;
    }
    let fd = match files().open(b"/config/hostname", Open::Read) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"config err\n");
            return;
        }
    };
    let mut buf = [0u8; 64];
    let read = files().read(fd, &mut buf);
    let _ = files().close(fd);
    let Ok(n) = read else {
        say(b"config err\n");
        return;
    };
    if n == 0 {
        say(b"config err\n");
        return;
    }
    let mut out = [0u8; 65];
    out[..n].copy_from_slice(&buf[..n]);
    out[n] = b'\n';
    say(&out[..=n]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn cmd_date(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"date err\n");
        return;
    }
    // The worker is the kernel thread while it is switched in, so the protected call runs here.
    let reply = Channel::new(usize::from(SHELL_TIMER_CHANNEL))
        .pp_call(MessageInfo::new(SDDF_TIMER_GET_TIME, 0));
    if reply.count() < 1 {
        say(b"date err\n");
        return;
    }
    let ns = get_mr(0);
    if ns == 0 {
        say(b"date err\n");
        return;
    }
    let (year, month, day) = date_from_ns(ns);
    let mut line = [0u8; 11];
    write_width(&mut line[0..4], year);
    line[4] = b'-';
    write_width(&mut line[5..7], u16::from(month));
    line[7] = b'-';
    write_width(&mut line[8..10], u16::from(day));
    line[10] = b'\n';
    say(&line);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn write_width(buf: &mut [u8], mut value: u16) {
    for index in (0..buf.len()).rev() {
        buf[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn cmd_table(rest: &[u8], ok: &'static [u8], err: &'static [u8]) {
    if !rest.is_empty() {
        say(err);
        return;
    }
    let page = schedule();
    for index in 0..usize::from(page.count) {
        say_role(&page.entry[index]);
    }
    say(ok);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn say_role(entry: &lerux_sddf::sched_entry_t) {
    let mut line = [0u8; 40];
    let name_len = entry
        .name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(entry.name.len());
    line[..name_len].copy_from_slice(&entry.name[..name_len]);
    let mut len = name_len;
    line[len] = b' ';
    len += 1;
    len += push_dec(&mut line[len..], u32::from(entry.priority));
    line[len] = b' ';
    len += 1;
    len += push_dec(&mut line[len..], entry.budget_us);
    line[len] = b'\n';
    len += 1;
    say(&line[..len]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn push_dec(buf: &mut [u8], mut value: u32) -> usize {
    let mut tmp = [0u8; 10];
    let mut index = tmp.len();
    loop {
        index -= 1;
        tmp[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let digits = &tmp[index..];
    buf[..digits.len()].copy_from_slice(digits);
    digits.len()
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn cmd_dmesg(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"dmesg err\n");
        return;
    }
    // Printing through `say` would append the dump to the log it is printing.
    // SAFETY: the worker is the only user of this log.
    let log = unsafe { &*LOG.0.get() };
    emit(&log.bytes[..log.len]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn cmd_reboot(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"reboot err\n");
        return;
    }
    say(b"reboot\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn store_serial(bytes: &[u8]) {
    // The boot line has to stay readable, so a full log stops instead of dropping the front.
    // SAFETY: the worker is the only user of this log.
    let log = unsafe { &mut *LOG.0.get() };
    for &byte in bytes {
        if log.len == LOG_CAP {
            break;
        }
        log.bytes[log.len] = byte;
        log.len += 1;
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn schedule() -> &'static sched_config_t {
    // SAFETY: `install_status` wrote this cell before the worker started.
    unsafe { (*SCHED.0.get()).assume_init_ref() }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_fetch(rest: &[u8]) {
    let Some(name) = one_name(rest) else {
        say(b"fetch err\n");
        return;
    };
    let mut body = [0u8; 512];
    let Ok(n) = net::fetch_body(&mut body) else {
        say(b"fetch err\n");
        return;
    };
    if n == 0 {
        say(b"fetch err\n");
        return;
    }
    let fd = match files().open(name, Open::CreateWrite) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"fetch err\n");
            return;
        }
    };
    let wrote = files().write(fd, &body[..n]);
    let _ = files().close(fd);
    let ok = matches!(wrote, Ok(written) if written == n);
    reply(ok, b"fetch ok\n", b"fetch err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_calc(rest: &[u8]) {
    if rest.is_empty() {
        say(b"calc err\n");
        return;
    }
    match eval_expr(rest) {
        Ok(value) => say_calc(value),
        Err(()) => say(b"calc err\n"),
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn say_calc(value: i64) {
    let mut line = [0u8; 32];
    let prefix = b"calc ";
    line[..prefix.len()].copy_from_slice(prefix);
    let mut len = prefix.len();
    len += write_i64(&mut line[len..], value);
    line[len] = b'\n';
    len += 1;
    say(&line[..len]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn write_i64(buf: &mut [u8], value: i64) -> usize {
    if value < 0 {
        buf[0] = b'-';
        return 1 + write_u64(&mut buf[1..], value.unsigned_abs());
    }
    write_u64(buf, u64::try_from(value).expect("non-negative"))
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn write_u64(buf: &mut [u8], mut value: u64) -> usize {
    let mut tmp = [0u8; 20];
    let mut index = tmp.len();
    loop {
        index -= 1;
        tmp[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let digits = &tmp[index..];
    buf[..digits.len()].copy_from_slice(digits);
    digits.len()
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn eval_expr(input: &[u8]) -> Result<i64, ()> {
    let mut parser = Parser { s: input, i: 0 };
    let value = parser.parse_expr()?;
    parser.skip();
    if parser.i != parser.s.len() {
        return Err(());
    }
    Ok(value)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
impl Parser<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && self.s[self.i] == b' ' {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.s.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.i += 1;
        Some(byte)
    }

    fn parse_expr(&mut self) -> Result<i64, ()> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.bump();
                    value = value.saturating_add(self.parse_term()?);
                }
                Some(b'-') => {
                    self.bump();
                    value = value.saturating_sub(self.parse_term()?);
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<i64, ()> {
        let mut value = self.parse_factor()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.bump();
                    value = value.saturating_mul(self.parse_factor()?);
                }
                Some(b'/') => {
                    self.bump();
                    let right = self.parse_factor()?;
                    if right == 0 {
                        return Err(());
                    }
                    value /= right;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<i64, ()> {
        match self.peek() {
            Some(b'(') => {
                self.bump();
                let value = self.parse_expr()?;
                if self.bump() != Some(b')') {
                    return Err(());
                }
                Ok(value)
            }
            Some(b'+') => {
                self.bump();
                self.parse_factor()
            }
            Some(b'-') => {
                self.bump();
                Ok(self.parse_factor()?.saturating_neg())
            }
            Some(b'0'..=b'9') => self.parse_number(),
            _ => Err(()),
        }
    }

    fn parse_number(&mut self) -> Result<i64, ()> {
        let mut value: i64 = 0;
        let mut saw = false;
        while let Some(byte @ b'0'..=b'9') = self.s.get(self.i).copied() {
            saw = true;
            self.i += 1;
            value = value
                .saturating_mul(10)
                .saturating_add(i64::from(byte - b'0'));
        }
        if saw {
            Ok(value)
        } else {
            Err(())
        }
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_backup(rest: &[u8]) {
    let Some((src, dest)) = split_first(rest) else {
        say(b"backup err\n");
        return;
    };
    let Some(dest) = one_name(dest) else {
        say(b"backup err\n");
        return;
    };
    if src.is_empty() {
        say(b"backup err\n");
        return;
    }
    let fd = match files().open(src, Open::Read) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"backup err\n");
            return;
        }
    };
    let mut buf = [0u8; 512];
    let read = files().read(fd, &mut buf);
    let _ = files().close(fd);
    let Ok(n) = read else {
        say(b"backup err\n");
        return;
    };
    if n == 0 {
        say(b"backup err\n");
        return;
    }
    let fd = match files().open(dest, Open::CreateWrite) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"backup err\n");
            return;
        }
    };
    let wrote = files().write(fd, &buf[..n]);
    let _ = files().close(fd);
    let ok = matches!(wrote, Ok(written) if written == n);
    reply(ok, b"backup ok\n", b"backup err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_source(rest: &[u8]) {
    use core::sync::atomic::{AtomicBool, Ordering};
    static SOURCING: AtomicBool = AtomicBool::new(false);
    let Some(name) = one_name(rest) else {
        say(b"source err\n");
        return;
    };
    if SOURCING.swap(true, Ordering::Relaxed) {
        say(b"source err\n");
        return;
    }
    let ran = source_file(name);
    SOURCING.store(false, Ordering::Relaxed);
    if !ran {
        say(b"source err\n");
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn source_file(name: &[u8]) -> bool {
    let fd = match files().open(name, Open::Read) {
        Ok(fd) => fd,
        Err(_) => return false,
    };
    let mut buf = [0u8; 512];
    let read = files().read(fd, &mut buf);
    let _ = files().close(fd);
    let Ok(n) = read else {
        return false;
    };
    if n == 0 {
        return false;
    }
    let mut ran = false;
    let mut start = 0;
    for index in 0..=n {
        let at_end = index == n || buf[index] == b'\n';
        if !at_end {
            continue;
        }
        let raw = &buf[start..index];
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        start = index + 1;
        if raw.is_empty() {
            continue;
        }
        if raw.len() > LINE_MAX {
            return false;
        }
        let mut line = [0u8; LINE_MAX];
        line[..raw.len()].copy_from_slice(raw);
        dispatch(&line[..raw.len()]);
        ran = true;
    }
    ran
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_cert(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"cert err\n");
        return;
    }
    reply(net::cert_ready(), b"cert ok\n", b"cert err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_edit(rest: &[u8]) {
    let Some((name, text)) = split_first(rest) else {
        say(b"edit err\n");
        return;
    };
    if name.is_empty() || text.is_empty() || text.len() > 512 {
        say(b"edit err\n");
        return;
    }
    let fd = match files().open(name, Open::CreateWrite) {
        Ok(fd) => fd,
        Err(_) => {
            say(b"edit err\n");
            return;
        }
    };
    let wrote = files().write(fd, text);
    let _ = files().close(fd);
    let ok = matches!(wrote, Ok(n) if n == text.len());
    reply(ok, b"edit ok\n", b"edit err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_chat(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"chat err\n");
        return;
    }
    let mut buf = [0u8; 16];
    let Ok(n) = net::chat_echo(&mut buf) else {
        say(b"chat err\n");
        return;
    };
    let echoed = n >= 4 && buf[..n].windows(4).any(|window| window == b"chat");
    reply(echoed, b"chat ok\n", b"chat err\n");
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

#[cfg_attr(feature = "net-client", protection_domain(heap_size = 1024 * 1024))]
#[cfg_attr(not(feature = "net-client"), protection_domain)]
fn init() -> HandlerImpl {
    let serial_len = size_of::<serial_client_config_t>();
    let fs_len = size_of::<fs_client_config_t>();
    #[cfg(not(any(
        feature = "net-client",
        any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        )
    )))]
    assert_eq!(CONFIG.len(), serial_len + fs_len);
    // SAFETY: the build script wrote a `serial_client_config_t` and then an `fs_client_config_t`.
    let serial = unsafe {
        serial_image::serial_config_from_bytes::<serial_client_config_t>(&CONFIG[..serial_len])
    };
    let fs = unsafe {
        fs_image::fs_config_from_bytes::<fs_client_config_t>(
            &CONFIG[serial_len..serial_len + fs_len],
        )
    };
    assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(fs.magic, LIONS_FS_MAGIC);
    assert_eq!(serial.tx.id, CLIENT_TX_CHANNEL);
    assert_eq!(serial.rx.id, CLIENT_RX_CHANNEL);
    assert_eq!(fs.server.id, FS_SHELL_SERVER_CHANNEL);
    #[cfg(feature = "net-client")]
    install_net(serial_len + fs_len);
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))]
    gpu::install();
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    ))]
    install_status(serial_len + fs_len);
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
        #[cfg(feature = "net-client")]
        net::on_notify(channels);
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_browser_sddf",
            feature = "board-qemu_virt_riscv64_browser_sddf"
        ))]
        gpu::on_notify(channels);
        Ok(())
    }
}

#[cfg(feature = "net-client")]
fn install_net(at: usize) {
    use lerux_sddf::{
        net_client_config_t,
        net_image::{self, SHELL_NET_RX_CHANNEL, SHELL_NET_TX_CHANNEL},
        SDDF_NET_MAGIC,
    };

    let net_len = size_of::<net_client_config_t>();
    assert_eq!(CONFIG.len(), at + net_len);
    // SAFETY: the build script wrote the network client page after the filesystem page.
    let net = unsafe { net_image::net_config_from_bytes::<net_client_config_t>(&CONFIG[at..]) };
    assert_eq!(net.magic, SDDF_NET_MAGIC);
    assert_eq!(net.rx.id, SHELL_NET_RX_CHANNEL);
    assert_eq!(net.tx.id, SHELL_NET_TX_CHANNEL);
    net::install(&net);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn install_status(at: usize) {
    let timer_len = size_of::<timer_client_config_t>();
    let schedule_len = size_of::<sched_config_t>();
    assert_eq!(CONFIG.len(), at + timer_len + schedule_len);
    // SAFETY: the build script wrote the timer client page and then the schedule page.
    let timer =
        unsafe { timer_config_from_bytes::<timer_client_config_t>(&CONFIG[at..at + timer_len]) };
    assert_eq!(timer.magic, SDDF_TIMER_MAGIC);
    assert_eq!(timer.driver_id, TIMER_CLIENT_CHANNEL);
    let page = unsafe { sched_from_bytes(&CONFIG[at + timer_len..]) };
    assert_eq!(usize::from(page.count), FS_IMAGE_ROLES.len());
    // SAFETY: init is the only writer, and the worker has not started.
    unsafe { (*SCHED.0.get()).write(page) };
}
