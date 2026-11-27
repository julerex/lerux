#![no_std]
#![no_main]
#![cfg_attr(
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-pc_z97_d3h_i217"
    ),
    expect(dead_code, reason = "this image does not run the file-command loop")
)]
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

#[cfg(all(
    feature = "board-pc_z97_d3h_i217",
    any(
        feature = "board-swift_sfx14_41g",
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf",
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf",
    )
))]
compile_error!("the I217 shell does not share a build with Swift, fetch, or the filesystem image");

#[cfg(all(
    any(feature = "board-swift_sfx14_41g", feature = "console-out"),
    any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_aarch64_web_sddf",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_web_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_web_sddf",
        feature = "board-x86_64_generic_fs_sddf",
    )
))]
compile_error!(
    "the Swift console image does not share a build with the fetch, web, browser, or filesystem image"
);

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
use core::sync::atomic::AtomicU64;
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
use core::sync::atomic::{AtomicBool, Ordering};
use core::{
    cell::UnsafeCell,
    mem::{size_of, MaybeUninit},
};

#[cfg(feature = "console-out")]
use embedded_hal_nb::{
    nb,
    serial::{Read, Write},
};
#[cfg(feature = "board-swift_sfx14_41g")]
use sel4_microkit::memory_region_symbol;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};
#[cfg(feature = "console-out")]
use sel4_microkit_driver_adapters::serial::client::Client;

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
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
use lerux_posix::Errno;
use lerux_posix::{Fd, Files, Open, Stat};
use lerux_sddf::{
    fs_client_config_t,
    fs_image::{
        self, FS_COMMAND_QUEUE_VADDR, FS_COMPLETION_QUEUE_VADDR, FS_REGION_SIZE, FS_SHARE_VADDR,
        FS_SHELL_SERVER_CHANNEL,
    },
    fs_queue_t, serial_client_config_t,
    serial_image::{self, CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL},
    LIONS_FS_MAGIC, SDDF_SERIAL_MAGIC,
};

#[cfg(not(feature = "console-out"))]
use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_enqueue, serial_handle_from_connection,
    serial_queue_handle_t, serial_request_consumer_signal, serial_require_consumer_signal,
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

#[cfg(feature = "board-swift_sfx14_41g")]
use lerux_signon::for_each_preamble;
#[cfg(feature = "board-swift_sfx14_41g")]
use lerux_swift::{
    format_survey_report, is_setup_wifi, join_up, read_survey_count, survey_choice, write_key,
    write_link_name, write_survey, SURVEY_REPORT_MAX,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
#[cfg(not(feature = "console-out"))]
const TX_CH: usize = CLIENT_TX_CHANNEL as usize;
#[cfg(not(feature = "console-out"))]
const RX_CH: usize = CLIENT_RX_CHANNEL as usize;
const FS_CH: usize = FS_SHELL_SERVER_CHANNEL as usize;
/// Filesystem server. The literal matches `FS_SHELL_SERVER_CHANNEL`.
const FATFS: Channel = Channel::new(2);
const _: () = assert!(FS_SHELL_SERVER_CHANNEL == 2);
/// Console byte channel. `console_driver` aliases `serial_driver` in the channel check.
#[cfg(feature = "console-out")]
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Receive copier. The I217 shell has no filesystem channel, so this is 1.
#[cfg(all(feature = "net-client", feature = "board-pc_z97_d3h_i217"))]
const NET_COPY: Channel = Channel::new(1);
/// Transmit virtualiser on the I217 shell.
#[cfg(all(feature = "net-client", feature = "board-pc_z97_d3h_i217"))]
const NET_VIRT_TX: Channel = Channel::new(2);
/// Receive copier. The literal matches `SHELL_NET_RX_CHANNEL`.
#[cfg(all(feature = "net-client", not(feature = "board-pc_z97_d3h_i217")))]
const NET_COPY: Channel = Channel::new(3);
/// Transmit virtualiser. The literal matches `SHELL_NET_TX_CHANNEL`.
#[cfg(all(feature = "net-client", not(feature = "board-pc_z97_d3h_i217")))]
const NET_VIRT_TX: Channel = Channel::new(4);
/// Survey notify. The radio's channel 3 runs the same handler as a passphrase.
#[cfg(feature = "board-swift_sfx14_41g")]
const SWIFT_MT7921: Channel = Channel::new(5);

/// Drain the radio's receive ring. The ring is 8 deep, and DHCP waits here
/// without a transmit for longer than that. A joined key does not associate again.
#[cfg(feature = "board-swift_sfx14_41g")]
pub(crate) fn kick_radio() {
    SWIFT_MT7921.notify();
}

#[cfg(all(feature = "net-client", not(feature = "board-pc_z97_d3h_i217")))]
const _: () = {
    assert!(lerux_sddf::net_image::SHELL_NET_RX_CHANNEL == 3);
    assert!(lerux_sddf::net_image::SHELL_NET_TX_CHANNEL == 4);
};
#[cfg(feature = "board-pc_z97_d3h_i217")]
const _: () = {
    assert!(lerux_sddf::net_image::I217_SHELL_NET_RX_CHANNEL == 1);
    assert!(lerux_sddf::net_image::I217_SHELL_NET_TX_CHANNEL == 2);
};
#[cfg(feature = "net-client")]
const STACK_SIZE: usize = 0x1_0000;
#[cfg(not(feature = "net-client"))]
const STACK_SIZE: usize = 0x4000;
const LINE_MAX: usize = 80;
const NAME_MAX: usize = 12;
const LIST_MAX: usize = 32;
const S_IFMT: u64 = 0o170_000;
const S_IFDIR: u64 = 0o040_000;
/// Sectors in the volume `sddf-fatfs` formats. Data starts at sector 97.
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const VOLUME_SECTORS: u64 = 8192;
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const VOLUME_DATA_LBA: u64 = 97;
/// One cluster is one sector on that volume. The root directory is not in it.
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const VOLUME_SECTOR: u64 = 512;

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
mod gpu;
#[cfg(feature = "net-client")]
mod net;
#[cfg(feature = "net-client")]
mod net_device;
mod rescue;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

#[cfg(not(feature = "console-out"))]
struct Io {
    tx: serial_queue_handle_t,
    rx: serial_queue_handle_t,
}

#[cfg(not(feature = "console-out"))]
struct IoCell(UnsafeCell<MaybeUninit<Io>>);

// SAFETY: init writes this cell before the worker starts. The root does not read it again.
#[cfg(not(feature = "console-out"))]
unsafe impl Sync for IoCell {}

#[cfg(not(feature = "console-out"))]
static IO: IoCell = IoCell(UnsafeCell::new(MaybeUninit::uninit()));

#[cfg(feature = "console-out")]
struct ConsoleCell(UnsafeCell<MaybeUninit<Client>>);

// SAFETY: init writes this cell before the worker starts. One kernel thread uses it.
#[cfg(feature = "console-out")]
unsafe impl Sync for ConsoleCell {}

#[cfg(feature = "console-out")]
static CONSOLE: ConsoleCell = ConsoleCell(UnsafeCell::new(MaybeUninit::uninit()));

struct FilesCell(UnsafeCell<MaybeUninit<Files>>);

// SAFETY: the worker is the only user. The table is several kilobytes, so it stays off the stack.
unsafe impl Sync for FilesCell {}

static FILES: FilesCell = FilesCell(UnsafeCell::new(MaybeUninit::uninit()));

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const LOG_CAP: usize = 2048;

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
struct Log {
    bytes: [u8; LOG_CAP],
    len: usize,
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
struct LogCell(UnsafeCell<Log>);

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
// SAFETY: the worker is the only user. The root does not read the log.
unsafe impl Sync for LogCell {}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
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

#[cfg(feature = "board-pc_z97_d3h_i217")]
extern "C" fn worker() {
    i217_worker();
}

#[cfg(not(feature = "board-pc_z97_d3h_i217"))]
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
    app_menu();
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    {
        #[cfg(any(
            feature = "board-qemu_virt_aarch64_fs_sddf",
            feature = "board-qemu_virt_riscv64_fs_sddf",
            feature = "board-x86_64_generic_fs_sddf"
        ))]
        note_boot();
        #[cfg(feature = "board-swift_sfx14_41g")]
        say_preamble();
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        say(b"lerux shell ready\n");
        let mut line = [0u8; LINE_MAX];
        loop {
            let len = read_line(&mut line);
            dispatch(&line[..len]);
            #[cfg(feature = "board-swift_sfx14_41g")]
            say(lerux_signon::PROMPT);
        }
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn say_preamble() {
    for_each_preamble(env!("LERUX_GIT_DESCRIBE").as_bytes(), say);
}

