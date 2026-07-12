//! FAT16 backend for Phase 44 (root-only, single-cluster files, 8.3 names).

use core::cmp::min;

use lerux_fat::{
    decode_bpb, decode_dir_entry, encode_boot_sector, encode_dir_entry, encode_fat_first_sector,
    encode_zero_sector, fat_get, fat_sector_index, fat_set, format_payload_is_fat_head,
    format_payload_lba, format_payload_sectors, path_to_short_name, root_slot_location,
    short_name_to_display, Bpb, DirEntry, BOOT_LBA, EOC, FREE, MAX_HANDLES,
};
use lerux_interface_types::{
    FsDirEntry, FsRequest, FsResponse, MAX_FS_DATA, MAX_FS_DIR_LIST, MAX_FS_PATH, SECTOR_SIZE,
};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::log;
use sel4_microkit::{Channel, ChannelSet, Handler, Infallible, MessageInfo};

use crate::block_io::{SectorIo, BLK_DRIVER, CLIENT};

#[derive(Clone, Copy)]
struct OpenFile {
    in_use: bool,
    first_cluster: u16,
    size: u32,
    /// Absolute root-dir slot for dirent updates.
    root_slot: u16,
}

impl OpenFile {
    const fn empty() -> Self {
        Self {
            in_use: false,
            first_cluster: 0,
            size: 0,
            root_slot: 0,
        }
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "Write job carries inline IPC payload while job is in flight"
)]
enum FsJob {
    None,
    /// step0: read boot; step1: write boot; step2+: write FAT/root payload sectors
    Format {
        step: u8,
        payload_i: u32,
    },
    Open {
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
        root_sec: u16,
    },
    Create {
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
        root_sec: u16,
        free_slot: u16,
        free_cluster: u16,
        fat_sec: u32,
    },
    Write {
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        data_lba: u32,
        new_size: u32,
        root_slot: u16,
        root_sec_off: u32,
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
        root_sec: u16,
    },
    ListDir {
        step: u8,
        root_sec: u16,
        out_count: u8,
        entries: [FsDirEntry; MAX_FS_DIR_LIST],
    },
}

pub struct HandlerImpl {
    io: SectorIo,
    bpb: Bpb,
    mounted: bool,
    fs_job: FsJob,
    after_format: Option<FsJob>,
    completed: Option<FsResponse>,
    opens: [OpenFile; MAX_HANDLES],
    /// Scratch for root sector currently being edited.
    root_buf: [u8; SECTOR_SIZE],
    fat_buf: [u8; SECTOR_SIZE],
}

impl HandlerImpl {
    pub fn new(block_size: usize) -> HandlerImpl {
        log::info!("lerux-fs: ready (FAT16)");
        HandlerImpl {
            io: SectorIo::new(block_size),
            bpb: Bpb::fixed(),
            mounted: false,
            fs_job: FsJob::None,
            after_format: None,
            completed: None,
            opens: [OpenFile::empty(); MAX_HANDLES],
            root_buf: [0; SECTOR_SIZE],
            fat_buf: [0; SECTOR_SIZE],
        }
    }

    fn alloc_handle(&mut self, cluster: u16, size: u32, root_slot: u16) -> Option<u8> {
        for (i, slot) in self.opens.iter_mut().enumerate() {
            if !slot.in_use {
                *slot = OpenFile {
                    in_use: true,
                    first_cluster: cluster,
                    size,
                    root_slot,
                };
                return Some(i as u8);
            }
        }
        None
    }

    fn begin_open(&mut self, path: [u8; MAX_FS_PATH], path_len: u8) {
        self.fs_job = FsJob::Open {
            path,
            path_len,
            step: 0,
            root_sec: 0,
        };
    }

    fn begin_create(&mut self, path: [u8; MAX_FS_PATH], path_len: u8) {
        self.fs_job = FsJob::Create {
            path,
            path_len,
            step: 0,
            root_sec: 0,
            free_slot: 0,
            free_cluster: 0,
            fat_sec: 0,
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
            root_slot: 0,
            root_sec_off: 0,
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
            root_sec: 0,
        };
    }

    fn begin_list_dir(&mut self) {
        self.fs_job = FsJob::ListDir {
            step: 0,
            root_sec: 0,
            out_count: 0,
            entries: [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST],
        };
    }

