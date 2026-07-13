//! Shared virtio-blk ring client for LERUXFS1 and FAT backends.
//!
//! Phase 45: also exposes stackless async sector helpers ([`read_sector`] /
//! [`write_sector`]) driven from Handler via [`lerux_service_async`].

use alloc::rc::Rc;
#[cfg(not(feature = "backend-fat"))]
use core::cell::RefCell;
use core::task::Poll;

use async_unsync::semaphore::Semaphore;
use lerux_interface_types::SECTOR_SIZE;
use lerux_logging::log;
#[cfg(not(feature = "backend-fat"))]
use lerux_service_async::poll_fn;
use lerux_service_async::WakeCell;
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

    // Fast PCI virtio can complete before we resume from notify; never panic the
    // PD on ring misbehavior — that hangs busy-wait PPC clients forever.
    if io.poll().is_err() {
        log::error!("lerux-fs: blk used-ring poll failed (read)");
        *io_state = IoState::Idle;
        return None;
    }
    match io.poll_read_request(*request_index, buf, None) {
        Ok(Poll::Ready(Ok(()))) => {
            let data = *buf;
            *io_state = IoState::Idle;
            Some(data)
        }
        Ok(Poll::Pending) => None,
        Ok(Poll::Ready(Err(_))) | Err(_) => {
            *io_state = IoState::Idle;
            None
        }
    }
}

fn advance_write(io_state: &mut IoState, io: &mut BlkIo) -> bool {
    let IoState::Writing { request_index } = io_state else {
        return false;
    };

    if io.poll().is_err() {
        log::error!("lerux-fs: blk used-ring poll failed (write)");
        *io_state = IoState::Idle;
        return false;
    }
    match io.poll_write_request(*request_index, None) {
        Ok(Poll::Ready(Ok(()))) => {
            *io_state = IoState::Idle;
            true
        }
        Ok(Poll::Pending) => false,
        Ok(Poll::Ready(Err(_))) | Err(_) => {
            *io_state = IoState::Idle;
            false
        }
    }
}

/// Shared handle for async sector ops (Phase 45; LERUXFS backend only).
#[cfg(not(feature = "backend-fat"))]
pub type SharedSectorIo = Rc<RefCell<SectorIo>>;

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
    /// Woken when a ring completion is observed (async tasks).
    pub wake: WakeCell,
    read_failed: bool,
    write_failed: bool,
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
            wake: WakeCell::new(),
            read_failed: false,
            write_failed: false,
        }
    }

    #[cfg(not(feature = "backend-fat"))]
    pub fn shared(block_size: usize) -> SharedSectorIo {
        Rc::new(RefCell::new(Self::new(block_size)))
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
        // Higher-prio driver may complete during issue_read's notify (x86 PCI).
        // Keep pending_read_lba set so poll_fn observes completed_sector.
        if let Some(data) = advance_read(&mut self.io_state, &mut self.blk_io) {
            self.completed_sector = Some(data);
        }
    }

    pub fn start_write(&mut self, lba: u32, data: &[u8; SECTOR_SIZE]) {
        if !matches!(self.io_state, IoState::Idle) {
            return;
        }
        self.sector_buf.copy_from_slice(data);
        let request_index = issue_write(&mut self.blk_io, lba, &self.sector_buf[..self.block_size]);
        self.io_state = IoState::Writing { request_index };
        self.pending_write_lba = Some(lba);
        // Same sync-completion race as start_read; keep pending_write_lba set.
        if advance_write(&mut self.io_state, &mut self.blk_io) {
            self.completed_ok = true;
        }
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
        if let Some(data) = self.try_advance_read() {
            self.completed_sector = Some(data);
            self.wake.wake();
        } else if matches!(self.io_state, IoState::Idle) {
            // advance_read cleared state on error
            self.read_failed = true;
            self.wake.wake();
        }
    }

    pub fn store_completed_write(&mut self) {
        if self.try_advance_write() {
            self.completed_ok = true;
            self.wake.wake();
        } else if matches!(self.io_state, IoState::Idle) {
            self.write_failed = true;
            self.wake.wake();
        }
    }

    pub fn handle_blk_driver(&mut self) {
        // Always drain the used ring first. Sync completions can land while
        // IoState is still Idle (notify during issue_read before Reading is set).
        if self.blk_io.poll().is_err() {
            log::error!("lerux-fs: blk used-ring poll failed (notify)");
            return;
        }
        match self.io_state {
            IoState::Reading { .. } => self.store_completed_read(),
            IoState::Writing { .. } => self.store_completed_write(),
            IoState::Idle => {
                // Completion may already be in SlotTracker as Complete; the next
                // start_read/poll_fn path observes completed_sector / advance.
            }
        }
    }

    pub fn io_busy(&self) -> bool {
        matches!(
            self.io_state,
            IoState::Reading { .. } | IoState::Writing { .. }
        )
    }

    fn try_advance_read(&mut self) -> Option<[u8; SECTOR_SIZE]> {
        advance_read(&mut self.io_state, &mut self.blk_io)
    }

    fn try_advance_write(&mut self) -> bool {
        advance_write(&mut self.io_state, &mut self.blk_io)
    }
}