#[cfg(feature = "board-pc_z97_d3h_i217")]
fn i217_worker() {
    use lerux_signon::{for_each_i217_preamble, PROMPT};
    for_each_i217_preamble(env!("LERUX_GIT_DESCRIBE").as_bytes(), say);
    let mut line = [0u8; LINE_MAX];
    loop {
        let len = read_line(&mut line);
        dispatch_i217(&line[..len]);
        say(PROMPT);
    }
}

#[cfg(feature = "board-pc_z97_d3h_i217")]
fn dispatch_i217(line: &[u8]) {
    let line = trim(line);
    if line.is_empty() {
        return;
    }
    let Some((cmd, rest)) = split_first(line) else {
        say(b"err\n");
        return;
    };
    if cmd == b"ping" && rest.is_empty() {
        say_ping(net::ping_gateway());
        return;
    }
    if cmd == b"ip" {
        cmd_ip(rest);
        return;
    }
    if cmd == b"date" {
        cmd_date(rest);
        return;
    }
    if cmd == b"dmesg" {
        cmd_dmesg(rest);
        return;
    }
    if rescue::command(cmd, rest) {
        return;
    }
    say(b"err\n");
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
            usize::try_from(FS_REGION_SIZE).expect("fs region"),
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
    FATFS.notify();
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
    let html = if let Ok(n) = net::fetch_body(&mut body) {
        match page_title(&body[..n]) {
            Some(title) => say_title(title),
            None => say(b"title err\n"),
        }
        &body[..n]
    } else {
        say(b"title err\n");
        &[][..]
    };
    match gpu::show_page(html) {
        Ok(line) => say(line.as_bytes()),
        Err(()) => say(b"pixel err\n"),
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn browse_name() {
    let mut body = [0u8; 512];
    let Ok((addr, n)) = net::fetch_named(&mut body) else {
        say(b"dns err\n");
        return;
    };
    match gpu::show_page(titled(&body[..n])) {
        Ok(line) => say_painted(line.as_bytes(), addr),
        Err(()) => say(b"pixel err\n"),
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn titled(body: &[u8]) -> &[u8] {
    if let Some(title) = page_title(body) {
        say_title(title);
    } else {
        say(b"title err\n");
    }
    body
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn say_painted(pixel: &[u8], addr: [u8; 4]) {
    let mut line = [0u8; 40];
    let n = copy_into(&mut line, 0, pixel);
    let n = write_dns(&mut line, n, addr);
    say(&line[..n]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn write_dns(buf: &mut [u8], mut at: usize, addr: [u8; 4]) -> usize {
    at = copy_into(buf, at, b"dns ");
    for (i, octet) in addr.iter().enumerate() {
        if i > 0 {
            at = copy_into(buf, at, b".");
        }
        at = write_dec(buf, at, *octet);
    }
    copy_into(buf, at, b"\n")
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn copy_into(buf: &mut [u8], at: usize, bytes: &[u8]) -> usize {
    let n = bytes.len().min(buf.len().saturating_sub(at));
    buf[at..at + n].copy_from_slice(&bytes[..n]);
    at + n
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn write_dec(buf: &mut [u8], mut at: usize, mut value: u8) -> usize {
    let mut tmp = [0u8; 3];
    let mut digits = 0usize;
    loop {
        tmp[digits] = b'0' + (value % 10);
        digits += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while digits > 0 {
        digits -= 1;
        let byte = [tmp[digits]];
        at = copy_into(buf, at, &byte);
    }
    at
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
const MENU: &[u8] = b"1 browser\n2 maths\n3 name\n";

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn app_menu() {
    let mut line = [0u8; LINE_MAX];
    loop {
        say(MENU);
        let len = read_line(&mut line);
        match trim(&line[..len]) {
            b"1" | b"browser" => browse(),
            b"2" | b"maths" => maths_repl(),
            b"3" | b"name" => browse_name(),
            _ => say(b"err\n"),
        }
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
))]
fn maths_repl() {
    let mut line = [0u8; LINE_MAX];
    loop {
        say(b"maths> ");
        let len = read_line(&mut line);
        let text = trim(&line[..len]);
        if text == b"quit" {
            return;
        }
        match lerux_calc::eval(text) {
            Ok(value) => say_value(b"maths ", value),
            Err(_) => say(b"maths err\n"),
        }
    }
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

#[cfg(not(feature = "console-out"))]
fn io() -> &'static Io {
    // SAFETY: init wrote this cell before spawning the worker.
    unsafe { (*IO.0.get()).assume_init_ref() }
}

#[cfg(feature = "console-out")]
fn console() -> &'static mut Client {
    // SAFETY: init wrote this cell before spawning the worker. The worker is the user.
    unsafe { (*CONSOLE.0.get()).assume_init_mut() }
}

pub(crate) fn say(bytes: &[u8]) {
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    store_serial(bytes);
    emit(bytes);
}

#[cfg(not(feature = "console-out"))]
pub(crate) fn emit(bytes: &[u8]) {
    let tx = &io().tx;
    for &byte in bytes {
        // SAFETY: `tx` is this client's mapped transmit queue. This worker produces it.
        while unsafe { serial_enqueue(tx, byte) } != 0 {
            unsafe { serial_request_consumer_signal(tx) };
            // This notify runs the virtualiser before the wait, so the wake is stored.
            Channel::new(TX_CH).notify();
            microkit_cothread_wait_on_channel(TX_CH);
        }
    }
    Channel::new(TX_CH).notify();
}

#[cfg(feature = "console-out")]
pub(crate) fn emit(bytes: &[u8]) {
    let screen = console();
    for &byte in bytes {
        loop {
            match screen.write(byte) {
                Ok(()) => break,
                Err(nb::Error::WouldBlock) => core::hint::spin_loop(),
                Err(nb::Error::Other(_)) => return,
            }
        }
    }
}

#[cfg(not(feature = "console-out"))]
pub(crate) fn read_byte() -> u8 {
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

#[cfg(feature = "console-out")]
pub(crate) fn read_byte() -> u8 {
    loop {
        match console().read() {
            Ok(byte) => return byte,
            Err(nb::Error::WouldBlock | nb::Error::Other(_)) => {
                microkit_cothread_wait_on_channel(SERIAL_DRIVER.index());
            }
        }
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
            #[cfg(feature = "console-out")]
            emit(b"\n");
            return if overflow { 0 } else { len };
        }
        #[cfg(feature = "console-out")]
        if byte == 0x7f {
            if len > 0 && !overflow {
                len -= 1;
                emit(&[0x08, b' ', 0x08]);
            }
            continue;
        }
        if len == buf.len() {
            overflow = true;
            continue;
        }
        buf[len] = byte;
        len += 1;
        #[cfg(feature = "console-out")]
        emit(&[byte]);
    }
}

fn dispatch(line: &[u8]) {
    let Some((cmd, rest)) = split_first(line) else {
        say(b"err empty\n");
        return;
    };
    if dispatch_file(cmd, rest) {
        return;
    }
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    if dispatch_daily(cmd, rest) {
        return;
    }
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ))]
    if dispatch_clock(cmd, rest) {
        return;
    }
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    ))]
    if dispatch_fs_board(cmd, rest) {
        return;
    }
    #[cfg(feature = "board-swift_sfx14_41g")]
    if dispatch_swift(cmd, rest) {
        return;
    }
    if rescue::command(cmd, rest) {
        return;
    }
    say_named_err(cmd, b"unknown");
}

/// `name err reason`, so a failed line names the command and why it failed.
fn say_named_err(cmd: &[u8], reason: &[u8]) {
    let mut out = [0u8; 96];
    let mut n = 0;
    for &byte in cmd {
        if n + 1 >= 64 {
            break;
        }
        out[n] = if byte.is_ascii_graphic() { byte } else { b'?' };
        n += 1;
    }
    for &byte in b" err " {
        out[n] = byte;
        n += 1;
    }
    for &byte in reason {
        if n + 1 >= out.len() {
            break;
        }
        out[n] = byte;
        n += 1;
    }
    out[n] = b'\n';
    say(&out[..=n]);
}

fn dispatch_file(cmd: &[u8], rest: &[u8]) -> bool {
    match cmd {
        b"mkdir" => cmd_mkdir(rest),
        b"write" => cmd_write(rest),
        b"cat" => cmd_cat(rest),
        b"ls" => cmd_ls(rest),
        b"mv" => cmd_mv(rest),
        b"rm" => cmd_rm(rest),
        b"stat" => cmd_stat(rest),
        _ => return false,
    }
    true
}

#[cfg(feature = "board-swift_sfx14_41g")]
mod iwctl_cmd;

#[cfg(feature = "board-swift_sfx14_41g")]
fn dispatch_swift(cmd: &[u8], rest: &[u8]) -> bool {
    if is_setup_wifi(cmd) {
        cmd_setup_wifi(rest);
        return true;
    }
    if lerux_iwctl::is_iwctl(cmd) {
        iwctl_cmd::run(rest);
        return true;
    }
    false
}

/// Ask the radio which names it can hear, then join the chosen row and ping.
///
/// The radio is priority 4 and this shell is priority 1, so the survey and
/// the association each finish before `notify` returns. A stored name is
/// `N name`, then `ssid>`. The row number selects that name. `psk>` takes
/// the passphrase. `psk ok` is a finished handshake. The address line and
/// `ping ok` follow. Zero names are the survey mark, with no prompt. A survey
/// the radio did not answer, a rejected row or passphrase, or a handshake
/// that did not finish is `wifi err`. Extra words and a page the shell cannot
/// mark are `setup-wifi err` plus the reason.
#[cfg(feature = "board-swift_sfx14_41g")]
fn cmd_setup_wifi(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"setup-wifi err args\n");
        return;
    }
    if !posted_survey() {
        return;
    }
    if read_survey_count(key_page()).unwrap_or(0) == 0 {
        return;
    }
    join_chosen();
}

/// The survey replaces the key page. `false` means the shell already printed the error.
#[cfg(feature = "board-swift_sfx14_41g")]
fn posted_survey() -> bool {
    if !write_survey(key_page()) {
        say(b"setup-wifi err page\n");
        return false;
    }
    SWIFT_MT7921.notify();
    discard_pending();
    let mut out = [0u8; SURVEY_REPORT_MAX];
    let Some(n) = format_survey_report(key_page(), &mut out) else {
        say(b"wifi err\n");
        return false;
    };
    say(&out[..n]);
    true
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn join_chosen() {
    say(b"ssid> ");
    let mut line = [0u8; LINE_MAX];
    let len = read_line(&mut line);
    if trim(&line[..len]) == b"quit" {
        return;
    }
    let Some((name, name_len)) = chosen_ssid(trim(&line[..len])) else {
        say(b"wifi err\n");
        return;
    };
    say(b"psk> ");
    let len = read_line(&mut line);
    let psk = trim(&line[..len]);
    if psk == b"quit" {
        return;
    }
    if !write_key(key_page(), &name[..name_len], psk) {
        say(b"wifi err\n");
        return;
    }
    SWIFT_MT7921.notify();
    if !join_up(key_page()) {
        say(b"wifi err\n");
        return;
    }
    // The name lives past the survey region. A later `iwctl` scan zeros the key.
    let _ = write_link_name(key_page(), &name[..name_len]);
    say(b"psk ok\n");
    say_ping(net::ping_gateway());
}

/// Copy the chosen name out of the survey page before the key write clears it.
#[cfg(feature = "board-swift_sfx14_41g")]
fn chosen_ssid(line: &[u8]) -> Option<([u8; 32], usize)> {
    let name = survey_choice(key_page(), line)?;
    let mut owned = [0u8; 32];
    owned[..name.len()].copy_from_slice(name);
    Some((owned, name.len()))
}

#[cfg(any(feature = "board-swift_sfx14_41g", feature = "board-pc_z97_d3h_i217"))]
fn say_ping(result: net::PingResult) {
    match result {
        net::PingResult::NoAddress => say(b"ip err\n"),
        net::PingResult::Up(addr) => {
            say_ip(addr);
            say(b"ping ok\n");
        }
        net::PingResult::NoReply(addr) => {
            say_ip(addr);
            say(b"ping err\n");
        }
    }
}

#[cfg(all(
    feature = "net-client",
    not(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))
))]
fn say_ip(octets: [u8; 4]) {
    let mut out = [0u8; 20];
    let mut n = 0;
    for &byte in b"ip " {
        out[n] = byte;
        n += 1;
    }
    for (index, octet) in octets.into_iter().enumerate() {
        if index > 0 {
            out[n] = b'.';
            n += 1;
        }
        n = write_octet(&mut out, n, octet);
    }
    out[n] = b'\n';
    say(&out[..=n]);
}

