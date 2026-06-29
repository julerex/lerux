#![no_std]
#![no_main]

#[cfg(feature = "virtio")]
extern crate alloc;

use lerux_logging::log;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

#[cfg(feature = "serial-ipc")]
use lerux_logging::serial;
#[cfg(not(feature = "serial-ipc"))]
use lerux_logging::debug;

#[cfg(feature = "virtio")]
use alloc::rc::Rc;
#[cfg(feature = "virtio")]
use async_unsync::semaphore::Semaphore;
#[cfg(feature = "virtio")]
use core::task::Poll;
#[cfg(feature = "virtio")]
use sel4_abstract_allocator::WithAlignmentBound;
#[cfg(feature = "virtio")]
use sel4_abstract_allocator::basic::BasicAllocator;
#[cfg(feature = "virtio")]
use sel4_driver_interfaces::block::GetBlockDeviceLayout;
#[cfg(feature = "virtio")]
use sel4_driver_interfaces::net::GetNetDeviceMeta;
#[cfg(feature = "virtio")]
use sel4_microkit::memory_region_symbol;
#[cfg(feature = "virtio")]
use sel4_microkit_driver_adapters::block::client::Client as BlockClient;
#[cfg(feature = "virtio")]
use sel4_microkit_driver_adapters::net::client::Client as NetClient;
#[cfg(feature = "virtio")]
use sel4_shared_memory::SharedMemoryRef;
#[cfg(feature = "virtio")]
use sel4_shared_ring_buffer::RingBuffers;
#[cfg(feature = "virtio")]
use sel4_shared_ring_buffer_block_io::OwnedSharedRingBufferBlockIO;


#[cfg(feature = "virtio")]
mod config;
#[cfg(feature = "virtio")]
mod net;

#[cfg(feature = "serial-ipc")]
const SERIAL_DRIVER: Channel = Channel::new(0);
#[cfg(feature = "virtio")]
const NET_DRIVER: Channel = Channel::new(1);
#[cfg(feature = "virtio")]
const BLK_DRIVER: Channel = Channel::new(2);

#[cfg(feature = "virtio")]
type BlkIo = OwnedSharedRingBufferBlockIO<Rc<Semaphore>, WithAlignmentBound<BasicAllocator>, fn()>;

#[cfg(feature = "virtio")]
struct BlkRead {
    io: BlkIo,
    request_index: usize,
    buf: [u8; 512],
    done: bool,
}

struct HandlerImpl {
    #[cfg(feature = "virtio")]
    blk_read: Option<BlkRead>,
    #[cfg(feature = "virtio")]
    net_io: Option<net::NetIo>,
}

#[cfg_attr(feature = "virtio", protection_domain(heap_size = 512 * 1024))]
#[cfg_attr(not(feature = "virtio"), protection_domain)]
fn init() -> HandlerImpl {
    init_logging();
    log::info!("lerux: Hello from Rust on seL4 Microkit!");
    #[cfg(feature = "virtio")]
    let (blk_read, net_io) = probe_virtio();
    HandlerImpl {
        #[cfg(feature = "virtio")]
        blk_read: Some(blk_read),
        #[cfg(feature = "virtio")]
        net_io: Some(net_io),
    }
}

fn init_logging() {
    #[cfg(feature = "serial-ipc")]
    serial::init(SERIAL_DRIVER).unwrap();

    #[cfg(not(feature = "serial-ipc"))]
    debug::init().unwrap();
}

#[cfg(feature = "virtio")]
fn probe_virtio() -> (BlkRead, net::NetIo) {
    let mut blk = BlockClient::new(BLK_DRIVER);
    let block_size = blk.get_block_size().unwrap();
    let num_blocks = blk.get_num_blocks().unwrap();
    log::info!("virtio-blk: {num_blocks} blocks x {block_size} bytes");

    let mut net = NetClient::new(NET_DRIVER);
    let mac = net.get_mac_address().unwrap();
    log::info!(
        "virtio-net: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac.0[0],
        mac.0[1],
        mac.0[2],
        mac.0[3],
        mac.0[4],
        mac.0[5],
    );

    let mut net_io = net::NetIo::new(mac);
    for _ in 0..2000 {
        net_io.poll();
        if net_io.is_done() {
            break;
        }
    }
    (start_blk_read(), net_io)
}

#[cfg(not(feature = "virtio"))]
fn probe_virtio() {}

#[cfg(feature = "virtio")]
fn start_blk_read() -> BlkRead {
    let notify_block: fn() = || BLK_DRIVER.notify();

    let dma_region = unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_blk_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_BLK_CLIENT_DMA_SIZE
        ))
    };

    let bounce_buffer_allocator =
        WithAlignmentBound::new(BasicAllocator::new(dma_region.as_ptr().len()), 1);

    let ring_buffers = RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_used: *mut _)) },
        notify_block,
    );

    let mut io = OwnedSharedRingBufferBlockIO::new(dma_region, bounce_buffer_allocator, ring_buffers);

    let sem = io.slot_set_semaphore().clone();
    let mut reservation = sem.try_reserve(1).unwrap().expect("blk slot reservation");
    let request_index = io.issue_read_request(&mut reservation, 0, 512).unwrap();

    BlkRead {
        io,
        request_index,
        buf: [0; 512],
        done: false,
    }
}

#[cfg(feature = "virtio")]
fn finish_blk_read(blk: &mut BlkRead) {
    blk.io.poll().unwrap();
    match blk
        .io
        .poll_read_request(blk.request_index, &mut blk.buf, None)
        .unwrap()
    {
        Poll::Ready(Ok(())) => {
            log::info!(
                "virtio-blk: MBR sig 0x{:02x} 0x{:02x}",
                blk.buf[510],
                blk.buf[511]
            );
            blk.done = true;
        }
        Poll::Pending => {}
        Poll::Ready(Err(_)) => panic!("virtio-blk read failed"),
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, #[cfg_attr(not(feature = "virtio"), allow(unused_variables))] channels: ChannelSet) -> Result<(), Self::Error> {
        #[cfg(feature = "virtio")]
        {
            if channels.contains(BLK_DRIVER) {
                if let Some(blk) = &mut self.blk_read {
                    if !blk.done {
                        finish_blk_read(blk);
                    }
                }
            }
            if channels.contains(NET_DRIVER) {
                if let Some(net_io) = &mut self.net_io {
                    if !net_io.is_done() {
                        net_io.poll();
                    }
                }
            }
        }
        Ok(())
    }
}