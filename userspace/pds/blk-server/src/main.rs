#![no_std]
#![no_main]

extern crate alloc;

use alloc::rc::Rc;
use core::task::Poll;

use async_unsync::semaphore::Semaphore;
use lerux_interface_types::{BlockRequest, BlockResponse, SECTOR_SIZE};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_abstract_allocator::{basic::BasicAllocator, WithAlignmentBound};
use sel4_driver_interfaces::block::GetBlockDeviceLayout;
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo,
};
use sel4_microkit_driver_adapters::block::client::Client as BlockClient;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::RingBuffers;
use sel4_shared_ring_buffer_block_io::OwnedSharedRingBufferBlockIO;
use sel4_shared_ring_buffer_block_io_types::BlockIORequest;

mod config;

const BLK_DRIVER: Channel = Channel::new(1);
const CLIENT: Channel = Channel::new(2);

type BlkIo = OwnedSharedRingBufferBlockIO<Rc<Semaphore>, WithAlignmentBound<BasicAllocator>, fn()>;

#[expect(clippy::large_enum_variant)] // 512-byte sector buffer in the active read state
enum ReadState {
    Idle,
    Reading {
        request_index: usize,
        buf: [u8; SECTOR_SIZE],
    },
}

struct HandlerImpl {
    blk_io: BlkIo,
    read_state: ReadState,
    block_size: usize,
    completed_sector: Option<[u8; SECTOR_SIZE]>,
}

fn create_blk_dma_region() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_blk_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_BLK_CLIENT_DMA_SIZE
        ))
    }
}

fn create_blk_ring_buffers(
) -> RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn(), BlockIORequest> {
    let notify_block: fn() = || BLK_DRIVER.notify();
    RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_used: *mut _)) },
        notify_block,
    )
}

fn create_blk_io() -> BlkIo {
    let dma_region = create_blk_dma_region();
    let bounce_buffer_allocator =
        WithAlignmentBound::new(BasicAllocator::new(dma_region.as_ptr().len()), 1);
    let ring_buffers = create_blk_ring_buffers();
    OwnedSharedRingBufferBlockIO::new(dma_region, bounce_buffer_allocator, ring_buffers)
}

fn log_blk_info(blk: &mut BlockClient) -> usize {
    let block_size = blk.get_block_size().unwrap();
    let num_blocks = blk.get_num_blocks().unwrap();
    log::info!("virtio-blk: {num_blocks} blocks x {block_size} bytes");
    block_size
}

fn issue_read(io: &mut BlkIo, lba: u32, block_size: usize) -> usize {
    let sem = io.slot_set_semaphore().clone();
    let mut reservation = sem.try_reserve(1).unwrap().expect("blk slot reservation");
    io.issue_read_request(&mut reservation, u64::from(lba), block_size)
        .unwrap()
}

fn advance_read(read_state: &mut ReadState, io: &mut BlkIo) -> BlockResponse {
    let ReadState::Reading { request_index, buf } = read_state else {
        return BlockResponse::Error;
    };

    io.poll().unwrap();
    match io.poll_read_request(*request_index, buf, None).unwrap() {
        Poll::Ready(Ok(())) => {
            let data = *buf;
            *read_state = ReadState::Idle;
            BlockResponse::Sector { data }
        }
        Poll::Pending => BlockResponse::Pending,
        Poll::Ready(Err(_)) => {
            *read_state = ReadState::Idle;
            BlockResponse::Error
        }
    }
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    let mut blk = BlockClient::new(BLK_DRIVER);
    let block_size = log_blk_info(&mut blk);
    log::info!("lerux-blk: ready");
    HandlerImpl {
        blk_io: create_blk_io(),
        read_state: ReadState::Idle,
        block_size,
        completed_sector: None,
    }
}

impl HandlerImpl {
    fn start_read(&mut self, lba: u32) {
        if !matches!(self.read_state, ReadState::Idle) {
            return;
        }
        let request_index = issue_read(&mut self.blk_io, lba, self.block_size);
        self.read_state = ReadState::Reading {
            request_index,
            buf: [0; SECTOR_SIZE],
        };
    }

    fn handle_read_sector(&mut self, lba: u32) -> BlockResponse {
        if let Some(data) = self.completed_sector.take() {
            return BlockResponse::Sector { data };
        }
        if matches!(self.read_state, ReadState::Idle) {
            self.start_read(lba);
        }
        BlockResponse::Pending
    }

    fn store_completed_read(&mut self) {
        if let BlockResponse::Sector { data } = advance_read(&mut self.read_state, &mut self.blk_io)
        {
            log::info!(
                "lerux-blk: MBR sig 0x{:02x} 0x{:02x}",
                data[SECTOR_SIZE - 2],
                data[SECTOR_SIZE - 1]
            );
            self.completed_sector = Some(data);
        }
    }

    fn handle_poll(&mut self) -> BlockResponse {
        if let Some(data) = self.completed_sector.take() {
            return BlockResponse::Sector { data };
        }
        if matches!(self.read_state, ReadState::Reading { .. }) {
            BLK_DRIVER.notify();
        }
        advance_read(&mut self.read_state, &mut self.blk_io)
    }

    fn handle_blk_driver(&mut self) {
        if matches!(self.read_state, ReadState::Reading { .. }) {
            self.store_completed_read();
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT {
            unreachable!();
        }

        Ok(match recv::<BlockRequest>(msg_info) {
            Ok(req) => match req {
                BlockRequest::ReadSector { lba } => send(self.handle_read_sector(lba)),
                BlockRequest::Poll => send(self.handle_poll()),
            },
            Err(_) => send_unspecified_error(),
        })
    }

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(BLK_DRIVER) {
            self.handle_blk_driver();
        }
        Ok(())
    }
}