#[cfg(all(
    feature = "net-client",
    not(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))
))]
fn write_octet(buf: &mut [u8], mut at: usize, mut value: u8) -> usize {
    let mut tmp = [0u8; 3];
    let mut digits = 0usize;
    loop {
        tmp[digits] = b'0' + (value % 10);
        digits += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while digits > 0 {
        digits -= 1;
        buf[at] = tmp[digits];
        at += 1;
    }
    at
}

/// Drop keys queued while the radio ran the survey.
#[cfg(feature = "board-swift_sfx14_41g")]
pub(crate) fn discard_pending() {
    loop {
        match console().read() {
            Ok(_) => {}
            Err(nb::Error::WouldBlock | nb::Error::Other(_)) => return,
        }
    }
}

/// The template maps this page read-write at `0x1fd0000`. The radio is the other writer.
#[cfg(feature = "board-swift_sfx14_41g")]
pub(crate) fn key_page() -> &'static mut [u8] {
    let ptr = memory_region_symbol!(wifi_key_vaddr: *mut u8).as_ptr();
    // SAFETY: this domain maps `wifi_key` for its whole life. The slice is not held across `notify`.
    unsafe { core::slice::from_raw_parts_mut(ptr, 0x1000) }
}

/// File, address, clock, log, and edit commands for the filesystem image and the fetch image.
///
/// A command that needs a socket prints its name and `err` when this image has no network client.
/// The browser image is a menu and does not compile this table.
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn dispatch_daily(cmd: &[u8], rest: &[u8]) -> bool {
    match cmd {
        b"df" => cmd_df(rest),
        b"ip" => cmd_ip(rest),
        b"date" => cmd_date(rest),
        b"dmesg" => cmd_dmesg(rest),
        b"edit" => cmd_edit(rest),
        b"reboot" => cmd_reboot(rest),
        b"calc" => cmd_calc(rest),
        b"backup" => cmd_backup(rest),
        b"source" => cmd_source(rest),
        b"fetch" => cmd_fetch_shared(rest),
        b"cert" => cmd_cert_shared(rest),
        b"chat" => cmd_chat_shared(rest),
        _ => return false,
    }
    true
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_fetch_shared(rest: &[u8]) {
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ))]
    cmd_fetch(rest);
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    )))]
    {
        let _ = rest;
        say(b"fetch err\n");
    }
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_cert_shared(rest: &[u8]) {
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ))]
    cmd_cert(rest);
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    )))]
    {
        let _ = rest;
        say(b"cert err\n");
    }
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_chat_shared(rest: &[u8]) {
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    ))]
    cmd_chat(rest);
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fetch_sddf",
        feature = "board-qemu_virt_riscv64_fetch_sddf",
        feature = "board-x86_64_generic_fetch_sddf"
    )))]
    {
        let _ = rest;
        say(b"chat err\n");
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn dispatch_clock(cmd: &[u8], rest: &[u8]) -> bool {
    match cmd {
        b"config" => cmd_config(rest),
        b"date" => cmd_date(rest),
        b"dmesg" => cmd_dmesg(rest),
        _ => return false,
    }
    true
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn dispatch_fs_board(cmd: &[u8], rest: &[u8]) -> bool {
    match cmd {
        b"config" => cmd_config(rest),
        b"ps" => cmd_table(rest, b"ps ok\n", b"ps err\n"),
        b"qos" => cmd_table(rest, b"qos ok\n", b"qos err\n"),
        _ => return false,
    }
    true
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
const HOSTNAME_PATH: &[u8] = b"/config/hostname";
/// `net.mode` is not an 8.3 name. The bytes sit beside `HOSTNAME`.
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
const NETMODE_PATH: &[u8] = b"/config/netmode";
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
const HOSTNAME_MAX: usize = 64;

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn cmd_config(rest: &[u8]) {
    let Some((verb, tail)) = split_first(rest) else {
        say(b"config err\n");
        return;
    };
    match verb {
        b"get" => {
            let Some(key) = one_name(tail) else {
                say(b"config err\n");
                return;
            };
            config_get(key);
        }
        b"set" => {
            let Some((key, value)) = split_first(tail) else {
                say(b"config err\n");
                return;
            };
            config_set(key, value);
        }
        _ => say(b"config err\n"),
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn config_get(key: &[u8]) {
    if key == b"hostname" {
        config_get_hostname();
        return;
    }
    if key == b"net.mode" {
        config_get_mode();
        return;
    }
    say(b"config err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn config_get_hostname() {
    let Ok(fd) = files().open(HOSTNAME_PATH, Open::Read) else {
        say(b"config err\n");
        return;
    };
    let mut buf = [0u8; HOSTNAME_MAX];
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
    let mut out = [0u8; HOSTNAME_MAX + 1];
    out[..n].copy_from_slice(&buf[..n]);
    out[n] = b'\n';
    say(&out[..=n]);
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn config_get_mode() {
    // This image has no network client, so the key is refused.
    #[cfg(not(feature = "net-client"))]
    {
        say(b"config err\n");
    }
    #[cfg(feature = "net-client")]
    {
        match read_mode() {
            ModeFile::Missing | ModeFile::Dhcp => say(b"dhcp\n"),
            ModeFile::Static => say(b"static\n"),
            ModeFile::Bad => say(b"config err\n"),
        }
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn config_set(key: &[u8], value: &[u8]) {
    if key == b"hostname" {
        config_set_hostname(value);
        return;
    }
    if key == b"net.mode" {
        config_set_mode(value);
        return;
    }
    say(b"config err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn config_set_hostname(value: &[u8]) {
    let Some(name) = one_name(value) else {
        say(b"config err\n");
        return;
    };
    if !hostname_ok(name) || !replace_file(HOSTNAME_PATH, name) {
        say(b"config err\n");
        return;
    }
    say(b"config ok\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn hostname_ok(name: &[u8]) -> bool {
    !name.is_empty()
        && name.len() <= HOSTNAME_MAX
        && name
            .iter()
            .all(|byte| byte.is_ascii_graphic() && *byte != b'/')
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn config_set_mode(value: &[u8]) {
    #[cfg(not(feature = "net-client"))]
    {
        let _ = value;
        say(b"config err\n");
    }
    #[cfg(feature = "net-client")]
    {
        if value == b"dhcp" {
            if replace_file(NETMODE_PATH, b"dhcp") {
                say(b"config ok\n");
            } else {
                say(b"config err\n");
            }
            return;
        }
        let Some((word, addr)) = split_first(value) else {
            say(b"config err\n");
            return;
        };
        if word != b"static" {
            say(b"config err\n");
            return;
        }
        let Some(addr) = one_name(addr) else {
            say(b"config err\n");
            return;
        };
        let Some(octets) = parse_v4(addr) else {
            say(b"config err\n");
            return;
        };
        if !net::apply_static(octets) {
            say(b"config err\n");
            return;
        }
        let (stored, len) = format_static(octets);
        if replace_file(NETMODE_PATH, &stored[..len]) {
            say(b"config ok\n");
        } else {
            say(b"config err\n");
        }
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf",
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn replace_file(path: &[u8], bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    match files().unlink(path) {
        Ok(()) => {}
        Err(Errno(code)) if code == Errno::ENOENT => {}
        Err(_) => return false,
    }
    let Ok(fd) = files().open(path, Open::CreateWrite) else {
        return false;
    };
    let wrote = files().write(fd, bytes);
    let _ = files().close(fd);
    matches!(wrote, Ok(n) if n == bytes.len())
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
enum ModeFile {
    Missing,
    Dhcp,
    Static,
    Bad,
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn read_mode() -> ModeFile {
    let fd = match files().open(NETMODE_PATH, Open::Read) {
        Ok(fd) => fd,
        Err(Errno(code)) if code == Errno::ENOENT => return ModeFile::Missing,
        Err(_) => return ModeFile::Bad,
    };
    let mut buf = [0u8; 32];
    let read = files().read(fd, &mut buf);
    let _ = files().close(fd);
    let Ok(n) = read else {
        return ModeFile::Bad;
    };
    // A stored mode is `dhcp` or `static` plus one dotted address.
    if n == 0 || n == buf.len() {
        return ModeFile::Bad;
    }
    parse_mode(&buf[..n])
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn parse_mode(bytes: &[u8]) -> ModeFile {
    if bytes == b"dhcp" {
        return ModeFile::Dhcp;
    }
    let Some(addr) = bytes.strip_prefix(b"static ") else {
        return ModeFile::Bad;
    };
    if parse_v4(addr).is_some() {
        ModeFile::Static
    } else {
        ModeFile::Bad
    }
}

/// Four decimal octets. A value above 255, an empty octet, or a trailing dot is refused.
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn parse_v4(text: &[u8]) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut index = 0usize;
    let mut value = 0u16;
    let mut digits = 0u8;
    for &byte in text {
        if byte == b'.' {
            if digits == 0 || index >= 3 {
                return None;
            }
            octets[index] = u8::try_from(value).ok()?;
            index += 1;
            value = 0;
            digits = 0;
            continue;
        }
        if !byte.is_ascii_digit() || digits == 3 {
            return None;
        }
        value = value * 10 + u16::from(byte - b'0');
        if value > 255 {
            return None;
        }
        digits += 1;
    }
    if digits == 0 || index != 3 {
        return None;
    }
    octets[3] = u8::try_from(value).ok()?;
    Some(octets)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn format_static(octets: [u8; 4]) -> ([u8; 22], usize) {
    let mut out = [0u8; 22];
    let prefix = b"static ";
    out[..prefix.len()].copy_from_slice(prefix);
    let len = prefix.len() + write_v4(&mut out[prefix.len()..], octets);
    (out, len)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn write_v4(buf: &mut [u8], octets: [u8; 4]) -> usize {
    let mut at = 0usize;
    for (index, octet) in octets.into_iter().enumerate() {
        if index > 0 {
            buf[at] = b'.';
            at += 1;
        }
        at = write_octet_dec(buf, at, octet);
    }
    at
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
fn write_octet_dec(buf: &mut [u8], mut at: usize, mut value: u8) -> usize {
    let mut tmp = [0u8; 3];
    let mut digits = 0usize;
    loop {
        tmp[digits] = b'0' + (value % 10);
        digits += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while digits > 0 {
        digits -= 1;
        buf[at] = tmp[digits];
        at += 1;
    }
    at
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_date(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"date err\n");
        return;
    }
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    ))]
    date_from_timer();
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    )))]
    say(b"date err\n");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
static BOOT_NS: AtomicU64 = AtomicU64::new(0);

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn note_boot() {
    let reply = Channel::new(usize::from(SHELL_TIMER_CHANNEL))
        .pp_call(MessageInfo::new(SDDF_TIMER_GET_TIME, 0));
    if reply.count() < 1 {
        return;
    }
    let ns = get_mr(0);
    if ns != 0 {
        BOOT_NS.store(ns, Ordering::Relaxed);
    }
}

pub(crate) fn boot_ns() -> Option<u64> {
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    ))]
    {
        let ns = BOOT_NS.load(Ordering::Relaxed);
        return if ns == 0 { None } else { Some(ns) };
    }
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    )))]
    {
        None
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn date_from_timer() {
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

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
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

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_reboot(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"reboot err\n");
        return;
    }
    say(b"reboot\n");
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
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
    let Ok(fd) = files().open(name, Open::CreateWrite) else {
        say(b"fetch err\n");
        return;
    };
    let wrote = files().write(fd, &body[..n]);
    let _ = files().close(fd);
    let ok = matches!(wrote, Ok(written) if written == n);
    reply(ok, b"fetch ok\n", b"fetch err\n");
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_calc(rest: &[u8]) {
    if rest.is_empty() {
        say(b"calc err\n");
        return;
    }
    match lerux_calc::eval(rest) {
        Ok(value) => say_value(b"calc ", value),
        Err(_) => say(b"calc err\n"),
    }
}

fn say_value(prefix: &[u8], value: i64) {
    let mut line = [0u8; 32];
    line[..prefix.len()].copy_from_slice(prefix);
    let mut len = prefix.len();
    len += lerux_calc::write_i64(&mut line[len..], value);
    line[len] = b'\n';
    len += 1;
    say(&line[..len]);
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
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
    let Ok(fd) = files().open(src, Open::Read) else {
        say(b"backup err\n");
        return;
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
    let Ok(fd) = files().open(dest, Open::CreateWrite) else {
        say(b"backup err\n");
        return;
    };
    let wrote = files().write(fd, &buf[..n]);
    let _ = files().close(fd);
    let ok = matches!(wrote, Ok(written) if written == n);
    reply(ok, b"backup ok\n", b"backup err\n");
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_source(rest: &[u8]) {
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

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn source_file(name: &[u8]) -> bool {
    let Ok(fd) = files().open(name, Open::Read) else {
        return false;
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

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_edit(rest: &[u8]) {
    let Some((name, text)) = split_first(rest) else {
        say(b"edit err\n");
        return;
    };
    if name.is_empty() || text.is_empty() || text.len() > 512 {
        say(b"edit err\n");
        return;
    }
    let Ok(fd) = files().open(name, Open::CreateWrite) else {
        say(b"edit err\n");
        return;
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

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_df(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"df err\n");
        return;
    }
    let Ok(root) = files().stat(b"/") else {
        say(b"df err\n");
        return;
    };
    if root.blksize != VOLUME_SECTOR {
        say(b"df err\n");
        return;
    }
    let Some(total) = (VOLUME_SECTORS - VOLUME_DATA_LBA).checked_mul(VOLUME_SECTOR) else {
        say(b"df err\n");
        return;
    };
    // A directory is one cluster. A file occupies one cluster per 512 bytes
    // of its size, and a nested name is stat'd so a file that spans clusters
    // is counted in full.
    let Some(clusters) = used_clusters(b".", true) else {
        say(b"df err\n");
        return;
    };
    let used = clusters.saturating_mul(VOLUME_SECTOR);
    if used > total {
        say(b"df err\n");
        return;
    }
    say_df(total, used);
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn cmd_ip(rest: &[u8]) {
    if !rest.is_empty() {
        say(b"ip err\n");
        return;
    }
    #[cfg(feature = "net-client")]
    match net::local_address() {
        Ok(addr) => say_ip(addr),
        Err(()) => say(b"ip err\n"),
    };
    #[cfg(not(feature = "net-client"))]
    say(b"ip err\n");
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn say_df(total: u64, used: u64) {
    let mut line = [0u8; 48];
    let mut len = 0;
    for &byte in b"df " {
        line[len] = byte;
        len += 1;
    }
    len += push_u64(&mut line[len..], total);
    line[len] = b' ';
    len += 1;
    len += push_u64(&mut line[len..], used);
    line[len] = b'\n';
    say(&line[..=len]);
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn push_u64(buf: &mut [u8], mut value: u64) -> usize {
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

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const DF_PATH: usize = 64;
/// One cursor per path component, including the root. A 64-byte path of
/// one-letter names is 32 components plus the root, which is less than this.
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const DF_CURSORS: usize = DF_PATH;
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
const _: () = {
    assert!(DF_CURSORS > DF_PATH.div_ceil(2));
    assert!(LIST_MAX < 255);
};

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
struct Listed {
    bytes: [[u8; NAME_MAX]; LIST_MAX],
    lens: [u8; LIST_MAX],
    count: usize,
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
#[derive(Clone, Copy)]
struct DfCursor {
    bytes: [u8; DF_PATH],
    len: u8,
    /// Next child index to visit. The parent is re-read from here.
    next: u8,
    root: bool,
}

/// One directory snapshot and a resume cursor per path level. Both stay off
/// the worker stack. The cursor count follows the 64-byte path, not the width
/// of a directory.
#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
struct DfScratch {
    listed: Listed,
    cursors: [DfCursor; DF_CURSORS],
    depth: usize,
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
struct DfScratchCell(UnsafeCell<DfScratch>);

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
// SAFETY: the worker is the only user. The root does not run `df`.
unsafe impl Sync for DfScratchCell {}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
static DF_SCRATCH: DfScratchCell = DfScratchCell(UnsafeCell::new(DfScratch {
    listed: Listed {
        bytes: [[0; NAME_MAX]; LIST_MAX],
        lens: [0; LIST_MAX],
        count: 0,
    },
    cursors: [DfCursor {
        bytes: [0; DF_PATH],
        len: 0,
        next: 0,
        root: false,
    }; DF_CURSORS],
    depth: 0,
}));

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_scratch() -> &'static mut DfScratch {
    // SAFETY: only the worker calls `df`. Each helper drops this borrow
    // before the next `files()` call.
    unsafe { &mut *DF_SCRATCH.0.get() }
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_push(path: &[u8], root: bool) -> Option<()> {
    if path.is_empty() || path.len() > DF_PATH {
        return None;
    }
    let len = u8::try_from(path.len()).unwrap_or(0);
    if len == 0 {
        return None;
    }
    let scratch = df_scratch();
    if scratch.depth == DF_CURSORS {
        return None;
    }
    let slot = &mut scratch.cursors[scratch.depth];
    slot.bytes[..path.len()].copy_from_slice(path);
    slot.len = len;
    slot.next = 0;
    slot.root = root;
    scratch.depth += 1;
    Some(())
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_top(out: &mut [u8]) -> Option<(usize, u8, bool)> {
    let scratch = df_scratch();
    if scratch.depth == 0 {
        return None;
    }
    let slot = &scratch.cursors[scratch.depth - 1];
    let len = usize::from(slot.len);
    if len == 0 || len > out.len() || len > slot.bytes.len() {
        return None;
    }
    out[..len].copy_from_slice(&slot.bytes[..len]);
    Some((len, slot.next, slot.root))
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_set_next(next: u8) -> Option<()> {
    let scratch = df_scratch();
    if scratch.depth == 0 {
        return None;
    }
    scratch.cursors[scratch.depth - 1].next = next;
    Some(())
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_pop() {
    let scratch = df_scratch();
    if scratch.depth > 0 {
        scratch.depth -= 1;
    }
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_child(index: usize, out: &mut [u8]) -> Option<usize> {
    let listed = &df_scratch().listed;
    if index >= listed.count {
        return None;
    }
    let len = usize::from(listed.lens[index]);
    if len == 0 || len > out.len() || len > NAME_MAX {
        return None;
    }
    out[..len].copy_from_slice(&listed.bytes[index][..len]);
    Some(len)
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn used_clusters(path: &[u8], root: bool) -> Option<u64> {
    df_scratch().depth = 0;
    df_push(path, root)?;
    let mut used = 0u64;
    while df_scratch().depth > 0 {
        let mut path_buf = [0u8; DF_PATH];
        let (path_len, next, root) = df_top(&mut path_buf)?;
        let path = &path_buf[..path_len];
        let count = read_names(path)?;
        if next == 0 {
            used += u64::from(!root);
        }
        let resume = usize::from(next);
        if resume > count {
            return None;
        }
        let mut descended = false;
        for index in resume..count {
            let mut child = [0u8; NAME_MAX];
            let child_len = df_child(index, &mut child)?;
            let mut joined = [0u8; DF_PATH];
            let joined_len = join_child(path, root, &child[..child_len], &mut joined)?;
            match files().stat(&joined[..joined_len]) {
                Ok(stat) if is_dir(&stat) => {
                    let next_child = u8::try_from(index + 1).ok()?;
                    df_set_next(next_child)?;
                    df_push(&joined[..joined_len], false)?;
                    descended = true;
                    break;
                }
                Ok(stat) => used += file_clusters(stat.size),
                Err(_) => return None,
            }
        }
        if !descended {
            df_pop();
        }
    }
    Some(used)
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn join_child(path: &[u8], root: bool, child: &[u8], out: &mut [u8]) -> Option<usize> {
    if root {
        if child.len() > out.len() {
            return None;
        }
        out[..child.len()].copy_from_slice(child);
        return Some(child.len());
    }
    let need = path.len().checked_add(1)?.checked_add(child.len())?;
    if need > out.len() {
        return None;
    }
    out[..path.len()].copy_from_slice(path);
    out[path.len()] = b'/';
    out[path.len() + 1..need].copy_from_slice(child);
    Some(need)
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn file_clusters(size: u64) -> u64 {
    size.div_ceil(VOLUME_SECTOR).max(1)
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn read_names(path: &[u8]) -> Option<usize> {
    let dir = files().open_dir(path).ok()?;
    df_scratch().listed = Listed {
        bytes: [[0; NAME_MAX]; LIST_MAX],
        lens: [0; LIST_MAX],
        count: 0,
    };
    let mut buf = [0u8; NAME_MAX];
    let mut failed = false;
    loop {
        match files().read_dir(dir, &mut buf) {
            Ok(None) => break,
            Ok(Some(0)) => {}
            Ok(Some(len)) => {
                if !df_note_name(&buf[..len]) {
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
    let _ = files().close_dir(dir);
    if failed {
        None
    } else {
        Some(df_scratch().listed.count)
    }
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn df_note_name(name: &[u8]) -> bool {
    let listed = &mut df_scratch().listed;
    if listed.count == LIST_MAX || name.is_empty() || name.len() > NAME_MAX {
        return false;
    }
    let len = u8::try_from(name.len()).unwrap_or(0);
    if len == 0 {
        return false;
    }
    listed.bytes[listed.count][..name.len()].copy_from_slice(name);
    listed.lens[listed.count] = len;
    listed.count += 1;
    true
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
    if let Ok(fd) = files().open(name, Open::CreateWrite) {
        finish_write(fd, payload);
        return;
    }
    // A second write of the same name appends. One line is 80 bytes, so a
    // file past 512 bytes is several writes at the current size.
    let Ok(stat) = files().stat(name) else {
        say(b"write err\n");
        return;
    };
    if is_dir(&stat) {
        say(b"write err\n");
        return;
    }
    let Ok(fd) = files().open(name, Open::Write) else {
        say(b"write err\n");
        return;
    };
    if files().seek(fd, stat.size).is_err() {
        let _ = files().close(fd);
        say(b"write err\n");
        return;
    }
    finish_write(fd, payload);
}

fn finish_write(fd: Fd, payload: &[u8]) {
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
    let Ok(fd) = files().open(name, Open::Read) else {
        say(b"cat err\n");
        return;
    };
    let mut buf = [0u8; 512];
    loop {
        match files().read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => say(&buf[..n]),
            Err(_) => {
                let _ = files().close(fd);
                say(b"cat err\n");
                return;
            }
        }
    }
    let _ = files().close(fd);
    say(b"\n");
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
    let Ok(dir) = files().open_dir(path) else {
        say(b"ls err\n");
        return;
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
        len: u8::try_from(bytes.len()).expect("directory name"),
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
    let serial = client_pages(serial_len, fs_len);
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
    #[cfg(not(feature = "console-out"))]
    install_serial(&serial);
    #[cfg(feature = "console-out")]
    {
        let _ = serial;
        install_console();
    }
    spawn_worker();
    HandlerImpl
}

fn client_pages(serial_len: usize, fs_len: usize) -> serial_client_config_t {
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
    serial
}

#[cfg(not(feature = "console-out"))]
fn install_serial(serial: &serial_client_config_t) {
    // SAFETY: the template maps this client's transmit and receive regions.
    let tx = unsafe { serial_handle_from_connection(&serial.tx) };
    let rx = unsafe { serial_handle_from_connection(&serial.rx) };
    unsafe { (*IO.0.get()).write(Io { tx, rx }) };
}

#[cfg(feature = "console-out")]
fn install_console() {
    // SAFETY: init is the only writer, and the worker has not started.
    unsafe { (*CONSOLE.0.get()).write(Client::new(SERIAL_DRIVER)) };
}

fn spawn_worker() {
    // SAFETY: `STACKS` is exclusive to this init and the spawned worker.
    let stacks = unsafe { &mut *STACKS.0.get() };
    let ptrs = core::array::from_fn(|index| stacks[index].0.as_mut_ptr());
    // SAFETY: the static stacks outlive the protection domain and do not overlap.
    unsafe { microkit_cothread_init(STACK_SIZE, &ptrs) };
    let handle = microkit_cothread_spawn(worker, core::ptr::null_mut());
    assert_ne!(handle, NULL_HANDLE, "worker cothread slot is free");
    microkit_cothread_yield();
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        #[cfg(not(feature = "console-out"))]
        if channels.contains(Channel::new(TX_CH)) {
            microkit_cothread_recv_ntfn(TX_CH);
        }
        #[cfg(feature = "console-out")]
        if channels.contains(SERIAL_DRIVER) {
            microkit_cothread_recv_ntfn(SERIAL_DRIVER.index());
        }
        #[cfg(not(feature = "console-out"))]
        if channels.contains(Channel::new(RX_CH)) {
            microkit_cothread_recv_ntfn(RX_CH);
        }
        // On the I217 shell channel 2 is the transmit virtualiser, not fatfs.
        #[cfg(not(feature = "board-pc_z97_d3h_i217"))]
        if channels.contains(FATFS) {
            microkit_cothread_recv_ntfn(FS_CH);
        }
        #[cfg(feature = "net-client")]
        if channels.contains(NET_COPY) || channels.contains(NET_VIRT_TX) {
            net::on_notify(channels);
        }
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
    #[cfg(not(feature = "board-pc_z97_d3h_i217"))]
    use lerux_sddf::net_image::{SHELL_NET_RX_CHANNEL, SHELL_NET_TX_CHANNEL};
    use lerux_sddf::{net_client_config_t, net_image, SDDF_NET_MAGIC};

    let net_len = size_of::<net_client_config_t>();
    assert_eq!(CONFIG.len(), at + net_len);
    // SAFETY: the build script wrote the network client page after the filesystem page.
    let net = unsafe { net_image::net_config_from_bytes::<net_client_config_t>(&CONFIG[at..]) };
    assert_eq!(net.magic, SDDF_NET_MAGIC);
    #[cfg(feature = "board-pc_z97_d3h_i217")]
    {
        assert_eq!(net.rx.id, lerux_sddf::net_image::I217_SHELL_NET_RX_CHANNEL);
        assert_eq!(net.tx.id, lerux_sddf::net_image::I217_SHELL_NET_TX_CHANNEL);
    }
    #[cfg(not(feature = "board-pc_z97_d3h_i217"))]
    {
        assert_eq!(net.rx.id, SHELL_NET_RX_CHANNEL);
        assert_eq!(net.tx.id, SHELL_NET_TX_CHANNEL);
    }
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
