#![no_std]
#![no_main]

extern crate alloc;

use alloc::rc::Rc;
use core::{cmp::min, task::Poll};

use async_unsync::semaphore::Semaphore;
use lerux_fs::{
    count_entries, decode_dir_entry, decode_superblock, encode_dir_entry, encode_superblock,
    find_by_name, find_free_slot, is_formatted, DirEntry, Superblock, DIR_LBA, SUPERBLOCK_LBA,
};
use lerux_interface_types::{
    FsDirEntry, FsRequest, FsResponse, MAX_FS_DATA, MAX_FS_DIR_LIST, MAX_FS_PATH, SECTOR_SIZE,
};
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

#[expect(clippy::large_enum_variant)]
enum IoState {
    Idle,
    Reading {
        request_index: usize,
        buf: [u8; SECTOR_SIZE],
    },
    Writing {
        request_index: usize,
    },
}

#[expect(
    clippy::large_enum_variant,
    reason = "Write job carries inline IPC payload while job is in flight"
)]
enum FsJob {
    None,
    Format {
        step: u8,
    },
    Open {
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
    },
    Create {
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
        slot: u8,
        data_lba: u32,
    },
    Write {
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        data_lba: u32,
        new_size: u32,
    },
    Read {
        handle: u8,
        offset: u32,
        len: u16,
        step: u8,
        data_lba: u32,
    },
    Stat {
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
    },
    ListDir {
        step: u8,
    },
}

struct HandlerImpl {
    blk_io: BlkIo,
    io_state: IoState,
    block_size: usize,
    completed_sector: Option<[u8; SECTOR_SIZE]>,
    completed_ok: bool,
    pending_read_lba: Option<u32>,
    pending_write_lba: Option<u32>,
    sector_buf: [u8; SECTOR_SIZE],
    superblock: Superblock,
    dir_sector: [u8; SECTOR_SIZE],
    formatted: bool,
    fs_job: FsJob,
    after_format: Option<FsJob>,
    completed: Option<FsResponse>,
}

