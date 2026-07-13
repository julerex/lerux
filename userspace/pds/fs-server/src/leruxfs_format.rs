//! LERUXFS2 backend: hierarchical dirs, multi-sector files, free-map allocation.

use core::cmp::min;

use lerux_fs::{
    alloc_contiguous, count_entries, count_free_blocks, decode_dir_entry, decode_superblock,
    dir_is_empty, encode_dir_entry, encode_free_map_fresh, encode_superblock, file_lba_for_offset,
    find_by_name, find_free_slot, free_contiguous, is_formatted, is_legacy_v1, make_entry,
    sector_offset, split_path, DirEntry, PathParts, Superblock, DATA_START_LBA, FREE_MAP_LBA,
    MAX_ENTRIES, MAX_FILE_SECTORS, ROOT_DIR_LBA, SUPERBLOCK_LBA,
};
use lerux_interface_types::{
    FsDirEntry, FsRequest, FsResponse, MAX_FS_DATA, MAX_FS_DIR_LIST, MAX_FS_PATH, SECTOR_SIZE,
};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::log;
use sel4_microkit::{Channel, ChannelSet, Handler, Infallible, MessageInfo};

use lerux_service_async::SingleTask;

use crate::block_io::{read_sector, write_sector, SectorIo, SharedSectorIo, BLK_DRIVER, CLIENT};

const MAX_OPEN: usize = 8;

#[derive(Clone, Copy)]
struct OpenFile {
    in_use: bool,
    dir_lba: u32,
    slot: u8,
    first_lba: u32,
    size: u32,
    is_dir: bool,
}

impl OpenFile {
    const fn empty() -> Self {
        Self {
            in_use: false,
            dir_lba: 0,
            slot: 0,
            first_lba: 0,
            size: 0,
            is_dir: false,
        }
    }

    fn file_sectors(&self) -> u32 {
        if self.is_dir || !self.in_use {
            return 0;
        }
        if self.size == 0 {
            return 1;
        }
        self.size
            .div_ceil(SECTOR_SIZE as u32)
            .clamp(1, MAX_FILE_SECTORS)
    }
}

#[derive(Clone, Copy)]
enum PathOp {
    Open,
    Create,
    Stat,
    ListDir,
    Mkdir,
    Unlink,
    RenameFrom,
}

enum FsJob {
    None,
    /// Resolve path then dispatch to a concrete op.
    Path {
        op: PathOp,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        /// Rename destination (RenameFrom only).
        to_path: [u8; MAX_FS_PATH],
        to_path_len: u8,
        parts: PathParts,
        /// Component index being resolved (0..parts.count).
        comp_i: u8,
        /// Directory LBA for the current component's parent.
        dir_lba: u32,
        step: u8,
        /// Cached parent dir sector for final component ops.
        /// After walk: dir_lba is parent of leaf; leaf name is last component.
        slot: u8,
        entry_lba: u32,
        entry_size: u32,
        entry_flags: u8,
        /// Rename: destination parent state
        to_parts: PathParts,
        to_comp_i: u8,
        to_dir_lba: u32,
        to_slot: u8,
    },
    Write {
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        first_lba: u32,
        size: u32,
        dir_lba: u32,
        slot: u8,
        /// Bytes of this Write already applied.
        done: u16,
        /// Target size after write.
        new_size: u32,
        /// Sectors allocated for the file (may grow).
        n_sectors: u32,
        freemap_dirty: bool,
    },
    Read {
        handle: u8,
        offset: u32,
        len: u16,
        step: u8,
        first_lba: u32,
        size: u32,
        done: u16,
        out: [u8; MAX_FS_DATA],
    },
}

pub struct HandlerImpl {
    io: SharedSectorIo,
    format_task: SingleTask<Result<(Superblock, [u8; SECTOR_SIZE]), ()>>,
    superblock: Superblock,
    free_map: [u8; SECTOR_SIZE],
    freemap_dirty: bool,
    formatted: bool,
    open: [OpenFile; MAX_OPEN],
    fs_job: FsJob,
    after_format: Option<FsJob>,
    completed: Option<FsResponse>,
    /// Client that owns the in-flight async operation (or pending completion).
    active_client: Option<Channel>,
}

fn path_slice(path: &[u8; MAX_FS_PATH], path_len: u8) -> &[u8] {
    &path[..path_len.min(MAX_FS_PATH as u8) as usize]
}

impl HandlerImpl {
    pub fn new(block_size: usize) -> HandlerImpl {
        log::info!("lerux-fs: ready (LERUXFS2)");
        HandlerImpl {
            io: SectorIo::shared(block_size),
            format_task: SingleTask::empty(),
            superblock: Superblock::new(),
            free_map: [0; SECTOR_SIZE],
            freemap_dirty: false,
            formatted: false,
            open: [OpenFile::empty(); MAX_OPEN],
            fs_job: FsJob::None,
            after_format: None,
            completed: None,
            active_client: None,
        }
    }

    fn is_client(channel: Channel) -> bool {
        channel == CLIENT
            || channel == Channel::new(3)
            || channel == Channel::new(5)
            || channel == Channel::new(6)
            || channel == Channel::new(7)
            || channel == Channel::new(8)
    }

    /// Reserve this client for an async op. Returns false when another client owns the stack
    /// or this client still has an undelivered completion.
    fn begin_async(&mut self, channel: Channel) -> bool {
        if self.completed.is_some() {
            return false;
        }
        let busy = !matches!(self.fs_job, FsJob::None) || self.format_task.is_running();
        if busy && self.active_client != Some(channel) {
            return false;
        }
        self.active_client = Some(channel);
        true
    }

    fn finish_async(&mut self) {
        self.active_client = None;
    }

    fn take_completed(&mut self, channel: Channel) -> Option<FsResponse> {
        if self.active_client != Some(channel) {
            return None;
        }
        let resp = self.completed.take()?;
        self.finish_async();
        Some(resp)
    }

    fn sync_response(&mut self, resp: FsResponse) -> FsResponse {
        if !matches!(resp, FsResponse::Pending) {
            self.finish_async();
        }
        resp
    }

