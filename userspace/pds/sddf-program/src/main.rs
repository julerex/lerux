//! Runtime protection domain.
//!
//! This is the serial client, the filesystem client, and the network client.
//! The worker checks a signed ordinary WebAssembly module. A bad signature
//! does not instantiate it. The module reads the seeded hostname and allocates
//! a socket through `lerux-posix`, then prints.

#![no_std]
#![no_main]

mod net;

use core::{
    cell::UnsafeCell,
    mem::{size_of, MaybeUninit},
};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_posix::{Errno, Files, Open, Sockets, Tcp};
use lerux_sddf::{
    fs_client_config_t,
    fs_image::{self, FS_SHELL_SERVER_CHANNEL},
    fs_queue_t, net_client_config_t, net_image, serial_client_config_t, serial_enqueue,
    serial_handle_from_connection,
    serial_image::{self, CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL},
    serial_queue_handle_t, serial_request_consumer_signal, LIONS_FS_MAGIC, SDDF_NET_MAGIC,
    SDDF_SERIAL_MAGIC,
};
use lerux_wasm::{run_signed, Error, Imports};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const GOOD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/smoke.lrw"));
const BAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/smoke-bad.lrw"));
const VERIFYING_KEY: [u8; 32] = *include_bytes!("../../../../support/keys/smoke.ed25519.pub");
const STACK_SIZE: usize = 0x1_0000;
const TX_CH: usize = CLIENT_TX_CHANNEL as usize;
const RX_CH: usize = CLIENT_RX_CHANNEL as usize;
const FS_CH: usize = FS_SHELL_SERVER_CHANNEL as usize;

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
}

struct IoCell(UnsafeCell<MaybeUninit<Io>>);

// SAFETY: init writes this cell before the worker starts. The root does not read it again.
unsafe impl Sync for IoCell {}

static IO: IoCell = IoCell(UnsafeCell::new(MaybeUninit::uninit()));

struct FilesCell(UnsafeCell<MaybeUninit<Files>>);

// SAFETY: the worker is the only user. The table is several kilobytes, so it stays off the stack.
unsafe impl Sync for FilesCell {}

static FILES: FilesCell = FilesCell(UnsafeCell::new(MaybeUninit::uninit()));

struct SocksCell(UnsafeCell<MaybeUninit<Sockets<Idle>>>);

// SAFETY: the worker creates the table once and is the only caller afterwards.
unsafe impl Sync for SocksCell {}

static SOCKS: SocksCell = SocksCell(UnsafeCell::new(MaybeUninit::uninit()));

struct HandlerImpl;

struct Guest {
    logged: bool,
}

impl Imports for Guest {
    fn log(&mut self, bytes: &[u8]) {
        self.logged = true;
        say(bytes);
    }

    fn read(&mut self, path: &[u8], dst: &mut [u8]) -> i32 {
        read_file(path, dst)
    }

    fn socket(&mut self) -> i32 {
        open_socket()
    }
}

/// Pipe that can be allocated and does not connect.
///
/// The smoke module only calls `socket`. The network queues stay mapped so a
/// later connect has a client.
struct Idle;

impl Tcp for Idle {
    fn start(&mut self) {}

    fn poll(&mut self) -> bool {
        false
    }

    fn connect(&mut self, _addr: [u8; 4], _port: u16) -> Result<(), Errno> {
        Err(Errno(Errno::ENOTCONN))
    }

    fn ready(&self) -> bool {
        false
    }

    fn failed(&self) -> bool {
        false
    }

    fn send(&mut self, _data: &[u8]) -> Result<usize, Errno> {
        Err(Errno(Errno::ENOTCONN))
    }

    fn recv(&mut self, _buf: &mut [u8]) -> Result<usize, Errno> {
        Err(Errno(Errno::ENOTCONN))
    }
}

