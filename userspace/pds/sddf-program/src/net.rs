//! Transmit-queue setup for the one network client.
//!
//! The worker does not bring up Dynamic Host Configuration Protocol. `socket`
//! allocates the `lerux-posix` entry. The queues are still this client's.

use core::{cell::UnsafeCell, mem::MaybeUninit};

use sel4_microkit::{Channel, ChannelSet};

use lerux_cothread::microkit_cothread_recv_ntfn;
use lerux_sddf::{
    net_buffers_init, net_client_config_t, net_connection_resource_t, net_queue_handle_t,
    net_queue_init,
};

struct Chans {
    rx: usize,
    tx: usize,
}

struct ChansCell(UnsafeCell<MaybeUninit<Chans>>);

// SAFETY: init writes this cell before the worker starts. The root only reads the channel ids.
unsafe impl Sync for ChansCell {}

static CHANS: ChansCell = ChansCell(UnsafeCell::new(MaybeUninit::uninit()));

pub fn install(net: &net_client_config_t) {
    let tx = queue_from(&net.tx);
    // SAFETY: this client produces the transmit free queue once.
    unsafe { net_buffers_init(&tx, 0) };
    // SAFETY: init is the only writer, and it runs before the worker.
    unsafe {
        (*CHANS.0.get()).write(Chans {
            rx: usize::from(net.rx.id),
            tx: usize::from(net.tx.id),
        });
    }
}

pub fn on_notify(channels: ChannelSet) {
    let chans = chans();
    if channels.contains(Channel::new(chans.rx)) {
        microkit_cothread_recv_ntfn(chans.rx);
    }
    if channels.contains(Channel::new(chans.tx)) {
        microkit_cothread_recv_ntfn(chans.tx);
    }
}

fn chans() -> &'static Chans {
    // SAFETY: `install` wrote this cell before the worker was spawned.
    unsafe { (*CHANS.0.get()).assume_init_ref() }
}

fn queue_from(conn: &net_connection_resource_t) -> net_queue_handle_t {
    let mut handle = net_queue_handle_t {
        free: core::ptr::null_mut(),
        active: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps these queues into this protection domain.
    unsafe {
        net_queue_init(
            &mut handle,
            conn.free_queue.vaddr.cast(),
            conn.active_queue.vaddr.cast(),
            u32::from(conn.num_buffers),
        );
    }
    handle
}