    fn begin_path_op(
        &mut self,
        op: PathOp,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        to_path: [u8; MAX_FS_PATH],
        to_path_len: u8,
    ) {
        self.fs_job = FsJob::Path {
            op,
            path,
            path_len,
            to_path,
            to_path_len,
            parts: PathParts::empty(),
            comp_i: 0,
            dir_lba: ROOT_DIR_LBA,
            step: 0,
            slot: 0,
            entry_lba: 0,
            entry_size: 0,
            entry_flags: 0,
            to_parts: PathParts::empty(),
            to_comp_i: 0,
            to_dir_lba: ROOT_DIR_LBA,
            to_slot: 0,
        };
    }

    fn begin_write(&mut self, handle: u8, offset: u32, data: [u8; MAX_FS_DATA], data_len: u16) {
        self.fs_job = FsJob::Write {
            handle,
            offset,
            data,
            data_len,
            step: 0,
            first_lba: 0,
            size: 0,
            dir_lba: 0,
            slot: 0,
            done: 0,
            new_size: 0,
            n_sectors: 0,
            freemap_dirty: false,
        };
    }

    fn begin_read(&mut self, handle: u8, offset: u32, len: u16) {
        self.fs_job = FsJob::Read {
            handle,
            offset,
            len,
            step: 0,
            first_lba: 0,
            size: 0,
            done: 0,
            out: [0; MAX_FS_DATA],
        };
    }

    fn finish_job(&mut self, response: FsResponse) {
        self.fs_job = FsJob::None;
        self.completed = Some(response);
    }

    fn alloc_handle(&mut self, of: OpenFile) -> Option<u8> {
        for (i, slot) in self.open.iter_mut().enumerate() {
            if !slot.in_use {
                *slot = of;
                slot.in_use = true;
                return Some(i as u8);
            }
        }
        None
    }

    fn restore_job(&mut self, job: FsJob) {
        self.fs_job = job;
    }

    fn advance_fs_job(&mut self) -> Option<FsResponse> {
        match core::mem::replace(&mut self.fs_job, FsJob::None) {
            FsJob::None => None,
            FsJob::Path {
                op,
                path,
                path_len,
                to_path,
                to_path_len,
                parts,
                comp_i,
                dir_lba,
                step,
                slot,
                entry_lba,
                entry_size,
                entry_flags,
                to_parts,
                to_comp_i,
                to_dir_lba,
                to_slot,
            } => self.advance_path(
                op,
                path,
                path_len,
                to_path,
                to_path_len,
                parts,
                comp_i,
                dir_lba,
                step,
                slot,
                entry_lba,
                entry_size,
                entry_flags,
                to_parts,
                to_comp_i,
                to_dir_lba,
                to_slot,
            ),
            FsJob::Write {
                handle,
                offset,
                data,
                data_len,
                step,
                first_lba,
                size,
                dir_lba,
                slot,
                done,
                new_size,
                n_sectors,
                freemap_dirty,
            } => self.advance_write(
                handle,
                offset,
                data,
                data_len,
                step,
                first_lba,
                size,
                dir_lba,
                slot,
                done,
                new_size,
                n_sectors,
                freemap_dirty,
            ),
            FsJob::Read {
                handle,
                offset,
                len,
                step,
                first_lba,
                size,
                done,
                out,
            } => self.advance_read(handle, offset, len, step, first_lba, size, done, out),
        }
    }