    fn finish_job(&mut self, response: FsResponse) {
        self.fs_job = FsJob::None;
        self.completed = Some(response);
    }

    fn restore_job(&mut self, job: FsJob) {
        self.fs_job = job;
    }

    fn advance_fs_job(&mut self) -> Option<FsResponse> {
        match core::mem::replace(&mut self.fs_job, FsJob::None) {
            FsJob::None => None,
            FsJob::Format { step, payload_i } => self.advance_format(step, payload_i),
            FsJob::Open {
                path,
                path_len,
                step,
                root_sec,
            } => self.advance_open(path, path_len, step, root_sec),
            FsJob::Create {
                path,
                path_len,
                step,
                root_sec,
                free_slot,
                free_cluster,
                fat_sec,
            } => self.advance_create(
                path,
                path_len,
                step,
                root_sec,
                free_slot,
                free_cluster,
                fat_sec,
            ),
            FsJob::Write {
                handle,
                offset,
                data,
                data_len,
                step,
                data_lba,
                new_size,
                root_slot,
                root_sec_off,
            } => self.advance_write(
                handle,
                offset,
                data,
                data_len,
                step,
                data_lba,
                new_size,
                root_slot,
                root_sec_off,
            ),
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
                root_sec,
            } => self.advance_stat(path, path_len, step, root_sec),
            FsJob::ListDir {
                step,
                root_sec,
                out_count,
                entries,
            } => self.advance_list_dir(step, root_sec, out_count, entries),
        }
    }

    fn maybe_mount_then(&mut self, next: FsJob) -> Option<FsResponse> {
        if self.mounted {
            self.fs_job = next;
            return self.advance_fs_job();
        }
        self.after_format = Some(next);
        if matches!(self.fs_job, FsJob::None) {
            self.fs_job = FsJob::Format {
                step: 0,
                payload_i: 0,
            };
        }
        self.advance_fs_job()
    }

    fn advance_format(&mut self, step: u8, payload_i: u32) -> Option<FsResponse> {
        let job = FsJob::Format { step, payload_i };
        match step {
            0 => {
                if let Some(sector) = self.io.poll_read_sector(BOOT_LBA) {
                    if let Some(bpb) = decode_bpb(&sector) {
                        self.bpb = bpb;
                        self.mounted = true;
                        if let Some(next) = self.after_format.take() {
                            self.fs_job = next;
                            return self.advance_fs_job();
                        }
                        self.fs_job = FsJob::None;
                        return Some(FsResponse::Ok);
                    }
                    // Not FAT — format with fixed layout.
                    self.bpb = Bpb::fixed();
                    encode_boot_sector(&mut self.io.sector_buf);
                    self.fs_job = FsJob::Format {
                        step: 1,
                        payload_i: 0,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            1 => {
                let sector = self.io.sector_buf;
                if self.io.poll_write_sector(BOOT_LBA, &sector) {
                    self.fs_job = FsJob::Format {
                        step: 2,
                        payload_i: 0,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            2 => {
                let total = format_payload_sectors();
                if payload_i >= total {
                    self.mounted = true;
                    if let Some(next) = self.after_format.take() {
                        self.fs_job = next;
                        return self.advance_fs_job();
                    }
                    self.fs_job = FsJob::None;
                    return Some(FsResponse::Ok);
                }
                if format_payload_is_fat_head(payload_i) {
                    encode_fat_first_sector(&mut self.io.sector_buf);
                } else {
                    encode_zero_sector(&mut self.io.sector_buf);
                }
                let lba = format_payload_lba(payload_i);
                let sector = self.io.sector_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    self.fs_job = FsJob::Format {
                        step: 2,
                        payload_i: payload_i + 1,
                    };
                    return self.advance_fs_job();
                }
                // Keep prepared sector_buf for retry of same payload_i.
                if format_payload_is_fat_head(payload_i) {
                    encode_fat_first_sector(&mut self.io.sector_buf);
                } else {
                    encode_zero_sector(&mut self.io.sector_buf);
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn find_in_root_sector(
        sector: &[u8; SECTOR_SIZE],
        short: &[u8; 11],
    ) -> Option<(usize, DirEntry)> {
        for index in 0..lerux_fat::ENTRIES_PER_SECTOR {
            let e = decode_dir_entry(sector, index);
            if e.is_end() {
                return None;
            }
            if e.is_free() || e.is_volume_or_lfn() || e.is_dir() {
                continue;
            }
            if &e.name == short {
                return Some((index, e));
            }
        }
        None
    }

    fn free_slot_in_sector(sector: &[u8; SECTOR_SIZE]) -> Option<usize> {
        for index in 0..lerux_fat::ENTRIES_PER_SECTOR {
            let e = decode_dir_entry(sector, index);
            if e.is_end() || e.is_free() {
                return Some(index);
            }
        }
        None
    }

    fn advance_open(
        &mut self,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
        root_sec: u16,
    ) -> Option<FsResponse> {
        let job = FsJob::Open {
            path,
            path_len,
            step,
            root_sec,
        };
        let name = &path[..path_len as usize];
        let Some(short) = path_to_short_name(name) else {
            return Some(FsResponse::Error);
        };
        match step {
            0 => self.maybe_mount_then(FsJob::Open {
                path,
                path_len,
                step: 1,
                root_sec: 0,
            }),
            1 => {
                let root_sectors = self.bpb.root_sectors() as u16;
                if root_sec >= root_sectors {
                    return Some(FsResponse::Error);
                }
                let lba = self.bpb.root_lba + u32::from(root_sec);
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    if let Some((idx, e)) = Self::find_in_root_sector(&sector, &short) {
                        let slot = root_sec * lerux_fat::ENTRIES_PER_SECTOR as u16 + idx as u16;
                        let Some(id) = self.alloc_handle(e.first_cluster, e.size, slot) else {
                            return Some(FsResponse::Error);
                        };
                        return Some(FsResponse::Handle { id });
                    }
                    // Continue search if sector did not end the directory.
                    let mut ended = false;
                    for index in 0..lerux_fat::ENTRIES_PER_SECTOR {
                        if decode_dir_entry(&sector, index).is_end() {
                            ended = true;
                            break;
                        }
                    }
                    if ended || root_sec + 1 >= root_sectors {
                        return Some(FsResponse::Error);
                    }
                    self.fs_job = FsJob::Open {
                        path,
                        path_len,
                        step: 1,
                        root_sec: root_sec + 1,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "create job stage state")]
    fn advance_create(
        &mut self,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        step: u8,
        root_sec: u16,
        free_slot: u16,
        free_cluster: u16,
        fat_sec: u32,
    ) -> Option<FsResponse> {
        let job = FsJob::Create {
            path,
            path_len,
            step,
            root_sec,
            free_slot,
            free_cluster,
            fat_sec,
        };
        let name = &path[..path_len as usize];
        let Some(short) = path_to_short_name(name) else {
            return Some(FsResponse::Error);
        };
        match step {
            0 => self.maybe_mount_then(FsJob::Create {
                path,
                path_len,
                step: 1,
                root_sec: 0,
                free_slot: 0,
                free_cluster: 0,
                fat_sec: 0,
            }),
            // Scan root for existing name + free slot
            1 => {
                let root_sectors = self.bpb.root_sectors() as u16;
                if root_sec >= root_sectors {
                    return Some(FsResponse::Error);
                }
                let lba = self.bpb.root_lba + u32::from(root_sec);
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    if Self::find_in_root_sector(&sector, &short).is_some() {
                        return Some(FsResponse::Error);
                    }
                    // free_cluster: 0 = no free slot yet, 1 = free_slot valid
                    let mut have_slot = free_cluster == 1;
                    let mut slot_abs = free_slot;
                    if !have_slot && let Some(idx) = Self::free_slot_in_sector(&sector) {
                        slot_abs = root_sec * lerux_fat::ENTRIES_PER_SECTOR as u16 + idx as u16;
                        have_slot = true;
                    }
                    let mut ended = false;
                    for index in 0..lerux_fat::ENTRIES_PER_SECTOR {
                        if decode_dir_entry(&sector, index).is_end() {
                            ended = true;
                            break;
                        }
                    }
                    if !ended && root_sec + 1 < root_sectors {
                        self.fs_job = FsJob::Create {
                            path,
                            path_len,
                            step: 1,
                            root_sec: root_sec + 1,
                            free_slot: if have_slot { slot_abs } else { 0 },
                            free_cluster: u16::from(have_slot),
                            fat_sec: 0,
                        };
                        return self.advance_fs_job();
                    }
                    if !have_slot {
                        return Some(FsResponse::Error);
                    }
                    self.fs_job = FsJob::Create {
                        path,
                        path_len,
                        step: 2,
                        root_sec: 0,
                        free_slot: slot_abs,
                        free_cluster: 2,
                        fat_sec: 0,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            // Scan FAT for free cluster starting at free_cluster
            2 => {
                let (sec_i, _idx) = fat_sector_index(free_cluster);
                let lba = self.bpb.fat1_lba + sec_i;
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    self.fat_buf = sector;
                    let max = self.bpb.max_cluster();
                    let mut c = free_cluster;
                    loop {
                        let (s, i) = fat_sector_index(c);
                        if s != sec_i {
                            self.fs_job = FsJob::Create {
                                path,
                                path_len,
                                step: 2,
                                root_sec,
                                free_slot,
                                free_cluster: c,
                                fat_sec: s,
                            };
                            return self.advance_fs_job();
                        }
                        if fat_get(&self.fat_buf, i) == FREE {
                            fat_set(&mut self.fat_buf, i, EOC);
                            self.fs_job = FsJob::Create {
                                path,
                                path_len,
                                step: 3,
                                root_sec,
                                free_slot,
                                free_cluster: c,
                                fat_sec: sec_i,
                            };
                            return self.advance_fs_job();
                        }
                        if c >= max {
                            return Some(FsResponse::Error);
                        }
                        c += 1;
                    }
                }
                self.restore_job(job);
                None
            }
            // Write FAT1 sector
            3 => {
                let lba = self.bpb.fat1_lba + fat_sec;
                let sector = self.fat_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    self.fs_job = FsJob::Create {
                        path,
                        path_len,
                        step: 4,
                        root_sec,
                        free_slot,
                        free_cluster,
                        fat_sec,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            // Write FAT2 sector (mirror)
            4 => {
                let lba = self.bpb.fat1_lba + u32::from(self.bpb.fat_sectors) + fat_sec;
                let sector = self.fat_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    self.fs_job = FsJob::Create {
                        path,
                        path_len,
                        step: 5,
                        root_sec,
                        free_slot,
                        free_cluster,
                        fat_sec,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            // Read root sector containing free_slot, write dirent
            5 => {
                let (sec_off, idx) = root_slot_location(free_slot);
                let lba = self.bpb.root_lba + sec_off;
                if let Some(mut sector) = self.io.poll_read_sector(lba) {
                    let mut e = DirEntry::empty();
                    e.name = short;
                    e.attr = 0x20; // archive
                    e.first_cluster = free_cluster;
                    e.size = 0;
                    encode_dir_entry(&mut sector, idx, &e);
                    self.root_buf = sector;
                    self.fs_job = FsJob::Create {
                        path,
                        path_len,
                        step: 6,
                        root_sec,
                        free_slot,
                        free_cluster,
                        fat_sec,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            6 => {
                let (sec_off, _) = root_slot_location(free_slot);
                let lba = self.bpb.root_lba + sec_off;
                let sector = self.root_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    let Some(id) = self.alloc_handle(free_cluster, 0, free_slot) else {
                        return Some(FsResponse::Error);
                    };
                    return Some(FsResponse::Handle { id });
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "write job stage state")]
    fn advance_write(
        &mut self,
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        data_lba: u32,
        new_size: u32,
        root_slot: u16,
        root_sec_off: u32,
    ) -> Option<FsResponse> {
        let job = FsJob::Write {
            handle,
            offset,
            data,
            data_len,
            step,
            data_lba,
            new_size,
            root_slot,
            root_sec_off,
        };
        let h = handle as usize;
        if h >= MAX_HANDLES || !self.opens[h].in_use {
            return Some(FsResponse::Error);
        }
        match step {
            0 => {
                let cluster = self.opens[h].first_cluster;
                let lba = self.bpb.cluster_to_lba(cluster);
                let slot = self.opens[h].root_slot;
                let (sec_off, _) = root_slot_location(slot);
                self.fs_job = FsJob::Write {
                    handle,
                    offset,
                    data,
                    data_len,
                    step: 1,
                    data_lba: lba,
                    new_size: self.opens[h].size,
                    root_slot: slot,
                    root_sec_off: sec_off,
                };
                self.advance_fs_job()
            }
            1 => {
                if let Some(mut sector) = self.io.poll_read_sector(data_lba) {
                    let off = offset as usize;
                    let len = data_len as usize;
                    if off >= SECTOR_SIZE
                        || len > MAX_FS_DATA
                        || off.saturating_add(len) > SECTOR_SIZE
                    {
                        return Some(FsResponse::Error);
                    }
                    sector[off..off + len].copy_from_slice(&data[..len]);
                    self.io.sector_buf = sector;
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
                        root_slot,
                        root_sec_off,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            2 => {
                let sector = self.io.sector_buf;
                if self.io.poll_write_sector(data_lba, &sector) {
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 3,
                        data_lba,
                        new_size,
                        root_slot,
                        root_sec_off,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            // Update dirent size
            3 => {
                let lba = self.bpb.root_lba + root_sec_off;
                if let Some(mut sector) = self.io.poll_read_sector(lba) {
                    let (_, idx) = root_slot_location(root_slot);
                    let mut e = decode_dir_entry(&sector, idx);
                    e.size = new_size;
                    encode_dir_entry(&mut sector, idx, &e);
                    self.root_buf = sector;
                    self.opens[h].size = new_size;
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 4,
                        data_lba,
                        new_size,
                        root_slot,
                        root_sec_off,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            4 => {
                let lba = self.bpb.root_lba + root_sec_off;
                let sector = self.root_buf;
                if self.io.poll_write_sector(lba, &sector) {
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
        let h = handle as usize;
        if h >= MAX_HANDLES || !self.opens[h].in_use {
            return Some(FsResponse::Error);
        }
        match step {
            0 => {
                let cluster = self.opens[h].first_cluster;
                let lba = self.bpb.cluster_to_lba(cluster);
                self.fs_job = FsJob::Read {
                    handle,
                    offset,
                    len,
                    step: 1,
                    data_lba: lba,
                };
                self.advance_fs_job()
            }
            1 => {
                if let Some(sector) = self.io.poll_read_sector(data_lba) {
                    let size = self.opens[h].size as usize;
                    let off = offset as usize;
                    let want = len as usize;
                    if off >= SECTOR_SIZE {
                        return Some(FsResponse::Error);
                    }
                    let avail = min(size, SECTOR_SIZE).saturating_sub(off);
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
        root_sec: u16,
    ) -> Option<FsResponse> {
        let job = FsJob::Stat {
            path,
            path_len,
            step,
            root_sec,
        };
        let name = &path[..path_len as usize];
        let Some(short) = path_to_short_name(name) else {
            return Some(FsResponse::Error);
        };
        match step {
            0 => {
                if !self.mounted {
                    return Some(FsResponse::Error);
                }
                self.fs_job = FsJob::Stat {
                    path,
                    path_len,
                    step: 1,
                    root_sec: 0,
                };
                self.advance_fs_job()
            }
            1 => {
                let root_sectors = self.bpb.root_sectors() as u16;
                if root_sec >= root_sectors {
                    return Some(FsResponse::Error);
                }
                let lba = self.bpb.root_lba + u32::from(root_sec);
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    if let Some((_, e)) = Self::find_in_root_sector(&sector, &short) {
                        return Some(FsResponse::Stat {
                            size: e.size,
                            is_dir: false,
                        });
                    }
                    let mut ended = false;
                    for index in 0..lerux_fat::ENTRIES_PER_SECTOR {
                        if decode_dir_entry(&sector, index).is_end() {
                            ended = true;
                            break;
                        }
                    }
                    if ended || root_sec + 1 >= root_sectors {
                        return Some(FsResponse::Error);
                    }
                    self.fs_job = FsJob::Stat {
                        path,
                        path_len,
                        step: 1,
                        root_sec: root_sec + 1,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn advance_list_dir(
        &mut self,
        step: u8,
        root_sec: u16,
        out_count: u8,
        entries: [FsDirEntry; MAX_FS_DIR_LIST],
    ) -> Option<FsResponse> {
        let job = FsJob::ListDir {
            step,
            root_sec,
            out_count,
            entries,
        };
        match step {
            0 => {
                if !self.mounted {
                    return Some(FsResponse::DirList {
                        count: 0,
                        entries: [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST],
                    });
                }
                self.fs_job = FsJob::ListDir {
                    step: 1,
                    root_sec: 0,
                    out_count: 0,
                    entries: [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST],
                };
                self.advance_fs_job()
            }
            1 => {
                let root_sectors = self.bpb.root_sectors() as u16;
                if root_sec >= root_sectors || out_count as usize >= MAX_FS_DIR_LIST {
                    return Some(FsResponse::DirList {
                        count: out_count,
                        entries,
                    });
                }
                let lba = self.bpb.root_lba + u32::from(root_sec);
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    let mut count = out_count;
                    let mut ents = entries;
                    let mut ended = false;
                    for index in 0..lerux_fat::ENTRIES_PER_SECTOR {
                        let e = decode_dir_entry(&sector, index);
                        if e.is_end() {
                            ended = true;
                            break;
                        }
                        if e.is_free() || e.is_volume_or_lfn() || e.is_dir() {
                            continue;
                        }
                        if (count as usize) < MAX_FS_DIR_LIST {
                            let mut disp = [0u8; 12];
                            let nlen = short_name_to_display(&e.name, &mut disp);
                            // Prefer lowercase path style for smoke: keep uppercase 8.3 display
                            // fs-client expects "ping" — convert to lowercase for pure-alpha names.
                            let mut name_buf = [0u8; 12];
                            let nl = nlen as usize;
                            for i in 0..nl {
                                let b = disp[i];
                                name_buf[i] = if b.is_ascii_uppercase() { b + 32 } else { b };
                            }
                            // 8.3 stores uppercase; clients use lowercase short names (e.g. "ping").
                            ents[count as usize] =
                                FsDirEntry::from_name_size(&name_buf[..nl], e.size);
                            count += 1;
                        }
                    }
                    if ended || count as usize >= MAX_FS_DIR_LIST || root_sec + 1 >= root_sectors {
                        return Some(FsResponse::DirList {
                            count,
                            entries: ents,
                        });
                    }
                    self.fs_job = FsJob::ListDir {
                        step: 1,
                        root_sec: root_sec + 1,
                        out_count: count,
                        entries: ents,
                    };
                    return self.advance_fs_job();
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
            FsRequest::Open { path_len, path } => self.begin_open(path, path_len),
            FsRequest::Create { path_len, path } => self.begin_create(path, path_len),
            FsRequest::Write {
                handle,
                offset,
                data_len,
                data,
            } => self.begin_write(handle, offset, data, data_len),
            FsRequest::Read {
                handle,
                offset,
                len,
            } => self.begin_read(handle, offset, len),
            FsRequest::Stat { path_len, path } => self.begin_stat(path, path_len),
            FsRequest::ListDir { path_len, path } => {
                // Root-only FAT: only "" or "/" are accepted.
                let p = &path[..path_len as usize];
                if path_len == 0 || p == b"/" || p == b"." {
                    self.begin_list_dir();
                } else {
                    return FsResponse::Error;
                }
            }
            FsRequest::Mkdir { .. } | FsRequest::Unlink { .. } | FsRequest::Rename { .. } => {
                // Hierarchy ops are LERUXFS2-only (Phase 50); FAT stays root-only.
                return FsResponse::Error;
            }
            FsRequest::DiskInfo => {
                // Approximate single-partition capacity (Phase 53 shell `df`).
                if !self.mounted {
                    return FsResponse::Error;
                }
                let data = self.bpb.total_sectors.saturating_sub(self.bpb.data_lba);
                return FsResponse::DiskInfo {
                    block_size: u32::from(self.bpb.bytes_per_sector),
                    total_blocks: data,
                    // Free-cluster walk is not tracked; report capacity only.
                    free_blocks: 0,
                };
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
        if self.io.io_busy() {
            BLK_DRIVER.notify();
        }
        FsResponse::Pending
    }
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
            self.io.handle_blk_driver();
        }
        Ok(())
    }
}
