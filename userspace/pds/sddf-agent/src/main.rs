//! Agent protection domain.
//!
//! `grok` fills the shared [`agent_tool_t`] and notifies this domain. The
//! worker runs the tool, writes the result into that page, and notifies
//! `grok`. Browse asks `grok` to fill the result on the browse channel first.

#![no_std]
#![no_main]

mod net;
mod net_device;
mod tools;

use core::{cell::UnsafeCell, mem::size_of};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_sddf::{
    agent_tool_t, fs_client_config_t, fs_image, net_client_config_t, net_image,
    AGENT_BROWSE_CHANNEL, AGENT_FS_CHANNEL, AGENT_NET_RX_CHANNEL, AGENT_NET_TX_CHANNEL,
    AGENT_STATE_ERR, AGENT_STATE_OK, AGENT_TOOL_CHANNEL, AGENT_TOOL_VADDR, LIONS_FS_MAGIC,
    SDDF_NET_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const STACK_SIZE: usize = 0x1_0000;
const TOOL_CH: usize = AGENT_TOOL_CHANNEL as usize;
const BROWSE_CH: usize = AGENT_BROWSE_CHANNEL as usize;
const FS_CH: usize = AGENT_FS_CHANNEL as usize;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

struct HandlerImpl;

#[protection_domain(heap_size = 1024 * 1024)]
fn init() -> HandlerImpl {
    let fs_len = size_of::<fs_client_config_t>();
    let net_len = size_of::<net_client_config_t>();
    assert_eq!(CONFIG.len(), fs_len + net_len);
    // SAFETY: the build script wrote the filesystem shell page and then the network shell page.
    let fs = unsafe { fs_image::fs_config_from_bytes::<fs_client_config_t>(&CONFIG[..fs_len]) };
    let net = unsafe { net_image::net_config_from_bytes::<net_client_config_t>(&CONFIG[fs_len..]) };
    assert_eq!(fs.magic, LIONS_FS_MAGIC);
    assert_eq!(net.magic, SDDF_NET_MAGIC);
    assert_eq!(fs.server.id, AGENT_FS_CHANNEL);
    assert_eq!(net.rx.id, AGENT_NET_RX_CHANNEL);
    assert_eq!(net.tx.id, AGENT_NET_TX_CHANNEL);
    assert_eq!(AGENT_TOOL_CHANNEL, 0);
    assert_eq!(AGENT_BROWSE_CHANNEL, 1);
    net::install(&net);
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
        if channels.contains(Channel::new(TOOL_CH)) {
            microkit_cothread_recv_ntfn(TOOL_CH);
        }
        if channels.contains(Channel::new(BROWSE_CH)) {
            microkit_cothread_recv_ntfn(BROWSE_CH);
        }
        if channels.contains(Channel::new(FS_CH)) {
            microkit_cothread_recv_ntfn(FS_CH);
        }
        net::on_notify(channels);
        Ok(())
    }
}

extern "C" fn worker() {
    tools::bind();
    loop {
        microkit_cothread_wait_on_channel(TOOL_CH);
        let ok = tools::run();
        let tool = tool_page();
        if ok {
            tool.state = AGENT_STATE_OK;
        } else {
            tool.result_len = 0;
            tool.state = AGENT_STATE_ERR;
        }
        Channel::new(TOOL_CH).notify();
    }
}

pub(crate) fn tool_page() -> &'static mut agent_tool_t {
    // SAFETY: the template maps the tool page at `AGENT_TOOL_VADDR` for this domain and `grok`.
    unsafe { &mut *(AGENT_TOOL_VADDR as *mut agent_tool_t) }
}