    fn poll_format_task(&mut self) -> Option<FsResponse> {
        match self.format_task.run_until_stalled() {
            Some(Ok((sb, map))) => {
                self.superblock = sb;
                self.free_map = map;
                self.freemap_dirty = false;
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

    #[expect(
        clippy::too_many_arguments,
        reason = "path job threads walk state through poll stages"
    )]
    fn advance_path(
        &mut self,
        op: PathOp,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        to_path: [u8; MAX_FS_PATH],
        to_path_len: u8,
        mut parts: PathParts,
        mut comp_i: u8,
        mut dir_lba: u32,
        step: u8,
        mut slot: u8,
        mut entry_lba: u32,
        mut entry_size: u32,
        mut entry_flags: u8,
        mut to_parts: PathParts,
        mut to_comp_i: u8,
        mut to_dir_lba: u32,
        mut to_slot: u8,
    ) -> Option<FsResponse> {
        let job = |parts: PathParts,
                   comp_i: u8,
                   dir_lba: u32,
                   step: u8,
                   slot: u8,
                   entry_lba: u32,
                   entry_size: u32,
                   entry_flags: u8,
                   to_parts: PathParts,
                   to_comp_i: u8,
                   to_dir_lba: u32,
                   to_slot: u8| FsJob::Path {
            op,
            path,
            path_len,
            to_path,
            to_path_len,
            parts,
            comp_i,
            dir_lba,
            step,
            slot,
            entry_lba,
            entry_size,
            entry_flags,
            to_parts,
            to_comp_i,
            to_dir_lba,
            to_slot,
        };

        match step {
            // Ensure formatted, parse path.
            0 => {
                let raw = path_slice(&path, path_len);
                let Ok(p) = split_path(raw) else {
                    return Some(FsResponse::Error);
                };
                parts = p;
                if matches!(op, PathOp::RenameFrom) {
                    let Ok(tp) = split_path(path_slice(&to_path, to_path_len)) else {
                        return Some(FsResponse::Error);
                    };
                    to_parts = tp;
                    if to_parts.count == 0 {
                        return Some(FsResponse::Error);
                    }
                }
                // ListDir / Stat on root: no leaf.
                if parts.count == 0 {
                    match op {
                        PathOp::ListDir => {
                            dir_lba = self.superblock.root_dir_lba;
                            return self.maybe_format_then(job(
                                parts, 0, dir_lba, 10, 0, 0, 0, 0, to_parts, 0, 0, 0,
                            ));
                        }
                        PathOp::Stat => {
                            return self.maybe_format_then(job(
                                parts, 0, 0, 11, 0, 0, 0, 0, to_parts, 0, 0, 0,
                            ));
                        }
                        PathOp::Open | PathOp::Create | PathOp::Mkdir | PathOp::Unlink => {
                            return Some(FsResponse::Error);
                        }
                        PathOp::RenameFrom => return Some(FsResponse::Error),
                    }
                }
                dir_lba = self.superblock.root_dir_lba;
                comp_i = 0;
                self.maybe_format_then(job(
                    parts,
                    comp_i,
                    dir_lba,
                    1,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ))
            }
            // Walk intermediates (auto-create dirs for Create/Mkdir).
            1 => {
                let Some(name) = parts.component(comp_i as usize) else {
                    return Some(FsResponse::Error);
                };
                let last = comp_i + 1 >= parts.count;
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(dir) = __tmp {
                    if last {
                        // Leaf in parent `dir_lba`.
                        match op {
                            PathOp::Open => {
                                let Some(idx) = find_by_name(&dir, name) else {
                                    return Some(FsResponse::Error);
                                };
                                let e = decode_dir_entry(&dir, idx);
                                if e.is_dir() {
                                    return Some(FsResponse::Error);
                                }
                                let id = self.alloc_handle(OpenFile {
                                    in_use: true,
                                    dir_lba,
                                    slot: idx as u8,
                                    first_lba: e.first_lba,
                                    size: e.size,
                                    is_dir: false,
                                });
                                return Some(match id {
                                    Some(id) => FsResponse::Handle { id },
                                    None => FsResponse::Error,
                                });
                            }
                            PathOp::Stat => {
                                let Some(idx) = find_by_name(&dir, name) else {
                                    return Some(FsResponse::Error);
                                };
                                let e = decode_dir_entry(&dir, idx);
                                return Some(FsResponse::Stat {
                                    size: e.size,
                                    is_dir: e.is_dir(),
                                });
                            }
                            PathOp::ListDir => {
                                let Some(idx) = find_by_name(&dir, name) else {
                                    return Some(FsResponse::Error);
                                };
                                let e = decode_dir_entry(&dir, idx);
                                if !e.is_dir() {
                                    return Some(FsResponse::Error);
                                }
                                dir_lba = e.first_lba;
                                return self.continue_path(job(
                                    parts, comp_i, dir_lba, 10, 0, 0, 0, 0, to_parts, to_comp_i,
                                    to_dir_lba, to_slot,
                                ));
                            }
                            PathOp::Create => {
                                if find_by_name(&dir, name).is_some() {
                                    return Some(FsResponse::Error);
                                }
                                let Some(idx) = find_free_slot(&dir) else {
                                    return Some(FsResponse::Error);
                                };
                                // Cache dir sector in io buffer area via write path.
                                self.io.borrow_mut().sector_buf = dir;
                                slot = idx as u8;
                                return self.continue_path(job(
                                    parts, comp_i, dir_lba, 2, // create: alloc data
                                    slot, 0, 0, 0, to_parts, to_comp_i, to_dir_lba, to_slot,
                                ));
                            }
                            PathOp::Mkdir => {
                                if find_by_name(&dir, name).is_some() {
                                    return Some(FsResponse::Error);
                                }
                                let Some(idx) = find_free_slot(&dir) else {
                                    return Some(FsResponse::Error);
                                };
                                self.io.borrow_mut().sector_buf = dir;
                                slot = idx as u8;
                                return self.continue_path(job(
                                    parts, comp_i, dir_lba, 4, // mkdir: alloc dir sector
                                    slot, 0, 0, 0, to_parts, to_comp_i, to_dir_lba, to_slot,
                                ));
                            }
                            PathOp::Unlink => {
                                let Some(idx) = find_by_name(&dir, name) else {
                                    return Some(FsResponse::Error);
                                };
                                let e = decode_dir_entry(&dir, idx);
                                self.io.borrow_mut().sector_buf = dir;
                                slot = idx as u8;
                                entry_lba = e.first_lba;
                                entry_size = e.size;
                                entry_flags = e.flags;
                                if e.is_dir() {
                                    return self.continue_path(job(
                                        parts,
                                        comp_i,
                                        dir_lba,
                                        6, // check empty dir
                                        slot,
                                        entry_lba,
                                        entry_size,
                                        entry_flags,
                                        to_parts,
                                        to_comp_i,
                                        to_dir_lba,
                                        to_slot,
                                    ));
                                }
                                return self.continue_path(job(
                                    parts,
                                    comp_i,
                                    dir_lba,
                                    7, // free file + clear
                                    slot,
                                    entry_lba,
                                    entry_size,
                                    entry_flags,
                                    to_parts,
                                    to_comp_i,
                                    to_dir_lba,
                                    to_slot,
                                ));
                            }
                            PathOp::RenameFrom => {
                                let Some(idx) = find_by_name(&dir, name) else {
                                    return Some(FsResponse::Error);
                                };
                                let e = decode_dir_entry(&dir, idx);
                                self.io.borrow_mut().sector_buf = dir;
                                slot = idx as u8;
                                entry_lba = e.first_lba;
                                entry_size = e.size;
                                entry_flags = e.flags;
                                // Walk destination parent.
                                to_dir_lba = self.superblock.root_dir_lba;
                                to_comp_i = 0;
                                return self.continue_path(job(
                                    parts,
                                    comp_i,
                                    dir_lba,
                                    20,
                                    slot,
                                    entry_lba,
                                    entry_size,
                                    entry_flags,
                                    to_parts,
                                    to_comp_i,
                                    to_dir_lba,
                                    to_slot,
                                ));
                            }
                        }
                    }
                    // Intermediate component.
                    match find_by_name(&dir, name) {
                        Some(idx) => {
                            let e = decode_dir_entry(&dir, idx);
                            if !e.is_dir() {
                                return Some(FsResponse::Error);
                            }
                            dir_lba = e.first_lba;
                            comp_i += 1;
                            return self.continue_path(job(
                                parts,
                                comp_i,
                                dir_lba,
                                1,
                                slot,
                                entry_lba,
                                entry_size,
                                entry_flags,
                                to_parts,
                                to_comp_i,
                                to_dir_lba,
                                to_slot,
                            ));
                        }
                        None => {
                            // Auto-create parent dirs for Create / Mkdir only.
                            if !matches!(op, PathOp::Create | PathOp::Mkdir) {
                                return Some(FsResponse::Error);
                            }
                            let Some(idx) = find_free_slot(&dir) else {
                                return Some(FsResponse::Error);
                            };
                            self.io.borrow_mut().sector_buf = dir;
                            slot = idx as u8;
                            // step 12: alloc intermediate dir
                            return self.continue_path(job(
                                parts,
                                comp_i,
                                dir_lba,
                                12,
                                slot,
                                entry_lba,
                                entry_size,
                                entry_flags,
                                to_parts,
                                to_comp_i,
                                to_dir_lba,
                                to_slot,
                            ));
                        }
                    }
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Create file: allocate one data sector.
            2 => {
                let Some(data_lba) = alloc_contiguous(&mut self.free_map, &self.superblock, 1)
                else {
                    return Some(FsResponse::Error);
                };
                self.freemap_dirty = true;
                entry_lba = data_lba;
                // zero data sector
                self.io.borrow_mut().sector_buf = [0; SECTOR_SIZE];
                self.continue_path(job(
                    parts, comp_i, dir_lba, 3, slot, entry_lba, 0, 0, to_parts, to_comp_i,
                    to_dir_lba, to_slot,
                ))
            }
            3 => {
                let data = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(entry_lba, &data);
                if __w {
                    // Read parent dir again is not needed — we stored it before alloc, but
                    // sector_buf was overwritten. Re-read parent.
                    return self.continue_path(job(
                        parts, comp_i, dir_lba, 14, slot, entry_lba, 0, 0, to_parts, to_comp_i,
                        to_dir_lba, to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Mkdir: allocate empty dir sector.
            4 => {
                let Some(new_lba) = alloc_contiguous(&mut self.free_map, &self.superblock, 1)
                else {
                    return Some(FsResponse::Error);
                };
                self.freemap_dirty = true;
                entry_lba = new_lba;
                self.io.borrow_mut().sector_buf = [0; SECTOR_SIZE];
                self.continue_path(job(
                    parts,
                    comp_i,
                    dir_lba,
                    5,
                    slot,
                    entry_lba,
                    0,
                    lerux_fs::FLAG_DIR,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ))
            }
            5 => {
                let data = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(entry_lba, &data);
                if __w {
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        15, // write mkdir dirent
                        slot,
                        entry_lba,
                        0,
                        lerux_fs::FLAG_DIR,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Unlink dir: ensure empty.
            6 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(entry_lba);
                if let Some(child) = __tmp {
                    if !dir_is_empty(&child) {
                        return Some(FsResponse::Error);
                    }
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        7,
                        slot,
                        entry_lba,
                        entry_size,
                        entry_flags,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Free leaf blocks + clear dirent (parent in sector_buf may be stale).
            7 => {
                let is_dir = (entry_flags & lerux_fs::FLAG_DIR) != 0;
                if is_dir {
                    free_contiguous(&mut self.free_map, entry_lba, 1);
                } else {
                    let n = if entry_size == 0 {
                        1
                    } else {
                        entry_size
                            .div_ceil(SECTOR_SIZE as u32)
                            .clamp(1, MAX_FILE_SECTORS)
                    };
                    free_contiguous(&mut self.free_map, entry_lba, n);
                }
                self.freemap_dirty = true;
                self.continue_path(job(
                    parts,
                    comp_i,
                    dir_lba,
                    8,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ))
            }
            8 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    encode_dir_entry(&mut dir, slot as usize, &DirEntry::empty());
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        9,
                        slot,
                        entry_lba,
                        entry_size,
                        entry_flags,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            9 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    return self.flush_freemap_then(FsResponse::Ok);
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // ListDir at dir_lba.
            10 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(dir) = __tmp {
                    let mut entries = [FsDirEntry::from_name(&[], 0, false); MAX_FS_DIR_LIST];
                    let mut out_count = 0u8;
                    let _ = count_entries(&dir);
                    for index in 0..MAX_ENTRIES {
                        let entry = decode_dir_entry(&dir, index);
                        if entry.is_free() {
                            continue;
                        }
                        if (out_count as usize) < MAX_FS_DIR_LIST {
                            entries[out_count as usize] = FsDirEntry::from_name(
                                entry.name_slice(),
                                entry.size,
                                entry.is_dir(),
                            );
                            out_count += 1;
                        }
                    }
                    return Some(FsResponse::DirList {
                        count: out_count,
                        entries,
                    });
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Stat root.
            11 => Some(FsResponse::Stat {
                size: 0,
                is_dir: true,
            }),
            // Auto-create intermediate directory.
            12 => {
                let Some(new_lba) = alloc_contiguous(&mut self.free_map, &self.superblock, 1)
                else {
                    return Some(FsResponse::Error);
                };
                self.freemap_dirty = true;
                entry_lba = new_lba;
                self.io.borrow_mut().sector_buf = [0; SECTOR_SIZE];
                self.continue_path(job(
                    parts,
                    comp_i,
                    dir_lba,
                    13,
                    slot,
                    entry_lba,
                    0,
                    lerux_fs::FLAG_DIR,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ))
            }
            13 => {
                let empty = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(entry_lba, &empty);
                if __w {
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        16,
                        slot,
                        entry_lba,
                        0,
                        lerux_fs::FLAG_DIR,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Write create file dirent (re-read parent).
            14 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    let Some(name) = parts.component(comp_i as usize) else {
                        return Some(FsResponse::Error);
                    };
                    let entry = make_entry(name, entry_lba, 0, false);
                    encode_dir_entry(&mut dir, slot as usize, &entry);
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts, comp_i, dir_lba, 17, slot, entry_lba, 0, 0, to_parts, to_comp_i,
                        to_dir_lba, to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Write mkdir dirent.
            15 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    let Some(name) = parts.component(comp_i as usize) else {
                        return Some(FsResponse::Error);
                    };
                    let entry = make_entry(name, entry_lba, 0, true);
                    encode_dir_entry(&mut dir, slot as usize, &entry);
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        18,
                        slot,
                        entry_lba,
                        0,
                        lerux_fs::FLAG_DIR,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Write intermediate auto-mkdir dirent then continue walk.
            16 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    let Some(name) = parts.component(comp_i as usize) else {
                        return Some(FsResponse::Error);
                    };
                    let entry = make_entry(name, entry_lba, 0, true);
                    encode_dir_entry(&mut dir, slot as usize, &entry);
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        19,
                        slot,
                        entry_lba,
                        0,
                        lerux_fs::FLAG_DIR,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            17 => {
                // flush create dirent + freemap, open handle
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    if self.freemap_dirty {
                        return self.continue_path(job(
                            parts, comp_i, dir_lba, 30, slot, entry_lba, 0, 0, to_parts, to_comp_i,
                            to_dir_lba, to_slot,
                        ));
                    }
                    let id = self.alloc_handle(OpenFile {
                        in_use: true,
                        dir_lba,
                        slot,
                        first_lba: entry_lba,
                        size: 0,
                        is_dir: false,
                    });
                    return Some(match id {
                        Some(id) => FsResponse::Handle { id },
                        None => FsResponse::Error,
                    });
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            18 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    return self.flush_freemap_then(FsResponse::Ok);
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            19 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    // Descend into new intermediate dir and continue walk.
                    dir_lba = entry_lba;
                    comp_i += 1;
                    return self.continue_path(job(
                        parts, comp_i, dir_lba, 1, 0, 0, 0, 0, to_parts, to_comp_i, to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Rename: walk destination parent.
            20 => {
                let last = to_comp_i + 1 >= to_parts.count;
                let Some(name) = to_parts.component(to_comp_i as usize) else {
                    return Some(FsResponse::Error);
                };
                let __tmp = self.io.borrow_mut().poll_read_sector(to_dir_lba);
                if let Some(dir) = __tmp {
                    if last {
                        if find_by_name(&dir, name).is_some() {
                            return Some(FsResponse::Error);
                        }
                        let Some(idx) = find_free_slot(&dir) else {
                            return Some(FsResponse::Error);
                        };
                        to_slot = idx as u8;
                        // same parent + rename in place?
                        if to_dir_lba == dir_lba {
                            // re-read source parent, rewrite name
                            return self.continue_path(job(
                                parts,
                                comp_i,
                                dir_lba,
                                21,
                                slot,
                                entry_lba,
                                entry_size,
                                entry_flags,
                                to_parts,
                                to_comp_i,
                                to_dir_lba,
                                to_slot,
                            ));
                        }
                        // move across dirs: write into dest, clear source
                        return self.continue_path(job(
                            parts,
                            comp_i,
                            dir_lba,
                            22,
                            slot,
                            entry_lba,
                            entry_size,
                            entry_flags,
                            to_parts,
                            to_comp_i,
                            to_dir_lba,
                            to_slot,
                        ));
                    }
                    match find_by_name(&dir, name) {
                        Some(idx) => {
                            let e = decode_dir_entry(&dir, idx);
                            if !e.is_dir() {
                                return Some(FsResponse::Error);
                            }
                            to_dir_lba = e.first_lba;
                            to_comp_i += 1;
                            return self.continue_path(job(
                                parts,
                                comp_i,
                                dir_lba,
                                20,
                                slot,
                                entry_lba,
                                entry_size,
                                entry_flags,
                                to_parts,
                                to_comp_i,
                                to_dir_lba,
                                to_slot,
                            ));
                        }
                        None => return Some(FsResponse::Error),
                    }
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Rename same-dir: rewrite name in place (slot stays).
            21 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    let Some(new_name) = to_parts.component(to_parts.count as usize - 1) else {
                        return Some(FsResponse::Error);
                    };
                    let mut e = decode_dir_entry(&dir, slot as usize);
                    let nlen = new_name.len().min(lerux_fs::NAME_LEN);
                    e.name = [0; lerux_fs::NAME_LEN];
                    e.name[..nlen].copy_from_slice(&new_name[..nlen]);
                    e.name_len = nlen as u8;
                    encode_dir_entry(&mut dir, slot as usize, &e);
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        23,
                        slot,
                        entry_lba,
                        entry_size,
                        entry_flags,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Rename cross-dir: write dest entry.
            22 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(to_dir_lba);
                if let Some(mut dir) = __tmp {
                    let Some(new_name) = to_parts.component(to_parts.count as usize - 1) else {
                        return Some(FsResponse::Error);
                    };
                    let entry = DirEntry {
                        name: {
                            let mut n = [0u8; lerux_fs::NAME_LEN];
                            let nlen = new_name.len().min(lerux_fs::NAME_LEN);
                            n[..nlen].copy_from_slice(&new_name[..nlen]);
                            n
                        },
                        name_len: new_name.len().min(lerux_fs::NAME_LEN) as u8,
                        flags: entry_flags,
                        first_lba: entry_lba,
                        size: entry_size,
                    };
                    encode_dir_entry(&mut dir, to_slot as usize, &entry);
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        24,
                        slot,
                        entry_lba,
                        entry_size,
                        entry_flags,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            23 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            24 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(to_dir_lba, &dir);
                if __w {
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        25,
                        slot,
                        entry_lba,
                        entry_size,
                        entry_flags,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            25 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    encode_dir_entry(&mut dir, slot as usize, &DirEntry::empty());
                    self.io.borrow_mut().sector_buf = dir;
                    return self.continue_path(job(
                        parts,
                        comp_i,
                        dir_lba,
                        26,
                        slot,
                        entry_lba,
                        entry_size,
                        entry_flags,
                        to_parts,
                        to_comp_i,
                        to_dir_lba,
                        to_slot,
                    ));
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            26 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            // Flush freemap after create, then return handle.
            30 => {
                let map = self.free_map;
                let __w = self
                    .io
                    .borrow_mut()
                    .poll_write_sector(self.superblock.free_map_lba, &map);
                if __w {
                    self.freemap_dirty = false;
                    let id = self.alloc_handle(OpenFile {
                        in_use: true,
                        dir_lba,
                        slot,
                        first_lba: entry_lba,
                        size: 0,
                        is_dir: false,
                    });
                    return Some(match id {
                        Some(id) => FsResponse::Handle { id },
                        None => FsResponse::Error,
                    });
                }
                self.restore_job(job(
                    parts,
                    comp_i,
                    dir_lba,
                    step,
                    slot,
                    entry_lba,
                    entry_size,
                    entry_flags,
                    to_parts,
                    to_comp_i,
                    to_dir_lba,
                    to_slot,
                ));
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn continue_path(&mut self, job: FsJob) -> Option<FsResponse> {
        self.fs_job = job;
        self.advance_fs_job()
    }

    fn flush_freemap_then(&mut self, resp: FsResponse) -> Option<FsResponse> {
        if !self.freemap_dirty {
            return Some(resp);
        }
        let map = self.free_map;
        // Busy-wait style: store resp by doing a one-shot write job inline via Path step 31? simpler:
        // use a dedicated micro-loop with Write freemap step stored in completed after write.
        // We'll poll write once; if not ready, stash as a Path job step 31 with entry_size encoding resp kind.
        // Simpler approach: restore a synthetic freemap flush job.
        self.fs_job = FsJob::Path {
            op: PathOp::Stat, // unused
            path: [0; MAX_FS_PATH],
            path_len: 0,
            to_path: [0; MAX_FS_PATH],
            to_path_len: 0,
            parts: PathParts::empty(),
            comp_i: 0,
            dir_lba: 0,
            step: 31,
            slot: match resp {
                FsResponse::Ok => 0,
                FsResponse::Handle { id } => id,
                _ => 0xff,
            },
            entry_lba: 0,
            entry_size: match &resp {
                FsResponse::Ok => 0,
                FsResponse::Handle { .. } => 1,
                _ => 2,
            },
            entry_flags: 0,
            to_parts: PathParts::empty(),
            to_comp_i: 0,
            to_dir_lba: 0,
            to_slot: 0,
        };
        // Write freemap using sector_buf
        let _ = map;
        self.io.borrow_mut().sector_buf = map;
        // Handle step 31 below by extending match — inject here:
        self.advance_flush_freemap(resp)
    }

    fn advance_flush_freemap(&mut self, resp: FsResponse) -> Option<FsResponse> {
        let map = self.free_map;
        let __w = self
            .io
            .borrow_mut()
            .poll_write_sector(self.superblock.free_map_lba, &map);
        if __w {
            self.freemap_dirty = false;
            self.fs_job = FsJob::None;
            return Some(resp);
        }
        // Keep freemap flush pending: store response kind in Path step 31.
        let (slot, entry_size) = match resp {
            FsResponse::Ok => (0u8, 0u32),
            FsResponse::Handle { id } => (id, 1u32),
            _ => (0xff, 2u32),
        };
        self.fs_job = FsJob::Path {
            op: PathOp::Stat,
            path: [0; MAX_FS_PATH],
            path_len: 0,
            to_path: [0; MAX_FS_PATH],
            to_path_len: 0,
            parts: PathParts::empty(),
            comp_i: 0,
            dir_lba: 0,
            step: 31,
            slot,
            entry_lba: 0,
            entry_size,
            entry_flags: 0,
            to_parts: PathParts::empty(),
            to_comp_i: 0,
            to_dir_lba: 0,
            to_slot: 0,
        };
        None
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "write job threads payload and extent state through poll stages"
    )]
    fn advance_write(
        &mut self,
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        mut first_lba: u32,
        mut size: u32,
        mut dir_lba: u32,
        mut slot: u8,
        mut done: u16,
        mut new_size: u32,
        mut n_sectors: u32,
        mut freemap_dirty: bool,
    ) -> Option<FsResponse> {
        let job = |step: u8,
                   first_lba: u32,
                   size: u32,
                   dir_lba: u32,
                   slot: u8,
                   done: u16,
                   new_size: u32,
                   n_sectors: u32,
                   freemap_dirty: bool| FsJob::Write {
            handle,
            offset,
            data,
            data_len,
            step,
            first_lba,
            size,
            dir_lba,
            slot,
            done,
            new_size,
            n_sectors,
            freemap_dirty,
        };

        match step {
            0 => {
                if !self.formatted {
                    return Some(FsResponse::Error);
                }
                let Some(of) = self.open.get(handle as usize).copied() else {
                    return Some(FsResponse::Error);
                };
                if !of.in_use || of.is_dir {
                    return Some(FsResponse::Error);
                }
                first_lba = of.first_lba;
                size = of.size;
                dir_lba = of.dir_lba;
                slot = of.slot;
                n_sectors = of.file_sectors();
                let end = offset.saturating_add(data_len as u32);
                if end > MAX_FILE_SECTORS * SECTOR_SIZE as u32 {
                    return Some(FsResponse::Error);
                }
                new_size = size.max(end);
                let need = if new_size == 0 {
                    1
                } else {
                    new_size
                        .div_ceil(SECTOR_SIZE as u32)
                        .clamp(1, MAX_FILE_SECTORS)
                };
                if need > n_sectors {
                    // Grow: allocate new extent, copy old sectors (step 1..).
                    let Some(new_lba) =
                        alloc_contiguous(&mut self.free_map, &self.superblock, need)
                    else {
                        return Some(FsResponse::Error);
                    };
                    freemap_dirty = true;
                    // free old after copy — stash old first_lba in size temporarily? use steps.
                    // Copy n_sectors from first_lba to new_lba.
                    self.fs_job = job(
                        1,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        0,
                        new_size,
                        need,
                        freemap_dirty,
                    );
                    // Overload: store new_lba in open temporarily via entry in job — use
                    // `done` as copy index; put new_lba where? Use first_lba as OLD, and
                    // encode new in a side field: put new_lba into open[handle].first_lba
                    // only after copy. Keep new_lba in `new_size` high? Better: use
                    // freemap_dirty path with first_lba=old, and n_sectors=need, store new
                    // base in open[handle] after.
                    // Store new base in open slot's first_lba as pending: use dir_lba field
                    // of job for parent, and put new_lba in a re-used: I'll put new_lba as
                    // the job's first_lba after free of old — copy loop:
                    // step 1: copy sector done from old first_lba to new.
                    // Actually rewrite: step 1 uses first_lba=old, and we need new.
                    // Put new_lba into `open[handle].first_lba` now as target, keep old in
                    // job first_lba for free later — messes open table.
                    // Simplest: store new_lba in `dir_lba` temporarily? No, need parent.
                    // Use step 1 with first_lba=new, size=old_first, n_sectors=need, slot=old_sectors
                    // Wait: size is file size. Let's use entry_size style:
                    // first_lba = NEW, and encode OLD in unused: freemap_dirty path.
                    // I'll use: first_lba stays OLD until copy done; n_sectors = need;
                    // new base stored by writing to open[handle].size as 0 and...
                    // Clean approach: field `new_size` keeps target size; add local via
                    // packing new_lba into high bits of something.
                    //
                    // Use open table:
                    let old_lba = first_lba;
                    let old_n = of.file_sectors();
                    self.open[handle as usize].first_lba = new_lba; // target
                    self.fs_job = job(
                        1,
                        old_lba,
                        size,
                        dir_lba,
                        slot,
                        0, // copy index
                        new_size,
                        need,
                        freemap_dirty,
                    );
                    // step 1 will copy from old_lba to open.first_lba
                    let _ = old_n;
                    return self.advance_fs_job();
                }
                done = 0;
                self.continue_write(job(
                    2,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ))
            }
            // Grow copy: copy sector `done` from old first_lba → open.first_lba.
            1 => {
                let new_base = self.open[handle as usize].first_lba;
                let old_base = first_lba;
                let old_n = if size == 0 {
                    1
                } else {
                    size.div_ceil(SECTOR_SIZE as u32).clamp(1, MAX_FILE_SECTORS)
                };
                if (done as u32) >= old_n {
                    // Free old extent (if different).
                    if old_base != new_base {
                        free_contiguous(&mut self.free_map, old_base, old_n);
                        freemap_dirty = true;
                    }
                    first_lba = new_base;
                    n_sectors = {
                        // need was stored in n_sectors already at begin of grow
                        n_sectors
                    };
                    done = 0;
                    return self.continue_write(job(
                        2,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                let src = old_base + done as u32;
                let __tmp = self.io.borrow_mut().poll_read_sector(src);
                if let Some(sector) = __tmp {
                    self.io.borrow_mut().sector_buf = sector;
                    return self.continue_write(job(
                        10,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            10 => {
                let new_base = self.open[handle as usize].first_lba;
                let dst = new_base + done as u32;
                let sector = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dst, &sector);
                if __w {
                    done = done.saturating_add(1);
                    return self.continue_write(job(
                        1,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            // Apply write chunks (may span sectors).
            2 => {
                if done >= data_len {
                    // Update dirent size + open table.
                    return self.continue_write(job(
                        4,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                let abs = offset + done as u32;
                let lba = file_lba_for_offset(first_lba, abs);
                let __tmp = self.io.borrow_mut().poll_read_sector(lba);
                if let Some(mut sector) = __tmp {
                    let soff = sector_offset(abs);
                    let remain = (data_len - done) as usize;
                    let chunk = min(remain, SECTOR_SIZE - soff).min(MAX_FS_DATA);
                    sector[soff..soff + chunk]
                        .copy_from_slice(&data[done as usize..done as usize + chunk]);
                    self.io.borrow_mut().sector_buf = sector;
                    // stash chunk length in entry: use step 3 with done advance after write
                    // store chunk in unused: pack into freemap_dirty high? use size field as chunk temporarily — no.
                    // put chunk into `slot` high? Keep chunk via recompute on step 3.
                    return self.continue_write(job(
                        3,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            3 => {
                let abs = offset + done as u32;
                let lba = file_lba_for_offset(first_lba, abs);
                let soff = sector_offset(abs);
                let remain = (data_len - done) as usize;
                let chunk = min(remain, SECTOR_SIZE - soff).min(MAX_FS_DATA) as u16;
                let sector = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(lba, &sector);
                if __w {
                    done = done.saturating_add(chunk);
                    return self.continue_write(job(
                        2,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            4 => {
                let __tmp = self.io.borrow_mut().poll_read_sector(dir_lba);
                if let Some(mut dir) = __tmp {
                    let mut entry = decode_dir_entry(&dir, slot as usize);
                    entry.first_lba = first_lba;
                    entry.size = new_size;
                    encode_dir_entry(&mut dir, slot as usize, &entry);
                    self.io.borrow_mut().sector_buf = dir;
                    if let Some(of) = self.open.get_mut(handle as usize) {
                        of.first_lba = first_lba;
                        of.size = new_size;
                    }
                    return self.continue_write(job(
                        5,
                        first_lba,
                        size,
                        dir_lba,
                        slot,
                        done,
                        new_size,
                        n_sectors,
                        freemap_dirty,
                    ));
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            5 => {
                let dir = self.io.borrow_mut().sector_buf;
                let __w = self.io.borrow_mut().poll_write_sector(dir_lba, &dir);
                if __w {
                    if freemap_dirty {
                        return self.continue_write(job(
                            6,
                            first_lba,
                            size,
                            dir_lba,
                            slot,
                            done,
                            new_size,
                            n_sectors,
                            freemap_dirty,
                        ));
                    }
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            6 => {
                let map = self.free_map;
                let __w = self
                    .io
                    .borrow_mut()
                    .poll_write_sector(self.superblock.free_map_lba, &map);
                if __w {
                    self.freemap_dirty = false;
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job(
                    step,
                    first_lba,
                    size,
                    dir_lba,
                    slot,
                    done,
                    new_size,
                    n_sectors,
                    freemap_dirty,
                ));
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn continue_write(&mut self, job: FsJob) -> Option<FsResponse> {
        self.fs_job = job;
        self.advance_fs_job()
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "read job threads payload through poll stages"
    )]
    fn advance_read(
        &mut self,
        handle: u8,
        offset: u32,
        len: u16,
        step: u8,
        mut first_lba: u32,
        mut size: u32,
        mut done: u16,
        mut out: [u8; MAX_FS_DATA],
    ) -> Option<FsResponse> {
        let job =
            |step: u8, first_lba: u32, size: u32, done: u16, out: [u8; MAX_FS_DATA]| FsJob::Read {
                handle,
                offset,
                len,
                step,
                first_lba,
                size,
                done,
                out,
            };

        match step {
            0 => {
                if !self.formatted {
                    return Some(FsResponse::Error);
                }
                let Some(of) = self.open.get(handle as usize).copied() else {
                    return Some(FsResponse::Error);
                };
                if !of.in_use || of.is_dir {
                    return Some(FsResponse::Error);
                }
                first_lba = of.first_lba;
                size = of.size;
                if offset >= size {
                    return Some(FsResponse::Data {
                        data_len: 0,
                        data: [0; MAX_FS_DATA],
                    });
                }
                done = 0;
                self.fs_job = job(1, first_lba, size, done, out);
                self.advance_fs_job()
            }
            1 => {
                let want_total = min(len as u32, size.saturating_sub(offset)) as u16;
                let want_total = min(want_total as usize, MAX_FS_DATA) as u16;
                if done >= want_total {
                    return Some(FsResponse::Data {
                        data_len: done,
                        data: out,
                    });
                }
                let abs = offset + done as u32;
                let lba = file_lba_for_offset(first_lba, abs);
                let __tmp = self.io.borrow_mut().poll_read_sector(lba);
                if let Some(sector) = __tmp {
                    let soff = sector_offset(abs);
                    let remain = (want_total - done) as usize;
                    let chunk = min(remain, SECTOR_SIZE - soff);
                    out[done as usize..done as usize + chunk]
                        .copy_from_slice(&sector[soff..soff + chunk]);
                    done = done.saturating_add(chunk as u16);
                    self.fs_job = job(1, first_lba, size, done, out);
                    return self.advance_fs_job();
                }
                self.restore_job(job(step, first_lba, size, done, out));
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn handle_request(&mut self, channel: Channel, req: FsRequest) -> FsResponse {
        if matches!(req, FsRequest::Poll) {
            return self.handle_poll(channel);
        }

        if !self.begin_async(channel) {
            return FsResponse::Pending;
        }

        if self.completed.is_some()
            || !matches!(self.fs_job, FsJob::None)
            || self.format_task.is_running()
        {
            return FsResponse::Pending;
        }

        // Resume freemap flush if left pending from previous (shouldn't hit with None job).
        match req {
            FsRequest::Open { path_len, path } => {
                if path_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::Open, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Create { path_len, path } => {
                if path_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::Create, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Write {
                handle,
                offset,
                data_len,
                data,
            } => {
                if data_len as usize > MAX_FS_DATA {
                    return self.sync_response(FsResponse::Error);
                }
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
                if path_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::Stat, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::ListDir { path_len, path } => {
                if path_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::ListDir, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Mkdir { path_len, path } => {
                if path_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::Mkdir, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Unlink { path_len, path } => {
                if path_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::Unlink, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Rename {
                from_len,
                from,
                to_len,
                to,
            } => {
                if from_len as usize > MAX_FS_PATH || to_len as usize > MAX_FS_PATH {
                    return self.sync_response(FsResponse::Error);
                }
                self.begin_path_op(PathOp::RenameFrom, from, from_len, to, to_len);
            }
            FsRequest::DiskInfo => {
                if !self.formatted {
                    // Trigger format/mount; client should Poll until ready, then retry DiskInfo.
                    if self.format_task.is_idle() {
                        let io = self.io.clone();
                        self.format_task
                            .spawn(async move { format_leruxfs(io).await });
                    }
                    if let Some(resp) = self.poll_format_task() {
                        // Discard Ok from format; fall through if now formatted.
                        if !self.formatted {
                            return self.sync_response(resp);
                        }
                    } else {
                        return FsResponse::Pending;
                    }
                }
                let free = count_free_blocks(&self.free_map, &self.superblock);
                let total = self
                    .superblock
                    .total_lbas
                    .saturating_sub(self.superblock.data_start_lba);
                return self.sync_response(FsResponse::DiskInfo {
                    block_size: SECTOR_SIZE as u32,
                    total_blocks: total,
                    free_blocks: free,
                });
            }
            FsRequest::Poll => return self.handle_poll(channel),
        }

        // Handle freemap flush resume step 31 if we ever stash it as active job with only freemap.
        if let Some(resp) = self.advance_fs_job() {
            // Special-case step 31 completion already returns response.
            if matches!(
                resp,
                FsResponse::Ok | FsResponse::Handle { .. } | FsResponse::Error
            ) || matches!(
                resp,
                FsResponse::Data { .. } | FsResponse::Stat { .. } | FsResponse::DirList { .. }
            ) {
                // If freemap flush incomplete, advance_path step 31:
                // Integrated: check job for step 31
            }
            // Also handle step 31 in advance_path — add case:
            self.finish_job(resp);
            return self.take_completed(channel).unwrap_or(FsResponse::Pending);
        }
        // If job is freemap flush step 31 pending:
        if let FsJob::Path { step: 31, .. } = self.fs_job {
            // try again
            if let Some(resp) = self.advance_fs_job() {
                self.finish_job(resp);
                return self.take_completed(channel).unwrap_or(FsResponse::Pending);
            }
        }
        FsResponse::Pending
    }

    fn handle_poll(&mut self, channel: Channel) -> FsResponse {
        if let Some(resp) = self.take_completed(channel) {
            return resp;
        }
        if self.active_client != Some(channel) {
            return FsResponse::Pending;
        }
        // Resume freemap flush.
        if let FsJob::Path {
            step: 31,
            slot,
            entry_size,
            ..
        } = self.fs_job
        {
            let resp = match entry_size {
                0 => FsResponse::Ok,
                1 => FsResponse::Handle { id: slot },
                _ => FsResponse::Error,
            };
            if let Some(r) = self.advance_flush_freemap(resp) {
                self.finish_job(r);
                return self.take_completed(channel).unwrap_or(FsResponse::Pending);
            }
            return FsResponse::Pending;
        }
        // Opportunistically drain blk completions even if the driver notify was
        // coalesced while we were only handling PPC Poll (busy-wait clients).
        if self.io.borrow().io_busy() {
            self.handle_blk_driver();
        }
        if self.format_task.is_running()
            && let Some(resp) = self.poll_format_task()
        {
            self.finish_job(resp);
            return self.take_completed(channel).unwrap_or(FsResponse::Pending);
        }
        if let Some(resp) = self.advance_fs_job() {
            self.finish_job(resp);
            return self.take_completed(channel).unwrap_or(FsResponse::Pending);
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

/// Sequential format/mount for LERUXFS2 (legacy v1 volumes are reformatted).
async fn format_leruxfs(io: SharedSectorIo) -> Result<(Superblock, [u8; SECTOR_SIZE]), ()> {
    let sector = read_sector(io.clone(), SUPERBLOCK_LBA).await?;
    if is_formatted(&sector) {
        let sb = decode_superblock(&sector).ok_or(())?;
        let map = read_sector(io, sb.free_map_lba).await?;
        return Ok((sb, map));
    }
    // LERUXFS1 or empty → format v2.
    let _ = is_legacy_v1(&sector);
    let sb = Superblock::new();
    let mut buf = [0u8; SECTOR_SIZE];
    encode_superblock(&mut buf, &sb);
    write_sector(io.clone(), SUPERBLOCK_LBA, buf).await?;
    let mut map = [0u8; SECTOR_SIZE];
    encode_free_map_fresh(&mut map, &sb);
    write_sector(io.clone(), FREE_MAP_LBA, map).await?;
    let root = [0u8; SECTOR_SIZE];
    write_sector(io, ROOT_DIR_LBA, root).await?;
    // map already has reserved bits; data_start free
    let _ = DATA_START_LBA;
    Ok((sb, map))
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if !Self::is_client(channel) {
            unreachable!("unexpected fs client");
        }

        Ok(match recv::<FsRequest>(msg_info) {
            Ok(req) => send(self.handle_request(channel, req)),
            Err(_) => send_unspecified_error(),
        })
    }

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(BLK_DRIVER) {
            self.handle_blk_driver();
            if self.format_task.is_running()
                && let Some(resp) = self.poll_format_task()
            {
                self.finish_job(resp);
            }
        }
        Ok(())
    }
}