/// Async read of one sector (Phase 45). Completes when the block driver notifies.
#[cfg(not(feature = "backend-fat"))]
pub async fn read_sector(io: SharedSectorIo, lba: u32) -> Result<[u8; SECTOR_SIZE], ()> {
    {
        let mut g = io.borrow_mut();
        g.read_failed = false;
        if g.pending_read_lba != Some(lba) {
            if !matches!(g.io_state, IoState::Idle) || g.pending_read_lba.is_some() {
                return Err(());
            }
            g.start_read(lba);
        }
    }
    poll_fn(move |cx| {
        let mut g = io.borrow_mut();
        g.wake.set(cx.waker());
        // Opportunistic progress without notify (same PD edge cases).
        if matches!(g.io_state, IoState::Reading { .. })
            && let Some(data) = g.try_advance_read()
        {
            g.completed_sector = Some(data);
            g.pending_read_lba = None;
            return Poll::Ready(Ok(data));
        }
        if g.pending_read_lba == Some(lba) {
            if let Some(data) = g.completed_sector.take() {
                g.pending_read_lba = None;
                return Poll::Ready(Ok(data));
            }
            if g.read_failed {
                g.read_failed = false;
                g.pending_read_lba = None;
                return Poll::Ready(Err(()));
            }
        }
        Poll::Pending
    })
    .await
}

/// Async write of one sector (Phase 45).
#[cfg(not(feature = "backend-fat"))]
pub async fn write_sector(io: SharedSectorIo, lba: u32, data: [u8; SECTOR_SIZE]) -> Result<(), ()> {
    {
        let mut g = io.borrow_mut();
        g.write_failed = false;
        if g.pending_write_lba != Some(lba) {
            if !matches!(g.io_state, IoState::Idle) || g.pending_write_lba.is_some() {
                return Err(());
            }
            g.start_write(lba, &data);
        }
    }
    poll_fn(move |cx| {
        let mut g = io.borrow_mut();
        g.wake.set(cx.waker());
        if matches!(g.io_state, IoState::Writing { .. }) && g.try_advance_write() {
            g.pending_write_lba = None;
            g.completed_ok = false;
            return Poll::Ready(Ok(()));
        }
        if g.pending_write_lba == Some(lba) {
            if g.completed_ok {
                g.completed_ok = false;
                g.pending_write_lba = None;
                return Poll::Ready(Ok(()));
            }
            if g.write_failed {
                g.write_failed = false;
                g.pending_write_lba = None;
                return Poll::Ready(Err(()));
            }
        }
        Poll::Pending
    })
    .await
}