#[protection_domain(heap_size = 2 * 1024 * 1024)]
fn init() -> HandlerImpl {
    let serial_len = size_of::<serial_client_config_t>();
    let fs_len = size_of::<fs_client_config_t>();
    let net_len = size_of::<net_client_config_t>();
    assert_eq!(CONFIG.len(), serial_len + fs_len + net_len);
    // SAFETY: the build script wrote the serial page, the filesystem page, then the network page.
    let serial = unsafe {
        serial_image::serial_config_from_bytes::<serial_client_config_t>(&CONFIG[..serial_len])
    };
    let fs = unsafe {
        fs_image::fs_config_from_bytes::<fs_client_config_t>(
            &CONFIG[serial_len..serial_len + fs_len],
        )
    };
    let net = unsafe {
        net_image::net_config_from_bytes::<net_client_config_t>(&CONFIG[serial_len + fs_len..])
    };
    assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(fs.magic, LIONS_FS_MAGIC);
    assert_eq!(net.magic, SDDF_NET_MAGIC);
    assert_eq!(serial.tx.id, CLIENT_TX_CHANNEL);
    assert_eq!(serial.rx.id, CLIENT_RX_CHANNEL);
    assert_eq!(fs.server.id, FS_SHELL_SERVER_CHANNEL);
    assert_eq!(net.rx.id, net_image::SHELL_NET_RX_CHANNEL);
    assert_eq!(net.tx.id, net_image::SHELL_NET_TX_CHANNEL);
    net::install(&net);
    // SAFETY: the template maps this client's transmit region.
    let tx = unsafe { serial_handle_from_connection(&serial.tx) };
    unsafe { (*IO.0.get()).write(Io { tx }) };
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
        net::on_notify(channels);
        Ok(())
    }
}

extern "C" fn worker() {
    if !bind_files() {
        say(b"program: err\n");
        return;
    }
    bind_sockets();
    let mut guest = Guest { logged: false };
    match run_signed(BAD, &VERIFYING_KEY, &mut guest) {
        Err(Error::Signature) => say(b"program: rejected\n"),
        _ => {
            say(b"program: err\n");
            return;
        }
    }
    guest.logged = false;
    if run_signed(GOOD, &VERIFYING_KEY, &mut guest).is_err() || !guest.logged {
        say(b"program: err\n");
    }
}

fn bind_files() -> bool {
    // SAFETY: the template maps these three regions. This worker is the only client.
    let files = unsafe {
        Files::bind(
            fs_image::FS_COMMAND_QUEUE_VADDR as *mut fs_queue_t,
            fs_image::FS_COMPLETION_QUEUE_VADDR as *mut fs_queue_t,
            fs_image::FS_SHARE_VADDR as *mut u8,
            fs_image::FS_REGION_SIZE as usize,
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

fn bind_sockets() {
    // SAFETY: the worker is the only writer, and it runs before the module.
    unsafe { (*SOCKS.0.get()).write(Sockets::new(Idle)) };
}

fn wake_fs(_: *mut ()) {
    Channel::new(FS_CH).notify();
    microkit_cothread_wait_on_channel(FS_CH);
}

fn files() -> &'static mut Files {
    // SAFETY: `bind_files` wrote this cell, and only the worker calls this.
    unsafe { (*FILES.0.get()).assume_init_mut() }
}

fn sockets() -> &'static mut Sockets<Idle> {
    // SAFETY: `bind_sockets` wrote this cell, and only the worker calls this.
    unsafe { (*SOCKS.0.get()).assume_init_mut() }
}

fn read_file(path: &[u8], dst: &mut [u8]) -> i32 {
    let files = files();
    let Ok(fd) = files.open(path, Open::Read) else {
        return -1;
    };
    let read = files.read(fd, dst);
    let _ = files.close(fd);
    match read {
        Ok(n) => i32::try_from(n).unwrap_or(-1),
        Err(_) => -1,
    }
}

fn open_socket() -> i32 {
    match sockets().socket() {
        Ok(fd) => fd.number(),
        Err(_) => -1,
    }
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

fn io() -> &'static Io {
    // SAFETY: init wrote this cell before spawning the worker.
    unsafe { (*IO.0.get()).assume_init_ref() }
}
