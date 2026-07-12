//! Shared virtio-blk ring client for LERUXFS1 and FAT backends.

use alloc::rc::Rc;
use core::task::Poll;

use async_unsync::semaphore::Semaphore;
use lerux_interface_types::SECTOR_SIZE;
use sel4_abstract_allocator::{basic::BasicAllocator, WithAlignmentBound};
use sel4_microkit::{memory_region_symbol, Channel};
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::RingBuffers;
use sel4_shared_ring_buffer_block_io::OwnedSharedRingBufferBlockIO;
use sel4_shared_ring_buffer_block_io_types::BlockIORequest;

use crate::config;

pub const BLK_DRIVER: Channel = Channel::new(1);
pub const CLIENT: Channel = Channel::new(2);
#[cfg(feature = "workstation")]
pub const LOG_SERVER: Channel = Channel::new(4);

pub type BlkIo =
    OwnedSharedRingBufferBlockIO<Rc<Semaphore>, WithAlignmentBound<BasicAllocator>, fn()>;

#[expect(clippy::large_enum_variant)]
pub enum IoState {
    Idle,
    Reading {
        request_index: usize,
        buf: [u8; SECTOR_SIZE],
    },
    Writing {
        request_index: usize,
    },
}

pub fn create_blk_dma_region() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_blk_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_BLK_CLIENT_DMA_SIZE
        ))
    }
}

pub fn create_blk_ring_buffers(
) -> RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn(), BlockIORequest> {
    let notify_block: fn() = || BLK_DRIVER.notify();
    RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_used: *mut _)) },
        notify_block,
    )
}

pub fn create_blk_io() -> BlkIo {
    let dma_region = create_blk_dma_region();
    let bounce_buffer_allocator =
        WithAlignmentBound::new(BasicAllocator::new(dma_region.as_ptr().len()), 1);
    let ring_buffers = create_blk_ring_buffers();
    OwnedSharedRingBufferBlockIO::new(dma_region, bounce_buffer_allocator, ring_buffers)
}

fn issue_read(io: &mut BlkIo, lba: u32, block_size: usize) -> usize {
    let sem = io.slot_set_semaphore().clone();
    let mut reservation = sem.try_reserve(1).unwrap().expect("blk slot reservation");
    io.issue_read_request(&mut reservation, u64::from(lba), block_size)
        .unwrap()
}

fn issue_write(io: &mut BlkIo, lba: u32, data: &[u8]) -> usize {
    let sem = io.slot_set_semaphore().clone();
    let mut reservation = sem.try_reserve(1).unwrap().expect("blk slot reservation");
    io.issue_write_request(&mut reservation, u64::from(lba), data)
        .unwrap()
}

fn advance_read(io_state: &mut IoState, io: &mut BlkIo) -> Option<[u8; SECTOR_SIZE]> {
    let IoState::Reading { request_index, buf } = io_state else {
        return None;
    };

    io.poll().unwrap();
    match io.poll_read_request(*request_index, buf, None).unwrap() {
        Poll::Ready(Ok(())) => {
            let data = *buf;
            *io_state = IoState::Idle;
            Some(data)
        }
        Poll::Pending => None,
        Poll::Ready(Err(_)) => {
            *io_state = IoState::Idle;
            None
        }
    }
}

fn advance_write(io_state: &mut IoState, io: &mut BlkIo) -> bool {
    let IoState::Writing { request_index } = io_state else {
        return false;
    };

    io.poll().unwrap();
    match io.poll_write_request(*request_index, None).unwrap() {
        Poll::Ready(Ok(())) => {
            *io_state = IoState::Idle;
            true
        }
        Poll::Pending => false,
        Poll::Ready(Err(_)) => {
            *io_state = IoState::Idle;
            false
        }
    }
}

/// Sector I/O helper held by both backends.
pub struct SectorIo {
    pub blk_io: BlkIo,
    pub io_state: IoState,
    pub block_size: usize,
    pub completed_sector: Option<[u8; SECTOR_SIZE]>,
    pub completed_ok: bool,
    pub pending_read_lba: Option<u32>,
    pub pending_write_lba: Option<u32>,
    pub sector_buf: [u8; SECTOR_SIZE],
}

impl SectorIo {
    pub fn new(block_size: usize) -> Self {
        Self {
            blk_io: create_blk_io(),
            io_state: IoState::Idle,
            block_size,
            completed_sector: None,
            completed_ok: false,
            pending_read_lba: None,
            pending_write_lba: None,
            sector_buf: [0; SECTOR_SIZE],
        }
    }

    pub fn start_read(&mut self, lba: u32) {
        if !matches!(self.io_state, IoState::Idle) {
            return;
        }
        let request_index = issue_read(&mut self.blk_io, lba, self.block_size);
        self.io_state = IoState::Reading {
            request_index,
            buf: [0; SECTOR_SIZE],
        };
        self.pending_read_lba = Some(lba);
    }

    pub fn start_write(&mut self, lba: u32, data: &[u8; SECTOR_SIZE]) {
        if !matches!(self.io_state, IoState::Idle) {
            return;
        }
        self.sector_buf.copy_from_slice(data);
        let request_index = issue_write(&mut self.blk_io, lba, &self.sector_buf[..self.block_size]);
        self.io_state = IoState::Writing { request_index };
        self.pending_write_lba = Some(lba);
    }

    pub fn poll_read_sector(&mut self, lba: u32) -> Option<[u8; SECTOR_SIZE]> {
        if self.pending_read_lba == Some(lba) {
            if let Some(data) = self.completed_sector.take() {
                self.pending_read_lba = None;
                return Some(data);
            }
            if let Some(data) = advance_read(&mut self.io_state, &mut self.blk_io) {
                self.pending_read_lba = None;
                return Some(data);
            }
            return None;
        }
        if self.pending_read_lba.is_none() && matches!(self.io_state, IoState::Idle) {
            self.start_read(lba);
        }
        None
    }

    pub fn poll_write_sector(&mut self, lba: u32, data: &[u8; SECTOR_SIZE]) -> bool {
        if self.pending_write_lba == Some(lba) {
            if self.completed_ok {
                self.completed_ok = false;
                self.pending_write_lba = None;
                return true;
            }
            if advance_write(&mut self.io_state, &mut self.blk_io) {
                self.pending_write_lba = None;
                return true;
            }
            return false;
        }
        if self.pending_write_lba.is_none() && matches!(self.io_state, IoState::Idle) {
            self.start_write(lba, data);
        }
        false
    }

    pub fn store_completed_read(&mut self) {
        if let Some(data) = advance_read(&mut self.io_state, &mut self.blk_io) {
            self.completed_sector = Some(data);
        }
    }

    pub fn store_completed_write(&mut self) {
        if advance_write(&mut self.io_state, &mut self.blk_io) {
            self.completed_ok = true;
        }
    }

    pub fn handle_blk_driver(&mut self) {
        match self.io_state {
            IoState::Reading { .. } => self.store_completed_read(),
            IoState::Writing { .. } => self.store_completed_write(),
            IoState::Idle => {}
        }
    }

    pub fn io_busy(&self) -> bool {
        matches!(
            self.io_state,
            IoState::Reading { .. } | IoState::Writing { .. }
        )
    }
}