fn path_slice(path: &[u8; MAX_FS_PATH], path_len: u8) -> &[u8] {
    &path[..path_len as usize]
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

impl HandlerImpl {
    fn start_read(&mut self, lba: u32) {
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

    fn start_write(&mut self, lba: u32, data: &[u8; SECTOR_SIZE]) {
        if !matches!(self.io_state, IoState::Idle) {
            return;
        }
        self.sector_buf.copy_from_slice(data);
        let request_index = issue_write(&mut self.blk_io, lba, &self.sector_buf[..self.block_size]);
        self.io_state = IoState::Writing { request_index };
        self.pending_write_lba = Some(lba);
    }

    fn poll_read_sector(&mut self, lba: u32) -> Option<[u8; SECTOR_SIZE]> {
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

    fn poll_write_sector(&mut self, lba: u32, data: &[u8; SECTOR_SIZE]) -> bool {
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

    fn store_completed_read(&mut self) {
        if let Some(data) = advance_read(&mut self.io_state, &mut self.blk_io) {
            self.completed_sector = Some(data);
        }
    }

    fn store_completed_write(&mut self) {
        if advance_write(&mut self.io_state, &mut self.blk_io) {
            self.completed_ok = true;
        }
    }

    fn begin_open(&mut self, path: [u8; MAX_FS_PATH], path_len: u8) {
        self.fs_job = FsJob::Open {
            path,
            path_len,
            step: 0,
        };
    }

    fn begin_create(&mut self, path: [u8; MAX_FS_PATH], path_len: u8) {
        self.fs_job = FsJob::Create {
            path,
            path_len,
            step: 0,
            slot: 0,
            data_lba: 0,
        };
    }

    fn begin_write(&mut self, handle: u8, offset: u32, data: [u8; MAX_FS_DATA], data_len: u16) {
        self.fs_job = FsJob::Write {
            handle,
            offset,
            data,
            data_len,
            step: 0,
            data_lba: 0,
            new_size: 0,
        };
    }

    fn begin_read(&mut self, handle: u8, offset: u32, len: u16) {
        self.fs_job = FsJob::Read {
            handle,
            offset,
            len,
            step: 0,
            data_lba: 0,
        };
    }

    fn begin_stat(&mut self, path: [u8; MAX_FS_PATH], path_len: u8) {
        self.fs_job = FsJob::Stat {
            path,
            path_len,
            step: 0,
        };
    }

    fn begin_list_dir(&mut self) {
        self.fs_job = FsJob::ListDir { step: 0 };
    }

    fn finish_job(&mut self, response: FsResponse) {
        self.fs_job = FsJob::None;
        self.completed = Some(response);
    }

    fn advance_fs_job(&mut self) -> Option<FsResponse> {
        match core::mem::replace(&mut self.fs_job, FsJob::None) {
            FsJob::None => None,
            FsJob::Format { step } => self.advance_format(step),
            FsJob::Open {
                path,
                path_len,
                step,
            } => self.advance_open(path, path_len, step),
            FsJob::Create {
                path,
                path_len,
                step,
                slot,
                data_lba,
            } => self.advance_create(path, path_len, step, slot, data_lba),
            FsJob::Write {
                handle,
                offset,
                data,
                data_len,
                step,
                data_lba,
                new_size,
            } => self.advance_write(handle, offset, data, data_len, step, data_lba, new_size),
            FsJob::Read {
                handle,
                offset,
                len,
                step,
                data_lba,
            } => self.advance_read(handle, offset, len, step, data_lba),
            FsJob::Stat {
                path,
                path_len,
                step,
            } => self.advance_stat(path, path_len, step),
            FsJob::ListDir { step } => self.advance_list_dir(step),
        }
    }

    fn restore_job(&mut self, job: FsJob) {
        self.fs_job = job;
    }

    fn advance_format(&mut self, step: u8) -> Option<FsResponse> {
        let job = FsJob::Format { step };
        match step {
            0 => {
                if let Some(sector) = self.poll_read_sector(SUPERBLOCK_LBA) {
                    if is_formatted(&sector) {
                        self.superblock = decode_superblock(&sector).unwrap();
                        self.formatted = true;
                        self.fs_job = FsJob::None;
                        return Some(FsResponse::Ok);
                    }
                    self.superblock = Superblock::new();
                    encode_superblock(&mut self.sector_buf, &self.superblock);
                    self.fs_job = FsJob::Format { step: 1 };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            1 => {
                let sector = self.sector_buf;
                if self.poll_write_sector(SUPERBLOCK_LBA, &sector) {
                    self.dir_sector.fill(0);
                    self.fs_job = FsJob::Format { step: 2 };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            2 => {
                let dir = self.dir_sector;
                if self.poll_write_sector(DIR_LBA, &dir) {
                    self.formatted = true;
                    if let Some(next) = self.after_format.take() {
                        self.fs_job = next;
                        return self.advance_fs_job();
                    }
                    self.fs_job = FsJob::None;
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn maybe_format_then(&mut self, next: FsJob) -> Option<FsResponse> {
        if self.formatted {
            self.fs_job = next;
            return self.advance_fs_job();
        }
        self.after_format = Some(next);
        if matches!(self.fs_job, FsJob::None) {
            self.fs_job = FsJob::Format { step: 0 };
        }
        self.advance_fs_job()
    }

    fn advance_open(
        &mut self,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
    ) -> Option<FsResponse> {
        let job = FsJob::Open {
            path,
            path_len,
            step,
        };
        let name = path_slice(&path, path_len);
        match step {
            0 => self.maybe_format_then(FsJob::Open {
                path,
                path_len,
                step: 1,
            }),
            1 => {
                if let Some(dir) = self.poll_read_sector(DIR_LBA) {
                    self.dir_sector = dir;
                    if let Some(index) = find_by_name(&self.dir_sector, name) {
                        return Some(FsResponse::Handle { id: index as u8 });
                    }
                    return Some(FsResponse::Error);
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn advance_create(
        &mut self,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
        slot: u8,
        data_lba: u32,
    ) -> Option<FsResponse> {
        let job = FsJob::Create {
            path,
            path_len,
            step,
            slot,
            data_lba,
        };
        let name = path_slice(&path, path_len);
        match step {
            0 => self.maybe_format_then(FsJob::Create {
                path,
                path_len,
                step: 1,
                slot,
                data_lba,
            }),
            1 => {
                if let Some(dir) = self.poll_read_sector(DIR_LBA) {
                    self.dir_sector = dir;
                    if find_by_name(&self.dir_sector, name).is_some() {
                        return Some(FsResponse::Error);
                    }
                    let Some(index) = find_free_slot(&self.dir_sector) else {
                        return Some(FsResponse::Error);
                    };
                    let lba = self.superblock.next_data_lba;
                    self.fs_job = FsJob::Create {
                        path,
                        path_len,
                        step: 2,
                        slot: index as u8,
                        data_lba: lba,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            2 => {
                let mut entry = DirEntry::empty();
                let name_len = name.len().min(lerux_fs::NAME_LEN);
                entry.name[..name_len].copy_from_slice(&name[..name_len]);
                entry.name_len = name_len as u8;
                entry.data_lba = data_lba;
                entry.size = 0;
                encode_dir_entry(&mut self.dir_sector, slot as usize, &entry);
                self.superblock.file_count = self.superblock.file_count.saturating_add(1);
                self.superblock.next_data_lba = data_lba.saturating_add(1);
                encode_superblock(&mut self.sector_buf, &self.superblock);
                self.fs_job = FsJob::Create {
                    path,
                    path_len,
                    step: 3,
                    slot,
                    data_lba,
                };
                self.advance_fs_job()
            }
            3 => {
                let dir = self.dir_sector;
                if self.poll_write_sector(DIR_LBA, &dir) {
                    self.fs_job = FsJob::Create {
                        path,
                        path_len,
                        step: 4,
                        slot,
                        data_lba,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            4 => {
                let sector = self.sector_buf;
                if self.poll_write_sector(SUPERBLOCK_LBA, &sector) {
                    return Some(FsResponse::Handle { id: slot });
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "async write job threads path, payload, and block metadata through poll stages"
    )]
    fn advance_write(
        &mut self,
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        data_lba: u32,
        new_size: u32,
    ) -> Option<FsResponse> {
        let job = FsJob::Write {
            handle,
            offset,
            data,
            data_len,
            step,
            data_lba,
            new_size,
        };
        match step {
            0 => {
                if let Some(dir) = self.poll_read_sector(DIR_LBA) {
                    self.dir_sector = dir;
                    let entry = decode_dir_entry(&self.dir_sector, handle as usize);
                    if entry.is_free() {
                        return Some(FsResponse::Error);
                    }
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 1,
                        data_lba: entry.data_lba,
                        new_size: entry.size,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            1 => {
                if let Some(mut sector) = self.poll_read_sector(data_lba) {
                    let off = offset as usize;
                    let len = data_len as usize;
                    if off >= SECTOR_SIZE
                        || len > MAX_FS_DATA
                        || off.saturating_add(len) > SECTOR_SIZE
                    {
                        return Some(FsResponse::Error);
                    }
                    sector[off..off + len].copy_from_slice(&data[..len]);
                    self.sector_buf = sector;
                    let end = (off + len) as u32;
                    let size = new_size.max(end);
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 2,
                        data_lba,
                        new_size: size,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            2 => {
                let sector = self.sector_buf;
                if self.poll_write_sector(data_lba, &sector) {
                    let mut entry = decode_dir_entry(&self.dir_sector, handle as usize);
                    entry.size = new_size;
                    encode_dir_entry(&mut self.dir_sector, handle as usize, &entry);
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 3,
                        data_lba,
                        new_size,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            3 => {
                let dir = self.dir_sector;
                if self.poll_write_sector(DIR_LBA, &dir) {
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn advance_read(
        &mut self,
        handle: u8,
        offset: u32,
        len: u16,
        step: u8,
        data_lba: u32,
    ) -> Option<FsResponse> {
        let job = FsJob::Read {
            handle,
            offset,
            len,
            step,
            data_lba,
        };
        match step {
            0 => {
                if let Some(dir) = self.poll_read_sector(DIR_LBA) {
                    self.dir_sector = dir;
                    let entry = decode_dir_entry(&self.dir_sector, handle as usize);
                    if entry.is_free() {
                        return Some(FsResponse::Error);
                    }
                    self.fs_job = FsJob::Read {
                        handle,
                        offset,
                        len,
                        step: 1,
                        data_lba: entry.data_lba,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            1 => {
                if let Some(sector) = self.poll_read_sector(data_lba) {
                    let entry = decode_dir_entry(&self.dir_sector, handle as usize);
                    let off = offset as usize;
                    let want = len as usize;
                    if off >= SECTOR_SIZE {
                        return Some(FsResponse::Error);
                    }
                    let avail = min(entry.size as usize, SECTOR_SIZE).saturating_sub(off);
                    let copy_len = min(want, avail).min(MAX_FS_DATA);
                    let mut out = [0u8; MAX_FS_DATA];
                    out[..copy_len].copy_from_slice(&sector[off..off + copy_len]);
                    return Some(FsResponse::Data {
                        data_len: copy_len as u16,
                        data: out,
                    });
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn advance_stat(
        &mut self,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
    ) -> Option<FsResponse> {
        let job = FsJob::Stat {
            path,
            path_len,
            step,
        };
        let name = path_slice(&path, path_len);
        match step {
            0 => {
                if !self.formatted {
                    return Some(FsResponse::Error);
                }
                if let Some(dir) = self.poll_read_sector(DIR_LBA) {
                    self.dir_sector = dir;
                    if let Some(index) = find_by_name(&self.dir_sector, name) {
                        let entry = decode_dir_entry(&self.dir_sector, index);
                        return Some(FsResponse::Stat { size: entry.size });
                    }
                    return Some(FsResponse::Error);
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn advance_list_dir(&mut self, step: u8) -> Option<FsResponse> {
        let job = FsJob::ListDir { step };
        match step {
            0 => {
                if !self.formatted {
                    return Some(FsResponse::DirList {
                        count: 0,
                        entries: [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST],
                    });
                }
                if let Some(dir) = self.poll_read_sector(DIR_LBA) {
                    self.dir_sector = dir;
                    let total = count_entries(&self.dir_sector);
                    let mut entries = [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST];
                    let mut out_count = 0u8;
                    for index in 0..lerux_fs::MAX_ENTRIES {
                        let entry = decode_dir_entry(&self.dir_sector, index);
                        if entry.is_free() {
                            continue;
                        }
                        if (out_count as usize) < MAX_FS_DIR_LIST {
                            entries[out_count as usize] =
                                FsDirEntry::from_name_size(entry.name_slice(), entry.size);
                            out_count += 1;
                        }
                    }
                    let _ = total;
                    return Some(FsResponse::DirList {
                        count: out_count,
                        entries,
                    });
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn handle_request(&mut self, req: FsRequest) -> FsResponse {
        if matches!(req, FsRequest::Poll) {
            return self.handle_poll();
        }

        if self.completed.is_some() || !matches!(self.fs_job, FsJob::None) {
            return FsResponse::Pending;
        }

        match req {
            FsRequest::Open { path_len, path } => {
                self.begin_open(path, path_len);
            }
            FsRequest::Create { path_len, path } => {
                self.begin_create(path, path_len);
            }
            FsRequest::Write {
                handle,
                offset,
                data_len,
                data,
            } => {
                self.begin_write(handle, offset, data, data_len);
            }
            FsRequest::Read {
                handle,
                offset,
                len,
            } => {
                self.begin_read(handle, offset, len);
            }
            FsRequest::Stat { path_len, path } => {
                self.begin_stat(path, path_len);
            }
            FsRequest::ListDir => {
                self.begin_list_dir();
            }
            FsRequest::Poll => return self.handle_poll(),
        }

        if let Some(resp) = self.advance_fs_job() {
            self.finish_job(resp);
            return self.completed.take().unwrap_or(FsResponse::Pending);
        }
        FsResponse::Pending
    }

    fn handle_poll(&mut self) -> FsResponse {
        if let Some(resp) = self.completed.take() {
            return resp;
        }
        if let Some(resp) = self.advance_fs_job() {
            self.finish_job(resp);
            return self.completed.take().unwrap_or(FsResponse::Pending);
        }
        if matches!(
            self.io_state,
            IoState::Reading { .. } | IoState::Writing { .. }
        ) {
            BLK_DRIVER.notify();
        }
        FsResponse::Pending
    }

    fn handle_blk_driver(&mut self) {
        match self.io_state {
            IoState::Reading { .. } => self.store_completed_read(),
            IoState::Writing { .. } => self.store_completed_write(),
            IoState::Idle => {}
        }
    }
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    let mut blk = BlockClient::new(BLK_DRIVER);
    let block_size = blk.get_block_size().unwrap();
    let num_blocks = blk.get_num_blocks().unwrap();
    log::info!("virtio-blk: {num_blocks} blocks x {block_size} bytes");
    log::info!("lerux-fs: ready");
    HandlerImpl {
        blk_io: create_blk_io(),
        io_state: IoState::Idle,
        block_size,
        completed_sector: None,
        completed_ok: false,
        pending_read_lba: None,
        pending_write_lba: None,
        sector_buf: [0; SECTOR_SIZE],
        superblock: Superblock::new(),
        dir_sector: [0; SECTOR_SIZE],
        formatted: false,
        fs_job: FsJob::None,
        after_format: None,
        completed: None,
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT && channel != Channel::new(3) {
            // Channel 3 is used by shell on workstation (multi-client fs)
            unreachable!("unexpected fs client");
        }

        Ok(match recv::<FsRequest>(msg_info) {
            Ok(req) => send(self.handle_request(req)),
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
