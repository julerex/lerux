use core::cmp::min;

use lerux_fs::{
    count_entries, decode_dir_entry, decode_superblock, encode_dir_entry, encode_superblock,
    find_by_name, find_free_slot, is_formatted, DirEntry, Superblock, DIR_LBA, SUPERBLOCK_LBA,
};
use lerux_interface_types::{
    FsDirEntry, FsRequest, FsResponse, MAX_FS_DATA, MAX_FS_DIR_LIST, MAX_FS_PATH, SECTOR_SIZE,
};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::log;
use sel4_microkit::{Channel, ChannelSet, Handler, Infallible, MessageInfo};

use lerux_service_async::SingleTask;

use crate::block_io::{read_sector, write_sector, SectorIo, SharedSectorIo, BLK_DRIVER, CLIENT};

#[expect(
    clippy::large_enum_variant,
    reason = "Write job carries inline IPC payload while job is in flight"
)]
enum FsJob {
    None,
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

pub struct HandlerImpl {
    io: SharedSectorIo,
    /// Phase 45: LERUXFS1 format runs as a stackless async task.
    format_task: SingleTask<Result<Superblock, ()>>,
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

impl HandlerImpl {
    pub fn new(block_size: usize) -> HandlerImpl {
        log::info!("lerux-fs: ready (LERUXFS1)");
        HandlerImpl {
            io: SectorIo::shared(block_size),
            format_task: SingleTask::empty(),
            superblock: Superblock::new(),
            dir_sector: [0; SECTOR_SIZE],
            formatted: false,
            fs_job: FsJob::None,
            after_format: None,
            completed: None,
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

    fn poll_format_task(&mut self) -> Option<FsResponse> {
        match self.format_task.run_until_stalled() {
            Some(Ok(sb)) => {
                self.superblock = sb;
                self.formatted = true;
                log::info!("lerux-fs: format/mount done (async)");
                if let Some(next) = self.after_format.take() {
                    self.fs_job = next;
                    return self.advance_fs_job();
                }
                Some(FsResponse::Ok)
            }
            Some(Err(())) => Some(FsResponse::Error),
            None => None,
        }
    }

    fn maybe_format_then(&mut self, next: FsJob) -> Option<FsResponse> {
        if self.formatted {
            self.fs_job = next;
            return self.advance_fs_job();
        }
        self.after_format = Some(next);
        if self.format_task.is_idle() {
            let io = self.io.clone();
            self.format_task
                .spawn(async move { format_leruxfs(io).await });
        }
        self.poll_format_task()
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
                let __tmp = self.io.borrow_mut().poll_read_sector(DIR_LBA);
                if let Some(dir) = __tmp {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(DIR_LBA);
                if let Some(dir) = __tmp {
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
                encode_superblock(&mut self.io.borrow_mut().sector_buf, &self.superblock);
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
                let __w = self.io.borrow_mut().poll_write_sector(DIR_LBA, &dir);
                if __w {
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
                let sector = self.io.borrow_mut().sector_buf;
                let __w = self
                    .io
                    .borrow_mut()
                    .poll_write_sector(SUPERBLOCK_LBA, &sector);
                if __w {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(DIR_LBA);
                if let Some(dir) = __tmp {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(data_lba);
                if let Some(mut sector) = __tmp {
                    let off = offset as usize;
                    let len = data_len as usize;
                    if off >= SECTOR_SIZE
                        || len > MAX_FS_DATA
                        || off.saturating_add(len) > SECTOR_SIZE
                    {
                        return Some(FsResponse::Error);
                    }
                    sector[off..off + len].copy_from_slice(&data[..len]);
                    self.io.borrow_mut().sector_buf = sector;
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
                let sector = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(data_lba, &sector);
                if __w {
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
                let __w = self.io.borrow_mut().poll_write_sector(DIR_LBA, &dir);
                if __w {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(DIR_LBA);
                if let Some(dir) = __tmp {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(data_lba);
                if let Some(sector) = __tmp {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(DIR_LBA);
                if let Some(dir) = __tmp {
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
                let __tmp = self.io.borrow_mut().poll_read_sector(DIR_LBA);
                if let Some(dir) = __tmp {
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

        if self.completed.is_some()
            || !matches!(self.fs_job, FsJob::None)
            || self.format_task.is_running()
        {
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
        if self.format_task.is_running() {
            if let Some(resp) = self.poll_format_task() {
                self.finish_job(resp);
                return self.completed.take().unwrap_or(FsResponse::Pending);
            }
        }
        if let Some(resp) = self.advance_fs_job() {
            self.finish_job(resp);
            return self.completed.take().unwrap_or(FsResponse::Pending);
        }
        if self.io.borrow().io_busy() {
            BLK_DRIVER.notify();
        }
        FsResponse::Pending
    }

    fn handle_blk_driver(&mut self) {
        self.io.borrow_mut().handle_blk_driver();
    }
}

/// Sequential format/mount for LERUXFS1 (Phase 45 stackless async).
async fn format_leruxfs(io: SharedSectorIo) -> Result<Superblock, ()> {
    let sector = read_sector(io.clone(), SUPERBLOCK_LBA).await?;
    if is_formatted(&sector) {
        return decode_superblock(&sector).ok_or(());
    }
    let sb = Superblock::new();
    let mut buf = [0u8; SECTOR_SIZE];
    encode_superblock(&mut buf, &sb);
    write_sector(io.clone(), SUPERBLOCK_LBA, buf).await?;
    let dir = [0u8; SECTOR_SIZE];
    write_sector(io, DIR_LBA, dir).await?;
    Ok(sb)
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT
            && channel != Channel::new(3)
            && channel != Channel::new(5)
            && channel != Channel::new(6)
            && channel != Channel::new(7)
        {
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
            if self.format_task.is_running() {
                if let Some(resp) = self.poll_format_task() {
                    self.finish_job(resp);
                }
            }
        }
        Ok(())
    }
}
