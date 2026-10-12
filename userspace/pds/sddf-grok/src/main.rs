//! Serial client that notifies the agent protection domain.
//!
//! `grok` writes one [`agent_tool_t`] into the shared page and notifies the
//! agent. The worker waits for the completion. Browse is answered on the root,
//! because the worker is already waiting for that completion.

#![no_std]
#![no_main]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_sddf::{
    agent_tool_t, serial_cancel_consumer_signal, serial_client_config_t, serial_dequeue,
    serial_enqueue, serial_handle_from_connection,
    serial_image::{self, CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL},
    serial_queue_handle_t, serial_request_consumer_signal, serial_require_consumer_signal,
    AGENT_KIND_BROWSE, AGENT_KIND_EDIT, AGENT_KIND_LIST_DIR, AGENT_KIND_READ, AGENT_KIND_SEARCH,
    AGENT_KIND_WEB_FETCH, AGENT_KIND_WRITE, AGENT_RESULT_CAP, AGENT_STATE_BROWSE,
    AGENT_STATE_BROWSE_DONE, AGENT_STATE_OK, AGENT_TOOL_VADDR, SDDF_SERIAL_MAGIC,
    SHELL_BROWSE_CHANNEL, SHELL_TOOL_CHANNEL,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const TX_CH: usize = CLIENT_TX_CHANNEL as usize;
const RX_CH: usize = CLIENT_RX_CHANNEL as usize;
const TOOL_CH: usize = SHELL_TOOL_CHANNEL as usize;
const BROWSE_CH: usize = SHELL_BROWSE_CHANNEL as usize;
const STACK_SIZE: usize = 0x4000;
const LINE_MAX: usize = 80;
const READY: &[u8] = b"lerux shell ready\n";

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

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    // SAFETY: the build script wrote a `serial_client_config_t`.
    let serial =
        unsafe { serial_image::serial_config_from_bytes::<serial_client_config_t>(CONFIG) };
    assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(serial.tx.id, CLIENT_TX_CHANNEL);
    assert_eq!(serial.rx.id, CLIENT_RX_CHANNEL);
    assert_eq!(SHELL_TOOL_CHANNEL, 2);
    assert_eq!(SHELL_BROWSE_CHANNEL, 3);
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
        if channels.contains(Channel::new(BROWSE_CH)) {
            answer_browse();
        }
        if channels.contains(Channel::new(TX_CH)) {
            microkit_cothread_recv_ntfn(TX_CH);
        }
        if channels.contains(Channel::new(RX_CH)) {
            microkit_cothread_recv_ntfn(RX_CH);
        }
        if channels.contains(Channel::new(TOOL_CH)) {
            microkit_cothread_recv_ntfn(TOOL_CH);
        }
        Ok(())
    }
}

extern "C" fn worker() {
    say(READY);
    let mut line = [0u8; LINE_MAX];
    loop {
        let len = read_line(&mut line);
        dispatch(&line[..len]);
    }
}

fn io() -> &'static mut Io {
    // SAFETY: init wrote this cell before the worker was spawned.
    unsafe { (*IO.0.get()).assume_init_mut() }
}

fn tool_page() -> &'static mut agent_tool_t {
    // SAFETY: the template maps the tool page at `AGENT_TOOL_VADDR` for this client and the agent.
    unsafe { &mut *(AGENT_TOOL_VADDR as *mut agent_tool_t) }
}

fn answer_browse() {
    let tool = tool_page();
    if tool.state != AGENT_STATE_BROWSE || tool.kind != AGENT_KIND_BROWSE {
        return;
    }
    let name_len = usize::try_from(tool.name_len).unwrap_or(usize::MAX);
    if name_len > tool.name.len() || name_len > tool.result.len() {
        tool.state = AGENT_STATE_BROWSE_DONE;
        tool.result_len = 0;
        Channel::new(BROWSE_CH).notify();
        return;
    }
    tool.result[..name_len].copy_from_slice(&tool.name[..name_len]);
    tool.result_len = name_len as u32;
    tool.state = AGENT_STATE_BROWSE_DONE;
    Channel::new(BROWSE_CH).notify();
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
    if cmd != b"grok" {
        say(b"err\n");
        return;
    }
    let Some((verb, rest)) = split_first(rest) else {
        say(b"err\n");
        return;
    };
    match verb {
        b"write" => {
            let Some((name, text)) = split_first(rest) else {
                say(b"err\n");
                return;
            };
            if text.is_empty() {
                say(b"err\n");
                return;
            }
            run_tool(AGENT_KIND_WRITE, name, text);
        }
        b"read" | b"ls" => {
            let Some(name) = one_name(rest) else {
                say(b"err\n");
                return;
            };
            let kind = if verb == b"read" {
                AGENT_KIND_READ
            } else {
                AGENT_KIND_LIST_DIR
            };
            run_tool(kind, name, b"");
        }
        b"edit" | b"search" => {
            let Some((name, arg)) = split_first(rest) else {
                say(b"err\n");
                return;
            };
            if arg.is_empty() {
                say(b"err\n");
                return;
            }
            let kind = if verb == b"edit" {
                AGENT_KIND_EDIT
            } else {
                AGENT_KIND_SEARCH
            };
            run_tool(kind, name, arg);
        }
        b"fetch" | b"browse" => {
            let name = trim(rest);
            if name.is_empty() {
                say(b"err\n");
                return;
            }
            let kind = if verb == b"fetch" {
                AGENT_KIND_WEB_FETCH
            } else {
                AGENT_KIND_BROWSE
            };
            run_tool(kind, name, b"");
        }
        _ => say(b"err\n"),
    }
}

fn run_tool(kind: u32, name: &[u8], arg: &[u8]) {
    if !issue(kind, name, arg) {
        say(b"agent: tool err\n");
        return;
    }
    if kind == AGENT_KIND_WRITE {
        say(b"agent: tool write ok\n");
        return;
    }
    let tool = tool_page();
    let Ok(len) = usize::try_from(tool.result_len) else {
        say(b"agent: tool err\n");
        return;
    };
    if len > AGENT_RESULT_CAP {
        say(b"agent: tool err\n");
        return;
    }
    let mut result = [0u8; AGENT_RESULT_CAP];
    result[..len].copy_from_slice(&tool.result[..len]);
    say(&result[..len]);
    if len == 0 || result[len - 1] != b'\n' {
        say(b"\n");
    }
}

fn issue(kind: u32, name: &[u8], arg: &[u8]) -> bool {
    let tool = tool_page();
    let mut page = lerux_sddf::agent_tool_zeroed();
    if name.len() > page.name.len() || arg.len() > page.arg.len() {
        return false;
    }
    page.kind = kind;
    page.name_len = name.len() as u32;
    page.arg_len = arg.len() as u32;
    page.name[..name.len()].copy_from_slice(name);
    page.arg[..arg.len()].copy_from_slice(arg);
    *tool = page;
    Channel::new(TOOL_CH).notify();
    microkit_cothread_wait_on_channel(TOOL_CH);
    tool.state == AGENT_STATE_OK
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
