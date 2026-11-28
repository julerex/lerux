#![no_std]
#![no_main]

use core::cell::UnsafeCell;

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_logging::{debug, log};
use lerux_sddf::{
    fs_buffer_t, fs_client_config_t, fs_cmd_params_file_open_t, fs_cmd_params_file_read_t,
    fs_cmd_params_file_write_t, fs_cmd_t, fs_command_enqueue,
    fs_image::{self, FS_CLIENT_SERVER_CHANNEL, FS_SHARE_VADDR},
    fs_message_dequeue, fs_msg_t, fs_queue_t, FS_CMD_FILE_OPEN, FS_CMD_FILE_READ,
    FS_CMD_FILE_WRITE, FS_OPEN_FLAGS_CREATE, FS_OPEN_FLAGS_READ_ONLY, FS_OPEN_FLAGS_WRITE_ONLY,
    FS_STATUS_SUCCESS, LIONS_FS_MAGIC,
};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const SERVER_CH: usize = FS_CLIENT_SERVER_CHANNEL as usize;
const STACK_SIZE: usize = 0x4000;
const PAYLOAD: &[u8] = b"fs-sddf payload";
const NAME: &[u8] = b"SMOKE.TXT";

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

struct HandlerImpl;

extern "C" fn worker() {
    let share = FS_SHARE_VADDR as *mut u8;
    // SAFETY: the template maps the share at `FS_SHARE_VADDR` for both names and file bytes.
    unsafe {
        core::ptr::copy_nonoverlapping(NAME.as_ptr(), share, NAME.len());
    }
    if !issue(open_cmd(1, FS_OPEN_FLAGS_CREATE | FS_OPEN_FLAGS_WRITE_ONLY)) {
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(PAYLOAD.as_ptr(), share.add(0x1000), PAYLOAD.len());
    }
    if !issue(write_cmd(2)) {
        return;
    }
    if !issue(open_cmd(3, FS_OPEN_FLAGS_READ_ONLY)) {
        return;
    }
    unsafe {
        core::ptr::write_bytes(share.add(0x1000), 0, PAYLOAD.len());
    }
    if !issue(read_cmd(4)) {
        return;
    }
    let mut got = [0u8; PAYLOAD.len()];
    unsafe {
        core::ptr::copy_nonoverlapping(share.add(0x1000), got.as_mut_ptr(), PAYLOAD.len());
    }
    if got == PAYLOAD {
        log::info!("fs-sddf read ok");
    } else {
        log::info!("fs-sddf read mismatch");
    }
}

fn open_cmd(id: u64, flags: u64) -> fs_cmd_t {
    let mut cmd: fs_cmd_t = unsafe { core::mem::zeroed() };
    cmd.id = id;
    cmd.r#type = FS_CMD_FILE_OPEN;
    cmd.params.file_open = fs_cmd_params_file_open_t {
        path: fs_buffer_t {
            offset: 0,
            size: NAME.len() as u64,
        },
        flags,
    };
    cmd
}

fn write_cmd(id: u64) -> fs_cmd_t {
    let mut cmd: fs_cmd_t = unsafe { core::mem::zeroed() };
    cmd.id = id;
    cmd.r#type = FS_CMD_FILE_WRITE;
    cmd.params.file_write = fs_cmd_params_file_write_t {
        fd: 0,
        offset: 0,
        buf: fs_buffer_t {
            offset: 0x1000,
            size: PAYLOAD.len() as u64,
        },
    };
    cmd
}

fn read_cmd(id: u64) -> fs_cmd_t {
    let mut cmd: fs_cmd_t = unsafe { core::mem::zeroed() };
    cmd.id = id;
    cmd.r#type = FS_CMD_FILE_READ;
    cmd.params.file_read = fs_cmd_params_file_read_t {
        fd: 0,
        offset: 0,
        buf: fs_buffer_t {
            offset: 0x1000,
            size: PAYLOAD.len() as u64,
        },
    };
    cmd
}

fn issue(cmd: fs_cmd_t) -> bool {
    let commands = lerux_sddf::fs_image::FS_COMMAND_QUEUE_VADDR as *mut fs_queue_t;
    let completions = lerux_sddf::fs_image::FS_COMPLETION_QUEUE_VADDR as *mut fs_queue_t;
    // SAFETY: this protection domain is the producer on the command ring.
    if unsafe { fs_command_enqueue(commands, cmd) } != 0 {
        log::info!("fs-sddf failed");
        return false;
    }
    Channel::new(SERVER_CH).notify();
    let mut msg: fs_msg_t = unsafe { core::mem::zeroed() };
    loop {
        microkit_cothread_wait_on_channel(SERVER_CH);
        // SAFETY: this protection domain is the consumer on the completion ring.
        if unsafe { fs_message_dequeue(completions, &mut msg) } == 0 {
            break;
        }
    }
    let cmpl = unsafe { msg.cmpl };
    if cmpl.id != cmd.id || cmpl.status != FS_STATUS_SUCCESS {
        log::info!("fs-sddf failed");
        return false;
    }
    true
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote an `fs_client_config_t`.
    let config = unsafe { fs_image::fs_config_from_bytes::<fs_client_config_t>(CONFIG) };
    assert_eq!(config.magic, LIONS_FS_MAGIC);
    assert_eq!(config.server.id, FS_CLIENT_SERVER_CHANNEL);
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
        if channels.contains(Channel::new(SERVER_CH)) {
            microkit_cothread_recv_ntfn(SERVER_CH);
        }
        Ok(())
    }
}
