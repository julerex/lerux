#![no_std]
#![no_main]

#[cfg(feature = "virtio")]
extern crate alloc;

use lerux_logging::log;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

#[cfg(not(feature = "serial-ipc"))]
use lerux_logging::debug;
#[cfg(feature = "serial-ipc")]
use lerux_logging::serial;

#[cfg(feature = "virtio")]
use alloc::rc::Rc;
#[cfg(feature = "virtio")]
use async_unsync::semaphore::Semaphore;
#[cfg(feature = "virtio")]
use core::task::Poll;
#[cfg(feature = "virtio")]
use sel4_abstract_allocator::basic::BasicAllocator;
#[cfg(feature = "virtio")]
use sel4_abstract_allocator::WithAlignmentBound;
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

#[cfg(all(feature = "serial-ipc", feature = "composed-sync"))]
const SERIAL_DRIVER: Channel = Channel::new(3);
#[cfg(all(feature = "serial-ipc", not(feature = "composed-sync")))]
const SERIAL_DRIVER: Channel = Channel::new(0);
#[cfg(feature = "composed-sync")]
const BOOT_INIT: Channel = Channel::new(0);
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
    #[cfg(all(feature = "virtio", feature = "composed-sync"))]
    virtio_pending: bool,
    #[cfg(feature = "virtio")]
    blk_read: Option<BlkRead>,
    #[cfg(feature = "virtio")]
    net_io: Option<net::NetIo>,
}

fn init_logging() {
    #[cfg(feature = "serial-ipc")]
    serial::init(SERIAL_DRIVER).unwrap();

    #[cfg(not(feature = "serial-ipc"))]
    debug::init().unwrap();
}

#[cfg(all(feature = "virtio", feature = "composed-sync"))]
fn init_composed_sync() -> HandlerImpl {
    HandlerImpl {
        virtio_pending: true,
        blk_read: None,
        net_io: None,
    }
}

#[cfg(all(feature = "virtio", not(feature = "composed-sync")))]
fn init_with_virtio() -> HandlerImpl {
    log::info!("lerux: Hello from Rust on seL4 Microkit!");
    let (blk_read, net_io) = probe_virtio();
    HandlerImpl {
        blk_read: Some(blk_read),
        net_io: Some(net_io),
    }
}

#[cfg(not(feature = "virtio"))]
fn init_basic() -> HandlerImpl {
    log::info!("lerux: Hello from Rust on seL4 Microkit!");
    HandlerImpl {}
}

#[cfg_attr(feature = "virtio", protection_domain(heap_size = 512 * 1024))]
#[cfg_attr(not(feature = "virtio"), protection_domain)]
fn init() -> HandlerImpl {
    init_logging();
    #[cfg(all(feature = "virtio", feature = "composed-sync"))]
    return init_composed_sync();
    #[cfg(all(feature = "virtio", not(feature = "composed-sync")))]
    return init_with_virtio();
    #[cfg(not(feature = "virtio"))]
    init_basic()
}

#[cfg(feature = "virtio")]
fn log_blk_info(blk: &mut BlockClient) {
    let block_size = blk.get_block_size().unwrap();
    let num_blocks = blk.get_num_blocks().unwrap();
    log::info!("virtio-blk: {num_blocks} blocks x {block_size} bytes");
}

#[cfg(feature = "virtio")]
fn log_net_mac(net: &mut NetClient) -> sel4_driver_interfaces::net::MacAddress {
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
    mac
}

#[cfg(feature = "virtio")]
fn wait_for_net(net_io: &mut net::NetIo) {
    for _ in 0..2000 {
        net_io.poll();
        if net_io.is_done() {
            break;
        }
    }
}

#[cfg(feature = "virtio")]
fn probe_virtio() -> (BlkRead, net::NetIo) {
    let mut blk = BlockClient::new(BLK_DRIVER);
    log_blk_info(&mut blk);
    let mut net = NetClient::new(NET_DRIVER);
    let mac = log_net_mac(&mut net);
    let mut net_io = net::NetIo::new(mac);
    wait_for_net(&mut net_io);
    (start_blk_read(), net_io)
}

#[cfg(not(feature = "virtio"))]
fn probe_virtio() {}

#[cfg(feature = "virtio")]
fn create_blk_dma_region() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_blk_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_BLK_CLIENT_DMA_SIZE
        ))
    }
}

#[cfg(feature = "virtio")]
fn create_blk_ring_buffers() -> RingBuffers<
    'static,
    sel4_shared_ring_buffer::roles::Provide,
    fn(),
    sel4_shared_ring_buffer_block_io_types::BlockIORequest,
> {
    let notify_block: fn() = || BLK_DRIVER.notify();
    RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_used: *mut _)) },
        notify_block,
    )
}

#[cfg(feature = "virtio")]
fn issue_blk_read(io: &mut BlkIo) -> usize {
    let sem = io.slot_set_semaphore().clone();
    let mut reservation = sem.try_reserve(1).unwrap().expect("blk slot reservation");
    io.issue_read_request(&mut reservation, 0, 512).unwrap()
}

#[cfg(feature = "virtio")]
fn start_blk_read() -> BlkRead {
    let dma_region = create_blk_dma_region();
    let bounce_buffer_allocator =
        WithAlignmentBound::new(BasicAllocator::new(dma_region.as_ptr().len()), 1);
    let ring_buffers = create_blk_ring_buffers();
    let mut io =
        OwnedSharedRingBufferBlockIO::new(dma_region, bounce_buffer_allocator, ring_buffers);
    let request_index = issue_blk_read(&mut io);
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

#[cfg(feature = "virtio")]
impl HandlerImpl {
    #[cfg(feature = "composed-sync")]
    fn handle_boot_init(&mut self) {
        if !self.virtio_pending {
            return;
        }
        log::info!("lerux: Hello from Rust on seL4 Microkit!");
        let (blk_read, net_io) = probe_virtio();
        self.blk_read = Some(blk_read);
        self.net_io = Some(net_io);
        self.virtio_pending = false;
    }

    fn handle_blk_driver(&mut self) {
        if let Some(blk) = &mut self.blk_read
            && !blk.done
        {
            finish_blk_read(blk);
        }
    }

    fn handle_net_driver(&mut self) {
        if let Some(net_io) = &mut self.net_io
            && !net_io.is_done()
        {
            net_io.poll();
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(
        &mut self,
        #[cfg_attr(not(feature = "virtio"), allow(unused_variables))] channels: ChannelSet,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "virtio")]
        {
            #[cfg(feature = "composed-sync")]
            if channels.contains(BOOT_INIT) {
                self.handle_boot_init();
            }
            if channels.contains(BLK_DRIVER) {
                self.handle_blk_driver();
            }
            if channels.contains(NET_DRIVER) {
                self.handle_net_driver();
            }
        }
        Ok(())
    }
}
