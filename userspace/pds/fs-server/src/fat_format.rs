//! FAT16 format adapter (Phase 44 + Phase 50 subdirs/LFN).
//!
//! Hierarchical directories (cluster chains with `.` / `..`), VFAT long names,
//! and `Mkdir` / `Unlink` / `Rename`. Files still span up to
//! [`lerux_fat::MAX_FILE_CLUSTERS`] via FAT chains.

use core::cmp::min;

use lerux_fat::{
    clear_dir_entry, decode_bpb, decode_dir_entry, decode_lfn_entry, dot_entry, dotdot_entry,
    encode_boot_sector, encode_dir_entry, encode_fat_first_sector, encode_lfn_entry,
    encode_lfn_run, encode_zero_sector, entry_matches, fat_get, fat_sector_index, fat_set,
    file_cluster_index, file_cluster_offset, format_payload_is_fat_head, format_payload_lba,
    format_payload_sectors, is_data_cluster, is_eoc, is_pure_short_name, make_short_alias,
    short_name_to_display, slots_for_name, Bpb, DirEntry, DirLoc, LfnBuilder, LfnPiece, ShortName,
    ATTR_ARCHIVE, ATTR_DIR, ATTR_VOLUME, BOOT_LBA, ENTRIES_PER_SECTOR, EOC, FREE,
    LFN_CHARS_PER_ENTRY, MAX_DIR_CLUSTERS, MAX_FILE_BYTES, MAX_FILE_CLUSTERS, MAX_HANDLES,
    MAX_LFN_ENTRIES,
};
use lerux_fs::{split_path, PathParts};
use lerux_interface_types::{
    FsDirEntry, FsRequest, FsResponse, MAX_FS_DATA, MAX_FS_DIR_LIST, MAX_FS_NAME, MAX_FS_PATH,
    SECTOR_SIZE,
};
use lerux_logging::log;

use crate::{block_io::SectorIo, shell::FsFormat};

#[derive(Clone, Copy)]
struct OpenFile {
    in_use: bool,
    first_cluster: u16,
    size: u32,
    /// LBA of the directory sector that holds the 8.3 dirent.
    dir_lba: u32,
    slot_in_sector: u8,
}

impl OpenFile {
    const fn empty() -> Self {
        Self {
            in_use: false,
            first_cluster: 0,
            size: 0,
            dir_lba: 0,
            slot_in_sector: 0,
        }
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

#[derive(Clone, Copy)]
enum PathPhase {
    Mount,
    Init,
    ScanDir,
    ScanFat,
    Leaf,
    ListDir,
    ListFat,
    AllocFat,
    AllocWrite1,
    AllocWrite2,
    InitDir,
    WriteDirent,
    WriteDirentFlush,
    UnlinkEmpty,
    UnlinkEmptyFat,
    UnlinkFree,
    UnlinkFreeWrite1,
    UnlinkFreeWrite2,
    UnlinkMark,
    UnlinkMarkFlush,
    RenameDestInit,
    RenameWrite,
    RenameWriteFlush,
    RenameDel,
    RenameDelFlush,
}

#[derive(Clone, Copy)]
struct ScanState {
    loc: DirLoc,
    cluster: u16,
    sec_off: u16,
    slot_base: u16,
    done: bool,
    need_fat: bool,
    lfn: LfnBuilder,
    found: bool,
    found_slot: u16,
    found_lba: u32,
    found_lfn_start: u16,
    found_cluster: u16,
    found_size: u32,
    found_attr: u8,
    need_free: u8,
    sector_free_start: u8,
    sector_free_len: u8,
    have_free: bool,
    free_slot: u16,
    free_lba: u32,
    saw_user: bool,
}

impl ScanState {
    fn start(loc: DirLoc, need_free: u8) -> Self {
        let cluster = match loc {
            DirLoc::Root => 0,
            DirLoc::Cluster(c) => c,
        };
        Self {
            loc,
            cluster,
            sec_off: 0,
            slot_base: 0,
            done: false,
            need_fat: false,
            lfn: LfnBuilder::new(),
            found: false,
            found_slot: 0,
            found_lba: 0,
            found_lfn_start: 0,
            found_cluster: 0,
            found_size: 0,
            found_attr: 0,
            need_free,
            sector_free_start: 0xFF,
            sector_free_len: 0,
            have_free: false,
            free_slot: 0,
            free_lba: 0,
            saw_user: false,
        }
    }

    fn current_lba(&self, bpb: &Bpb) -> u32 {
        match self.loc {
            DirLoc::Root => bpb.root_lba + u32::from(self.sec_off),
            DirLoc::Cluster(_) => bpb.cluster_to_lba(self.cluster),
        }
    }
}

#[derive(Clone, Copy)]
struct PathJob {
    op: PathOp,
    phase: PathPhase,
    path: [u8; MAX_FS_PATH],
    path_len: u8,
    to_path: [u8; MAX_FS_PATH],
    to_path_len: u8,
    parts: PathParts,
    comp_i: u8,
    scan: ScanState,
    alloc_cluster: u16,
    fat_sec: u32,
    making_parent: bool,
    rename_dest: bool,
    chosen_short: ShortName,
    chosen_seq: u8,
    src_lba: u32,
    src_slot: u16,
    src_lfn_start: u16,
    src_cluster: u16,
    src_size: u32,
    src_attr: u8,
    free_cluster: u16,
    out_count: u8,
    entries: [FsDirEntry; MAX_FS_DIR_LIST],
}

impl PathJob {
    fn new(
        op: PathOp,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        to_path: [u8; MAX_FS_PATH],
        to_path_len: u8,
    ) -> Self {
        Self {
            op,
            phase: PathPhase::Mount,
            path,
            path_len,
            to_path,
            to_path_len,
            parts: PathParts::empty(),
            comp_i: 0,
            scan: ScanState::start(DirLoc::Root, 0),
            alloc_cluster: 2,
            fat_sec: 0,
            making_parent: false,
            rename_dest: false,
            chosen_short: [b' '; 11],
            chosen_seq: 1,
            src_lba: 0,
            src_slot: 0,
            src_lfn_start: 0,
            src_cluster: 0,
            src_size: 0,
            src_attr: 0,
            free_cluster: 0,
            out_count: 0,
            entries: [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST],
        }
    }

    fn want(&self) -> Option<&[u8]> {
        self.parts.component(self.comp_i as usize)
    }

    fn is_last(&self) -> bool {
        self.parts.count == 0 || self.comp_i + 1 >= self.parts.count
    }
}

#[expect(clippy::large_enum_variant)] // PathJob holds a sector-sized dir scan buffer
enum FsJob {
    None,
    /// step0: read boot; step1: write boot; step2+: write FAT/root payload sectors
    Format {
        step: u8,
        payload_i: u32,
    },
    Path(PathJob),
    Write {
        handle: u8,
        offset: u32,
        data: [u8; MAX_FS_DATA],
        data_len: u16,
        step: u8,
        data_lba: u32,
        new_size: u32,
        dir_lba: u32,
        slot_in_sector: u8,
        need_idx: u16,
        cluster_off: u32,
        walk_cluster: u16,
        walk_idx: u16,
        fat_sec: u32,
        alloc_cluster: u16,
    },
    Read {
        handle: u8,
        offset: u32,
        len: u16,
        step: u8,
        data_lba: u32,
        need_idx: u16,
        cluster_off: u32,
        walk_cluster: u16,
        walk_idx: u16,
    },
}

/// FAT16 [`FsFormat`] adapter.
pub struct FatFormat {
    io: SectorIo,
    bpb: Bpb,
    mounted: bool,
    fs_job: FsJob,
    after_format: Option<FsJob>,
    opens: [OpenFile; MAX_HANDLES],
    /// Scratch for directory sector currently being edited.
    root_buf: [u8; SECTOR_SIZE],
    fat_buf: [u8; SECTOR_SIZE],
}

fn display_name(short: &ShortName, lfn: Option<&[u8]>, out: &mut [u8; MAX_FS_NAME]) -> u8 {
    if let Some(n) = lfn {
        let len = n.len().min(MAX_FS_NAME);
        out[..len].copy_from_slice(&n[..len]);
        return len as u8;
    }
    let mut disp = [0u8; 12];
    let nlen = short_name_to_display(short, &mut disp);
    for i in 0..nlen as usize {
        let b = disp[i];
        out[i] = if b.is_ascii_uppercase() { b + 32 } else { b };
    }
    nlen
}

fn paint_named_entry(
    sector: &mut [u8; SECTOR_SIZE],
    first_index: usize,
    name: &[u8],
    short: &ShortName,
    attr: u8,
    cluster: u16,
    size: u32,
) {
    let mut idx = first_index;
    if !is_pure_short_name(name) {
        let mut pieces = [LfnPiece {
            seq: 0,
            last: false,
            checksum: 0,
            chars: [0; LFN_CHARS_PER_ENTRY],
        }; MAX_LFN_ENTRIES];
        let n = encode_lfn_run(name, short, &mut pieces);
        for (i, piece) in pieces.iter().take(n as usize).enumerate() {
            encode_lfn_entry(sector, idx + i, piece);
        }
        idx += n as usize;
    }
    encode_dir_entry(
        sector,
        idx,
        &DirEntry {
            name: *short,
            attr,
            first_cluster: cluster,
            size,
        },
    );
}

fn scan_dir_sector(
    scan: &mut ScanState,
    sector: &[u8; SECTOR_SIZE],
    lba: u32,
    want: Option<&[u8]>,
    chosen_short: &mut ShortName,
    chosen_seq: &mut u8,
    bump_alias: bool,
) {
    scan.sector_free_start = 0xFF;
    scan.sector_free_len = 0;
    for index in 0..ENTRIES_PER_SECTOR {
        let slot = scan.slot_base + index as u16;
        let e = decode_dir_entry(sector, index);
        if e.is_end() {
            let remain = (ENTRIES_PER_SECTOR - index) as u8;
            if !scan.have_free && scan.need_free > 0 && remain >= scan.need_free {
                scan.have_free = true;
                scan.free_slot = slot;
                scan.free_lba = lba;
            }
            scan.done = true;
            return;
        }
        if let Some(piece) = decode_lfn_entry(sector, index) {
            scan.lfn.feed(slot, &piece);
            scan.sector_free_start = 0xFF;
            scan.sector_free_len = 0;
            continue;
        }
        if e.is_free() {
            scan.lfn.reset();
            if scan.sector_free_start == 0xFF {
                scan.sector_free_start = index as u8;
                scan.sector_free_len = 1;
            } else {
                scan.sector_free_len = scan.sector_free_len.saturating_add(1);
            }
            if !scan.have_free && scan.need_free > 0 && scan.sector_free_len >= scan.need_free {
                scan.have_free = true;
                scan.free_slot = scan.slot_base + u16::from(scan.sector_free_start);
                scan.free_lba = lba;
            }
            continue;
        }
        scan.sector_free_start = 0xFF;
        scan.sector_free_len = 0;
        if e.attr & ATTR_VOLUME != 0 {
            scan.lfn.reset();
            continue;
        }
        let taken = scan.lfn.take(&e.name);
        if e.is_dot() {
            continue;
        }
        scan.saw_user = true;
        if bump_alias
            && let Some(want_name) = want
            && !is_pure_short_name(want_name)
            && e.name == *chosen_short
        {
            *chosen_seq = chosen_seq.saturating_add(1);
            if let Some(next) = make_short_alias(want_name, *chosen_seq) {
                *chosen_short = next;
            }
        }
        if scan.found {
            continue;
        }
        if let Some(want_name) = want {
            let lfn = taken.as_ref().map(|(buf, len, _)| &buf[..*len as usize]);
            if entry_matches(&e.name, lfn, want_name) {
                scan.found = true;
                scan.found_slot = slot;
                scan.found_lba = lba;
                scan.found_lfn_start = taken.map(|(_, _, s)| s).unwrap_or(slot);
                scan.found_cluster = e.first_cluster;
                scan.found_size = e.size;
                scan.found_attr = e.attr;
            }
        }
    }
}

fn collect_list_entries(
    sector: &[u8; SECTOR_SIZE],
    slot_base: u16,
    lfn: &mut LfnBuilder,
    entries: &mut [FsDirEntry; MAX_FS_DIR_LIST],
    count: &mut u8,
) -> bool {
    for index in 0..ENTRIES_PER_SECTOR {
        let slot = slot_base + index as u16;
        let e = decode_dir_entry(sector, index);
        if e.is_end() {
            return true;
        }
        if let Some(piece) = decode_lfn_entry(sector, index) {
            lfn.feed(slot, &piece);
            continue;
        }
        if e.is_free() || e.attr & ATTR_VOLUME != 0 {
            lfn.reset();
            continue;
        }
        let taken = lfn.take(&e.name);
        if e.is_dot() {
            continue;
        }
        if (*count as usize) >= MAX_FS_DIR_LIST {
            continue;
        }
        let lfn_slice = taken.as_ref().map(|(buf, len, _)| &buf[..*len as usize]);
        let mut name = [0u8; MAX_FS_NAME];
        let nlen = display_name(&e.name, lfn_slice, &mut name);
        entries[*count as usize] =
            FsDirEntry::from_name(&name[..nlen as usize], e.size, e.is_dir());
        *count += 1;
    }
    false
}

fn advance_scan_cursor(scan: &mut ScanState, bpb: &Bpb) {
    if scan.done {
        return;
    }
    match scan.loc {
        DirLoc::Root => {
            scan.sec_off = scan.sec_off.saturating_add(1);
            scan.slot_base = scan.slot_base.saturating_add(ENTRIES_PER_SECTOR as u16);
            if u32::from(scan.sec_off) >= bpb.root_sectors() {
                scan.done = true;
            }
        }
        DirLoc::Cluster(_) => {
            scan.need_fat = true;
        }
    }
}

fn apply_fat_next(scan: &mut ScanState, next: u16) {
    scan.need_fat = false;
    if is_data_cluster(next) {
        scan.cluster = next;
        scan.slot_base = scan.slot_base.saturating_add(ENTRIES_PER_SECTOR as u16);
        let clusters = scan.slot_base / ENTRIES_PER_SECTOR as u16;
        if clusters >= MAX_DIR_CLUSTERS {
            scan.done = true;
        }
    } else {
        scan.done = true;
    }
}

fn found_is_dir(scan: &ScanState) -> bool {
    scan.found_attr & ATTR_DIR != 0
}

impl FatFormat {
    pub fn new(block_size: usize) -> FatFormat {
        log::info!("lerux-fs: ready (FAT16)");
        FatFormat {
            io: SectorIo::new(block_size),
            bpb: Bpb::fixed(),
            mounted: false,
            fs_job: FsJob::None,
            after_format: None,
            opens: [OpenFile::empty(); MAX_HANDLES],
            root_buf: [0; SECTOR_SIZE],
            fat_buf: [0; SECTOR_SIZE],
        }
    }

    fn alloc_handle(
        &mut self,
        cluster: u16,
        size: u32,
        dir_lba: u32,
        slot_in_sector: u8,
    ) -> Option<u8> {
        for (i, slot) in self.opens.iter_mut().enumerate() {
            if !slot.in_use {
                *slot = OpenFile {
                    in_use: true,
                    first_cluster: cluster,
                    size,
                    dir_lba,
                    slot_in_sector,
                };
                return Some(i as u8);
            }
        }
        None
    }

    fn begin_path(
        &mut self,
        op: PathOp,
        path: [u8; MAX_FS_PATH],
        path_len: u8,
        to_path: [u8; MAX_FS_PATH],
        to_path_len: u8,
    ) {
        self.fs_job = FsJob::Path(PathJob::new(op, path, path_len, to_path, to_path_len));
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
            dir_lba: 0,
            slot_in_sector: 0,
            need_idx: 0,
            cluster_off: 0,
            walk_cluster: 0,
            walk_idx: 0,
            fat_sec: 0,
            alloc_cluster: 0,
        };
    }

    fn begin_read(&mut self, handle: u8, offset: u32, len: u16) {
        self.fs_job = FsJob::Read {
            handle,
            offset,
            len,
            step: 0,
            data_lba: 0,
            need_idx: 0,
            cluster_off: 0,
            walk_cluster: 0,
            walk_idx: 0,
        };
    }

    fn restore_job(&mut self, job: FsJob) {
        self.fs_job = job;
    }

    fn advance_fs_job(&mut self) -> Option<FsResponse> {
        match core::mem::replace(&mut self.fs_job, FsJob::None) {
            FsJob::None => None,
            FsJob::Format { step, payload_i } => self.advance_format(step, payload_i),
            FsJob::Path(pj) => self.advance_path(pj),
            FsJob::Write {
                handle,
                offset,
                data,
                data_len,
                step,
                data_lba,
                new_size,
                dir_lba,
                slot_in_sector,
                need_idx,
                cluster_off,
                walk_cluster,
                walk_idx,
                fat_sec,
                alloc_cluster,
            } => self.advance_write(
                handle,
                offset,
                data,
                data_len,
                step,
                data_lba,
                new_size,
                dir_lba,
                slot_in_sector,
                need_idx,
                cluster_off,
                walk_cluster,
                walk_idx,
                fat_sec,
                alloc_cluster,
            ),
            FsJob::Read {
                handle,
                offset,
                len,
                step,
                data_lba,
                need_idx,
                cluster_off,
                walk_cluster,
                walk_idx,
            } => self.advance_read(
                handle,
                offset,
                len,
                step,
                data_lba,
                need_idx,
                cluster_off,
                walk_cluster,
                walk_idx,
            ),
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

    fn prepare_component(pj: &mut PathJob) {
        let Some(name) = pj.want() else {
            pj.scan.need_free = 0;
            return;
        };
        let mut name_buf = [0u8; MAX_FS_NAME];
        let n = name.len().min(MAX_FS_NAME);
        name_buf[..n].copy_from_slice(&name[..n]);
        let name = &name_buf[..n];
        pj.chosen_seq = 1;
        pj.chosen_short = make_short_alias(name, 1).unwrap_or([b' '; 11]);
        let last = pj.is_last();
        pj.scan.need_free = if last {
            match pj.op {
                PathOp::Create | PathOp::Mkdir => slots_for_name(name),
                PathOp::RenameFrom if pj.rename_dest => slots_for_name(name),
                _ => 0,
            }
        } else if !pj.rename_dest && matches!(pj.op, PathOp::Create | PathOp::Mkdir) {
            slots_for_name(name)
        } else {
            0
        };
        pj.scan.have_free = false;
        pj.scan.found = false;
        pj.scan.done = false;
        pj.scan.need_fat = false;
        pj.scan.lfn.reset();
        pj.scan.saw_user = false;
    }

    fn start_list(pj: &mut PathJob, loc: DirLoc) {
        pj.scan = ScanState::start(loc, 0);
        pj.out_count = 0;
        pj.entries = [FsDirEntry::from_name_size(&[], 0); MAX_FS_DIR_LIST];
        pj.phase = PathPhase::ListDir;
    }

    fn restart_in(&mut self, mut pj: PathJob, loc: DirLoc) -> Option<FsResponse> {
        pj.scan = ScanState::start(loc, 0);
        Self::prepare_component(&mut pj);
        pj.phase = PathPhase::ScanDir;
        self.fs_job = FsJob::Path(pj);
        self.advance_fs_job()
    }

    fn go(&mut self, mut pj: PathJob, phase: PathPhase) -> Option<FsResponse> {
        pj.phase = phase;
        self.fs_job = FsJob::Path(pj);
        self.advance_fs_job()
    }

    fn bump_alias(pj: &PathJob) -> bool {
        matches!(pj.op, PathOp::Create | PathOp::Mkdir)
            || (matches!(pj.op, PathOp::RenameFrom) && pj.rename_dest)
    }

    fn advance_path(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        match pj.phase {
            PathPhase::Mount => {
                pj.phase = PathPhase::Init;
                self.maybe_mount_then(FsJob::Path(pj))
            }
            PathPhase::Init => self.path_init(pj),
            PathPhase::ScanDir => self.path_scan_dir(pj),
            PathPhase::ScanFat => self.path_scan_fat(pj),
            PathPhase::Leaf => self.path_leaf(pj),
            PathPhase::ListDir => self.path_list_dir(pj),
            PathPhase::ListFat => self.path_list_fat(pj),
            PathPhase::AllocFat => self.path_alloc_fat(pj),
            PathPhase::AllocWrite1 => self.path_alloc_write(pj, false),
            PathPhase::AllocWrite2 => self.path_alloc_write(pj, true),
            PathPhase::InitDir => self.path_init_dir(pj),
            PathPhase::WriteDirent => self.path_write_dirent(pj),
            PathPhase::WriteDirentFlush => self.path_write_dirent_flush(pj),
            PathPhase::UnlinkEmpty => self.path_unlink_empty(pj),
            PathPhase::UnlinkEmptyFat => self.path_unlink_empty_fat(pj),
            PathPhase::UnlinkFree => self.path_unlink_free(pj),
            PathPhase::UnlinkFreeWrite1 => self.path_unlink_free_write(pj, false),
            PathPhase::UnlinkFreeWrite2 => self.path_unlink_free_write(pj, true),
            PathPhase::UnlinkMark => self.path_unlink_mark(pj, false),
            PathPhase::UnlinkMarkFlush => self.path_mark_flush(pj, false),
            PathPhase::RenameDestInit => self.path_rename_dest_init(pj),
            PathPhase::RenameWrite => self.path_rename_write(pj),
            PathPhase::RenameWriteFlush => self.path_rename_write_flush(pj),
            PathPhase::RenameDel => self.path_unlink_mark(pj, true),
            PathPhase::RenameDelFlush => self.path_mark_flush(pj, true),
        }
    }

    fn path_init(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let path = &pj.path[..pj.path_len as usize];
        let Ok(parts) = split_path(path) else {
            return Some(FsResponse::Error);
        };
        pj.parts = parts;
        if pj.parts.count == 0 {
            return match pj.op {
                PathOp::ListDir => {
                    Self::start_list(&mut pj, DirLoc::Root);
                    self.fs_job = FsJob::Path(pj);
                    self.advance_fs_job()
                }
                PathOp::Stat => Some(FsResponse::Stat {
                    size: 0,
                    is_dir: true,
                }),
                _ => Some(FsResponse::Error),
            };
        }
        pj.comp_i = 0;
        pj.scan = ScanState::start(DirLoc::Root, 0);
        Self::prepare_component(&mut pj);
        self.go(pj, PathPhase::ScanDir)
    }

    fn path_scan_dir(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = pj.scan.current_lba(&self.bpb);
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let bump = Self::bump_alias(&pj);
        let want = pj.want().map(|s| {
            let mut buf = [0u8; MAX_FS_NAME];
            let n = s.len().min(MAX_FS_NAME);
            buf[..n].copy_from_slice(&s[..n]);
            (buf, n)
        });
        let want_ref = want.as_ref().map(|(b, n)| &b[..*n]);
        scan_dir_sector(
            &mut pj.scan,
            &sector,
            lba,
            want_ref,
            &mut pj.chosen_short,
            &mut pj.chosen_seq,
            bump,
        );
        if pj.scan.found
            && matches!(
                pj.op,
                PathOp::Open | PathOp::Stat | PathOp::ListDir | PathOp::Unlink
            )
            && pj.is_last()
            && !matches!(pj.op, PathOp::Create | PathOp::Mkdir)
        {
            return self.go(pj, PathPhase::Leaf);
        }
        if pj.scan.found && pj.is_last() && matches!(pj.op, PathOp::RenameFrom) && !pj.rename_dest {
            return self.go(pj, PathPhase::Leaf);
        }
        if pj.scan.done {
            return self.go(pj, PathPhase::Leaf);
        }
        advance_scan_cursor(&mut pj.scan, &self.bpb);
        if pj.scan.need_fat {
            return self.go(pj, PathPhase::ScanFat);
        }
        if pj.scan.done {
            return self.go(pj, PathPhase::Leaf);
        }
        self.go(pj, PathPhase::ScanDir)
    }

    fn path_scan_fat(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let (sec_i, idx) = fat_sector_index(pj.scan.cluster);
        let lba = self.bpb.fat1_lba + sec_i;
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let next = fat_get(&sector, idx);
        apply_fat_next(&mut pj.scan, next);
        if pj.scan.done {
            return self.go(pj, PathPhase::Leaf);
        }
        self.go(pj, PathPhase::ScanDir)
    }

    fn path_leaf(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        if !pj.is_last() {
            if pj.scan.found {
                if !found_is_dir(&pj.scan) {
                    return Some(FsResponse::Error);
                }
                pj.comp_i += 1;
                return self.restart_in(pj, DirLoc::Cluster(pj.scan.found_cluster));
            }
            if !pj.rename_dest && matches!(pj.op, PathOp::Create | PathOp::Mkdir) {
                if !pj.scan.have_free {
                    return Some(FsResponse::Error);
                }
                pj.making_parent = true;
                pj.alloc_cluster = 2;
                return self.go(pj, PathPhase::AllocFat);
            }
            return Some(FsResponse::Error);
        }

        match pj.op {
            PathOp::Open => {
                if !pj.scan.found || found_is_dir(&pj.scan) {
                    return Some(FsResponse::Error);
                }
                let idx = (pj.scan.found_slot % ENTRIES_PER_SECTOR as u16) as u8;
                let Some(id) = self.alloc_handle(
                    pj.scan.found_cluster,
                    pj.scan.found_size,
                    pj.scan.found_lba,
                    idx,
                ) else {
                    return Some(FsResponse::Error);
                };
                Some(FsResponse::Handle { id })
            }
            PathOp::Stat => {
                if !pj.scan.found {
                    return Some(FsResponse::Error);
                }
                Some(FsResponse::Stat {
                    size: if found_is_dir(&pj.scan) {
                        0
                    } else {
                        pj.scan.found_size
                    },
                    is_dir: found_is_dir(&pj.scan),
                })
            }
            PathOp::ListDir => {
                if !pj.scan.found || !found_is_dir(&pj.scan) {
                    return Some(FsResponse::Error);
                }
                let cluster = pj.scan.found_cluster;
                Self::start_list(&mut pj, DirLoc::Cluster(cluster));
                self.fs_job = FsJob::Path(pj);
                self.advance_fs_job()
            }
            PathOp::Create | PathOp::Mkdir => {
                if pj.scan.found {
                    return Some(FsResponse::Error);
                }
                if !pj.scan.have_free {
                    return Some(FsResponse::Error);
                }
                pj.making_parent = false;
                pj.alloc_cluster = 2;
                self.go(pj, PathPhase::AllocFat)
            }
            PathOp::Unlink => {
                if !pj.scan.found {
                    return Some(FsResponse::Error);
                }
                if found_is_dir(&pj.scan) {
                    let loc = DirLoc::Cluster(pj.scan.found_cluster);
                    pj.src_cluster = pj.scan.found_cluster;
                    pj.src_lba = pj.scan.found_lba;
                    pj.src_slot = pj.scan.found_slot;
                    pj.src_lfn_start = pj.scan.found_lfn_start;
                    pj.src_attr = pj.scan.found_attr;
                    pj.scan = ScanState::start(loc, 0);
                    return self.go(pj, PathPhase::UnlinkEmpty);
                }
                pj.src_cluster = pj.scan.found_cluster;
                pj.src_lba = pj.scan.found_lba;
                pj.src_slot = pj.scan.found_slot;
                pj.src_lfn_start = pj.scan.found_lfn_start;
                pj.free_cluster = pj.scan.found_cluster;
                self.go(pj, PathPhase::UnlinkFree)
            }
            PathOp::RenameFrom => {
                if pj.rename_dest {
                    if pj.scan.found || !pj.scan.have_free {
                        return Some(FsResponse::Error);
                    }
                    return self.go(pj, PathPhase::RenameWrite);
                }
                if !pj.scan.found {
                    return Some(FsResponse::Error);
                }
                self.go(pj, PathPhase::RenameDestInit)
            }
        }
    }

    fn path_list_dir(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        if pj.out_count as usize >= MAX_FS_DIR_LIST || pj.scan.done {
            return Some(FsResponse::DirList {
                count: pj.out_count,
                entries: pj.entries,
            });
        }
        let job = FsJob::Path(pj);
        let lba = pj.scan.current_lba(&self.bpb);
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let ended = collect_list_entries(
            &sector,
            pj.scan.slot_base,
            &mut pj.scan.lfn,
            &mut pj.entries,
            &mut pj.out_count,
        );
        if ended {
            return Some(FsResponse::DirList {
                count: pj.out_count,
                entries: pj.entries,
            });
        }
        advance_scan_cursor(&mut pj.scan, &self.bpb);
        if pj.scan.need_fat {
            return self.go(pj, PathPhase::ListFat);
        }
        if pj.scan.done {
            return Some(FsResponse::DirList {
                count: pj.out_count,
                entries: pj.entries,
            });
        }
        self.go(pj, PathPhase::ListDir)
    }

    fn path_list_fat(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let (sec_i, idx) = fat_sector_index(pj.scan.cluster);
        let lba = self.bpb.fat1_lba + sec_i;
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let next = fat_get(&sector, idx);
        apply_fat_next(&mut pj.scan, next);
        if pj.scan.done {
            return Some(FsResponse::DirList {
                count: pj.out_count,
                entries: pj.entries,
            });
        }
        self.go(pj, PathPhase::ListDir)
    }

    fn path_alloc_fat(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let (sec_i, _idx) = fat_sector_index(pj.alloc_cluster);
        let lba = self.bpb.fat1_lba + sec_i;
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        self.fat_buf = sector;
        let max = self.bpb.max_cluster();
        let mut c = pj.alloc_cluster;
        loop {
            let (s, i) = fat_sector_index(c);
            if s != sec_i {
                pj.alloc_cluster = c;
                return self.go(pj, PathPhase::AllocFat);
            }
            if fat_get(&self.fat_buf, i) == FREE {
                fat_set(&mut self.fat_buf, i, EOC);
                pj.alloc_cluster = c;
                pj.fat_sec = sec_i;
                return self.go(pj, PathPhase::AllocWrite1);
            }
            if c >= max {
                return Some(FsResponse::Error);
            }
            c += 1;
        }
    }

    fn path_alloc_write(&mut self, pj: PathJob, mirror: bool) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = if mirror {
            self.bpb.fat1_lba + u32::from(self.bpb.fat_sectors) + pj.fat_sec
        } else {
            self.bpb.fat1_lba + pj.fat_sec
        };
        let sector = self.fat_buf;
        if !self.io.poll_write_sector(lba, &sector) {
            self.restore_job(job);
            return None;
        }
        if !mirror {
            return self.go(pj, PathPhase::AllocWrite2);
        }
        let make_dir = pj.making_parent || matches!(pj.op, PathOp::Mkdir);
        if make_dir {
            return self.go(pj, PathPhase::InitDir);
        }
        self.go(pj, PathPhase::WriteDirent)
    }

    fn path_init_dir(&mut self, pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = self.bpb.cluster_to_lba(pj.alloc_cluster);
        let mut sector = [0u8; SECTOR_SIZE];
        encode_dir_entry(&mut sector, 0, &dot_entry(pj.alloc_cluster));
        encode_dir_entry(&mut sector, 1, &dotdot_entry(pj.scan.loc.parent_cluster()));
        if !self.io.poll_write_sector(lba, &sector) {
            self.restore_job(job);
            return None;
        }
        self.go(pj, PathPhase::WriteDirent)
    }

    fn path_write_dirent(&mut self, pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = pj.scan.free_lba;
        let Some(mut sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let Some(name) = pj.want() else {
            return Some(FsResponse::Error);
        };
        let name_buf = {
            let mut buf = [0u8; MAX_FS_NAME];
            let n = name.len().min(MAX_FS_NAME);
            buf[..n].copy_from_slice(&name[..n]);
            (buf, n)
        };
        let attr = if pj.making_parent || matches!(pj.op, PathOp::Mkdir) {
            ATTR_DIR
        } else {
            ATTR_ARCHIVE
        };
        let idx = (pj.scan.free_slot % ENTRIES_PER_SECTOR as u16) as usize;
        paint_named_entry(
            &mut sector,
            idx,
            &name_buf.0[..name_buf.1],
            &pj.chosen_short,
            attr,
            pj.alloc_cluster,
            0,
        );
        self.root_buf = sector;
        self.go(pj, PathPhase::WriteDirentFlush)
    }

    fn path_write_dirent_flush(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = pj.scan.free_lba;
        let sector = self.root_buf;
        if !self.io.poll_write_sector(lba, &sector) {
            self.restore_job(job);
            return None;
        }
        if pj.making_parent {
            pj.making_parent = false;
            pj.comp_i += 1;
            return self.restart_in(pj, DirLoc::Cluster(pj.alloc_cluster));
        }
        match pj.op {
            PathOp::Mkdir => Some(FsResponse::Ok),
            PathOp::Create => {
                let Some(name) = pj.want() else {
                    return Some(FsResponse::Error);
                };
                let slots = slots_for_name(name);
                let short_slot = pj.scan.free_slot + u16::from(slots.saturating_sub(1));
                let idx = (short_slot % ENTRIES_PER_SECTOR as u16) as u8;
                let Some(id) = self.alloc_handle(pj.alloc_cluster, 0, pj.scan.free_lba, idx) else {
                    return Some(FsResponse::Error);
                };
                Some(FsResponse::Handle { id })
            }
            _ => Some(FsResponse::Error),
        }
    }

    fn path_unlink_empty(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        if pj.scan.done {
            if pj.scan.saw_user {
                return Some(FsResponse::Error);
            }
            pj.free_cluster = pj.src_cluster;
            return self.go(pj, PathPhase::UnlinkFree);
        }
        let job = FsJob::Path(pj);
        let lba = pj.scan.current_lba(&self.bpb);
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        scan_dir_sector(
            &mut pj.scan,
            &sector,
            lba,
            None,
            &mut pj.chosen_short,
            &mut pj.chosen_seq,
            false,
        );
        if pj.scan.saw_user {
            return Some(FsResponse::Error);
        }
        if pj.scan.done {
            pj.free_cluster = pj.src_cluster;
            return self.go(pj, PathPhase::UnlinkFree);
        }
        advance_scan_cursor(&mut pj.scan, &self.bpb);
        if pj.scan.need_fat {
            return self.go(pj, PathPhase::UnlinkEmptyFat);
        }
        self.go(pj, PathPhase::UnlinkEmpty)
    }

    fn path_unlink_empty_fat(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let (sec_i, idx) = fat_sector_index(pj.scan.cluster);
        let lba = self.bpb.fat1_lba + sec_i;
        let Some(sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        apply_fat_next(&mut pj.scan, fat_get(&sector, idx));
        if pj.scan.done {
            if pj.scan.saw_user {
                return Some(FsResponse::Error);
            }
            pj.free_cluster = pj.src_cluster;
            return self.go(pj, PathPhase::UnlinkFree);
        }
        self.go(pj, PathPhase::UnlinkEmpty)
    }

    fn path_unlink_free(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        if pj.free_cluster < 2 {
            return self.go(pj, PathPhase::UnlinkMark);
        }
        let job = FsJob::Path(pj);
        let (sec_i, idx) = fat_sector_index(pj.free_cluster);
        let lba = self.bpb.fat1_lba + sec_i;
        let Some(mut sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let next = fat_get(&sector, idx);
        fat_set(&mut sector, idx, FREE);
        self.fat_buf = sector;
        pj.fat_sec = sec_i;
        pj.alloc_cluster = next;
        self.go(pj, PathPhase::UnlinkFreeWrite1)
    }

    fn path_unlink_free_write(&mut self, mut pj: PathJob, mirror: bool) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = if mirror {
            self.bpb.fat1_lba + u32::from(self.bpb.fat_sectors) + pj.fat_sec
        } else {
            self.bpb.fat1_lba + pj.fat_sec
        };
        let sector = self.fat_buf;
        if !self.io.poll_write_sector(lba, &sector) {
            self.restore_job(job);
            return None;
        }
        if !mirror {
            return self.go(pj, PathPhase::UnlinkFreeWrite2);
        }
        if is_data_cluster(pj.alloc_cluster) {
            pj.free_cluster = pj.alloc_cluster;
            return self.go(pj, PathPhase::UnlinkFree);
        }
        self.go(pj, PathPhase::UnlinkMark)
    }

    fn path_unlink_mark(&mut self, pj: PathJob, rename: bool) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = pj.src_lba;
        let Some(mut sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let start = pj.src_lfn_start;
        let end = pj.src_slot;
        let same_sector = start / ENTRIES_PER_SECTOR as u16 == end / ENTRIES_PER_SECTOR as u16;
        if same_sector {
            let a = (start % ENTRIES_PER_SECTOR as u16) as usize;
            let b = (end % ENTRIES_PER_SECTOR as u16) as usize;
            for i in a..=b {
                clear_dir_entry(&mut sector, i);
            }
        } else {
            clear_dir_entry(&mut sector, (end % ENTRIES_PER_SECTOR as u16) as usize);
        }
        self.root_buf = sector;
        if rename {
            self.go(pj, PathPhase::RenameDelFlush)
        } else {
            self.go(pj, PathPhase::UnlinkMarkFlush)
        }
    }

    fn path_mark_flush(&mut self, pj: PathJob, rename: bool) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        if !self.io.poll_write_sector(pj.src_lba, &self.root_buf) {
            self.restore_job(job);
            return None;
        }
        let _ = rename;
        Some(FsResponse::Ok)
    }

    fn path_rename_dest_init(&mut self, mut pj: PathJob) -> Option<FsResponse> {
        let to = &pj.to_path[..pj.to_path_len as usize];
        let Ok(to_parts) = split_path(to) else {
            return Some(FsResponse::Error);
        };
        if to_parts.count == 0 {
            return Some(FsResponse::Error);
        }
        let same = pj.parts.count == to_parts.count
            && (0..pj.parts.count as usize).all(|i| {
                pj.parts
                    .component(i)
                    .zip(to_parts.component(i))
                    .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b))
            });
        if same {
            return Some(FsResponse::Ok);
        }
        pj.src_lba = pj.scan.found_lba;
        pj.src_slot = pj.scan.found_slot;
        pj.src_lfn_start = pj.scan.found_lfn_start;
        pj.src_cluster = pj.scan.found_cluster;
        pj.src_size = pj.scan.found_size;
        pj.src_attr = pj.scan.found_attr;
        pj.parts = to_parts;
        pj.comp_i = 0;
        pj.rename_dest = true;
        pj.scan = ScanState::start(DirLoc::Root, 0);
        Self::prepare_component(&mut pj);
        self.go(pj, PathPhase::ScanDir)
    }

    fn path_rename_write(&mut self, pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        let lba = pj.scan.free_lba;
        let Some(mut sector) = self.io.poll_read_sector(lba) else {
            self.restore_job(job);
            return None;
        };
        let Some(name) = pj.want() else {
            return Some(FsResponse::Error);
        };
        let mut name_buf = [0u8; MAX_FS_NAME];
        let n = name.len().min(MAX_FS_NAME);
        name_buf[..n].copy_from_slice(&name[..n]);
        let idx = (pj.scan.free_slot % ENTRIES_PER_SECTOR as u16) as usize;
        paint_named_entry(
            &mut sector,
            idx,
            &name_buf[..n],
            &pj.chosen_short,
            pj.src_attr,
            pj.src_cluster,
            pj.src_size,
        );
        self.root_buf = sector;
        self.go(pj, PathPhase::RenameWriteFlush)
    }

    fn path_rename_write_flush(&mut self, pj: PathJob) -> Option<FsResponse> {
        let job = FsJob::Path(pj);
        if !self.io.poll_write_sector(pj.scan.free_lba, &self.root_buf) {
            self.restore_job(job);
            return None;
        }
        self.go(pj, PathPhase::RenameDel)
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
        dir_lba: u32,
        slot_in_sector: u8,
        need_idx: u16,
        cluster_off: u32,
        walk_cluster: u16,
        walk_idx: u16,
        fat_sec: u32,
        alloc_cluster: u16,
    ) -> Option<FsResponse> {
        let job = FsJob::Write {
            handle,
            offset,
            data,
            data_len,
            step,
            data_lba,
            new_size,
            dir_lba,
            slot_in_sector,
            need_idx,
            cluster_off,
            walk_cluster,
            walk_idx,
            fat_sec,
            alloc_cluster,
        };
        let h = handle as usize;
        if h >= MAX_HANDLES || !self.opens[h].in_use {
            return Some(FsResponse::Error);
        }
        let cl_size = self.bpb.cluster_size_bytes();
        match step {
            0 => {
                let len = data_len as u32;
                if data_len as usize > MAX_FS_DATA || self.opens[h].first_cluster < 2 {
                    return Some(FsResponse::Error);
                }
                let c_off = file_cluster_offset(offset, cl_size);
                if c_off.saturating_add(len) > cl_size {
                    return Some(FsResponse::Error);
                }
                let n_idx = file_cluster_index(offset, cl_size);
                if n_idx >= MAX_FILE_CLUSTERS {
                    return Some(FsResponse::Error);
                }
                let end = offset.saturating_add(len);
                if end > MAX_FILE_BYTES {
                    return Some(FsResponse::Error);
                }
                let size = if offset == 0 && end < self.opens[h].size {
                    end
                } else {
                    self.opens[h].size.max(end)
                };
                self.fs_job = FsJob::Write {
                    handle,
                    offset,
                    data,
                    data_len,
                    step: 1,
                    data_lba: 0,
                    new_size: size,
                    dir_lba: self.opens[h].dir_lba,
                    slot_in_sector: self.opens[h].slot_in_sector,
                    need_idx: n_idx,
                    cluster_off: c_off,
                    walk_cluster: self.opens[h].first_cluster,
                    walk_idx: 0,
                    fat_sec: 0,
                    alloc_cluster: 0,
                };
                self.advance_fs_job()
            }
            1 => {
                if walk_idx == need_idx {
                    let lba = self.bpb.cluster_to_lba(walk_cluster);
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 5,
                        data_lba: lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                let (sec_i, idx) = fat_sector_index(walk_cluster);
                let lba = self.bpb.fat1_lba + sec_i;
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    self.fat_buf = sector;
                    let next = fat_get(&self.fat_buf, idx);
                    if is_data_cluster(next) {
                        self.fs_job = FsJob::Write {
                            handle,
                            offset,
                            data,
                            data_len,
                            step: 1,
                            data_lba,
                            new_size,
                            dir_lba,
                            slot_in_sector,
                            need_idx,
                            cluster_off,
                            walk_cluster: next,
                            walk_idx: walk_idx + 1,
                            fat_sec,
                            alloc_cluster,
                        };
                        return self.advance_fs_job();
                    }
                    if is_eoc(next) {
                        self.fs_job = FsJob::Write {
                            handle,
                            offset,
                            data,
                            data_len,
                            step: 2,
                            data_lba,
                            new_size,
                            dir_lba,
                            slot_in_sector,
                            need_idx,
                            cluster_off,
                            walk_cluster,
                            walk_idx,
                            fat_sec: 0,
                            alloc_cluster: 2,
                        };
                        return self.advance_fs_job();
                    }
                    return Some(FsResponse::Error);
                }
                self.restore_job(job);
                None
            }
            2 => {
                let (sec_i, _idx) = fat_sector_index(alloc_cluster);
                let lba = self.bpb.fat1_lba + sec_i;
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    self.fat_buf = sector;
                    let max = self.bpb.max_cluster();
                    let mut c = alloc_cluster;
                    loop {
                        let (s, i) = fat_sector_index(c);
                        if s != sec_i {
                            self.fs_job = FsJob::Write {
                                handle,
                                offset,
                                data,
                                data_len,
                                step: 2,
                                data_lba,
                                new_size,
                                dir_lba,
                                slot_in_sector,
                                need_idx,
                                cluster_off,
                                walk_cluster,
                                walk_idx,
                                fat_sec: s,
                                alloc_cluster: c,
                            };
                            return self.advance_fs_job();
                        }
                        if fat_get(&self.fat_buf, i) == FREE {
                            self.fs_job = FsJob::Write {
                                handle,
                                offset,
                                data,
                                data_len,
                                step: 3,
                                data_lba,
                                new_size,
                                dir_lba,
                                slot_in_sector,
                                need_idx,
                                cluster_off,
                                walk_cluster,
                                walk_idx,
                                fat_sec: sec_i,
                                alloc_cluster: c,
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
            3 => {
                let (walk_sec, walk_i) = fat_sector_index(walk_cluster);
                let (alloc_sec, alloc_i) = fat_sector_index(alloc_cluster);
                if walk_sec == alloc_sec {
                    let lba = self.bpb.fat1_lba + walk_sec;
                    if let Some(mut sector) = self.io.poll_read_sector(lba) {
                        fat_set(&mut sector, walk_i, alloc_cluster);
                        fat_set(&mut sector, alloc_i, EOC);
                        self.fat_buf = sector;
                        self.fs_job = FsJob::Write {
                            handle,
                            offset,
                            data,
                            data_len,
                            step: 4,
                            data_lba,
                            new_size,
                            dir_lba,
                            slot_in_sector,
                            need_idx,
                            cluster_off,
                            walk_cluster,
                            walk_idx,
                            fat_sec: walk_sec,
                            alloc_cluster,
                        };
                        return self.advance_fs_job();
                    }
                    self.restore_job(job);
                    return None;
                }
                let lba = self.bpb.fat1_lba + walk_sec;
                if let Some(mut sector) = self.io.poll_read_sector(lba) {
                    fat_set(&mut sector, walk_i, alloc_cluster);
                    self.fat_buf = sector;
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 10,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec: walk_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            4 => {
                let lba = self.bpb.fat1_lba + fat_sec;
                let sector = self.fat_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 11,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            11 => {
                let lba = self.bpb.fat1_lba + u32::from(self.bpb.fat_sectors) + fat_sec;
                let sector = self.fat_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 1,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster: alloc_cluster,
                        walk_idx: walk_idx + 1,
                        fat_sec: 0,
                        alloc_cluster: 0,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            10 => {
                let lba = self.bpb.fat1_lba + fat_sec;
                let sector = self.fat_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 12,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            12 => {
                let lba = self.bpb.fat1_lba + u32::from(self.bpb.fat_sectors) + fat_sec;
                let sector = self.fat_buf;
                if self.io.poll_write_sector(lba, &sector) {
                    let (alloc_sec, _) = fat_sector_index(alloc_cluster);
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 13,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec: alloc_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            13 => {
                let (alloc_sec, alloc_i) = fat_sector_index(alloc_cluster);
                let lba = self.bpb.fat1_lba + alloc_sec;
                if let Some(mut sector) = self.io.poll_read_sector(lba) {
                    fat_set(&mut sector, alloc_i, EOC);
                    self.fat_buf = sector;
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 4,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec: alloc_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            5 => {
                if let Some(mut sector) = self.io.poll_read_sector(data_lba) {
                    let off = cluster_off as usize;
                    let len = data_len as usize;
                    if off >= SECTOR_SIZE
                        || len > MAX_FS_DATA
                        || off.saturating_add(len) > SECTOR_SIZE
                    {
                        return Some(FsResponse::Error);
                    }
                    sector[off..off + len].copy_from_slice(&data[..len]);
                    self.io.sector_buf = sector;
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 6,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            6 => {
                let sector = self.io.sector_buf;
                if self.io.poll_write_sector(data_lba, &sector) {
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 7,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            7 => {
                if let Some(mut sector) = self.io.poll_read_sector(dir_lba) {
                    let mut e = decode_dir_entry(&sector, slot_in_sector as usize);
                    e.size = new_size;
                    encode_dir_entry(&mut sector, slot_in_sector as usize, &e);
                    self.root_buf = sector;
                    self.opens[h].size = new_size;
                    self.fs_job = FsJob::Write {
                        handle,
                        offset,
                        data,
                        data_len,
                        step: 8,
                        data_lba,
                        new_size,
                        dir_lba,
                        slot_in_sector,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                        fat_sec,
                        alloc_cluster,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            8 => {
                let sector = self.root_buf;
                if self.io.poll_write_sector(dir_lba, &sector) {
                    return Some(FsResponse::Ok);
                }
                self.restore_job(job);
                None
            }
            _ => Some(FsResponse::Error),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "read job stage state")]
    fn advance_read(
        &mut self,
        handle: u8,
        offset: u32,
        len: u16,
        step: u8,
        data_lba: u32,
        need_idx: u16,
        cluster_off: u32,
        walk_cluster: u16,
        walk_idx: u16,
    ) -> Option<FsResponse> {
        let job = FsJob::Read {
            handle,
            offset,
            len,
            step,
            data_lba,
            need_idx,
            cluster_off,
            walk_cluster,
            walk_idx,
        };
        let h = handle as usize;
        if h >= MAX_HANDLES || !self.opens[h].in_use {
            return Some(FsResponse::Error);
        }
        let cl_size = self.bpb.cluster_size_bytes();
        match step {
            0 => {
                if self.opens[h].first_cluster < 2 {
                    return Some(FsResponse::Error);
                }
                let c_off = file_cluster_offset(offset, cl_size);
                let n_idx = file_cluster_index(offset, cl_size);
                if n_idx >= MAX_FILE_CLUSTERS {
                    return Some(FsResponse::Error);
                }
                if offset >= self.opens[h].size {
                    return Some(FsResponse::Data {
                        data_len: 0,
                        data: [0u8; MAX_FS_DATA],
                    });
                }
                self.fs_job = FsJob::Read {
                    handle,
                    offset,
                    len,
                    step: 1,
                    data_lba: 0,
                    need_idx: n_idx,
                    cluster_off: c_off,
                    walk_cluster: self.opens[h].first_cluster,
                    walk_idx: 0,
                };
                self.advance_fs_job()
            }
            1 => {
                if walk_idx == need_idx {
                    let lba = self.bpb.cluster_to_lba(walk_cluster);
                    self.fs_job = FsJob::Read {
                        handle,
                        offset,
                        len,
                        step: 2,
                        data_lba: lba,
                        need_idx,
                        cluster_off,
                        walk_cluster,
                        walk_idx,
                    };
                    return self.advance_fs_job();
                }
                let (sec_i, idx) = fat_sector_index(walk_cluster);
                let lba = self.bpb.fat1_lba + sec_i;
                if let Some(sector) = self.io.poll_read_sector(lba) {
                    let next = fat_get(&sector, idx);
                    if !is_data_cluster(next) {
                        return Some(FsResponse::Error);
                    }
                    self.fs_job = FsJob::Read {
                        handle,
                        offset,
                        len,
                        step: 1,
                        data_lba,
                        need_idx,
                        cluster_off,
                        walk_cluster: next,
                        walk_idx: walk_idx + 1,
                    };
                    return self.advance_fs_job();
                }
                self.restore_job(job);
                None
            }
            2 => {
                if let Some(sector) = self.io.poll_read_sector(data_lba) {
                    let size = self.opens[h].size as usize;
                    let file_off = offset as usize;
                    let want = len as usize;
                    let off = cluster_off as usize;
                    if off >= SECTOR_SIZE {
                        return Some(FsResponse::Error);
                    }
                    let file_avail = size.saturating_sub(file_off);
                    let sector_avail = SECTOR_SIZE.saturating_sub(off);
                    let copy_len = min(want, min(file_avail, sector_avail)).min(MAX_FS_DATA);
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
}

impl FsFormat for FatFormat {
    fn begin(&mut self, req: FsRequest) -> Option<FsResponse> {
        match req {
            FsRequest::Open { path_len, path } => {
                self.begin_path(PathOp::Open, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Create { path_len, path } => {
                self.begin_path(PathOp::Create, path, path_len, [0; MAX_FS_PATH], 0);
            }
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
            FsRequest::Stat { path_len, path } => {
                self.begin_path(PathOp::Stat, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::ListDir { path_len, path } => {
                self.begin_path(PathOp::ListDir, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Mkdir { path_len, path } => {
                self.begin_path(PathOp::Mkdir, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Unlink { path_len, path } => {
                self.begin_path(PathOp::Unlink, path, path_len, [0; MAX_FS_PATH], 0);
            }
            FsRequest::Rename {
                from_len,
                from,
                to_len,
                to,
            } => self.begin_path(PathOp::RenameFrom, from, from_len, to, to_len),
            FsRequest::DiskInfo => {
                if !self.mounted {
                    return Some(FsResponse::Error);
                }
                let data = self.bpb.total_sectors.saturating_sub(self.bpb.data_lba);
                return Some(FsResponse::DiskInfo {
                    block_size: u32::from(self.bpb.bytes_per_sector),
                    total_blocks: data,
                    free_blocks: 0,
                });
            }
            FsRequest::Poll => unreachable!("shell handles Poll"),
        }
        None
    }

    fn advance(&mut self) -> Option<FsResponse> {
        self.advance_fs_job()
    }

    fn busy(&self) -> bool {
        !matches!(self.fs_job, FsJob::None)
    }

    fn io_busy(&self) -> bool {
        self.io.io_busy()
    }

    fn on_blk_notified(&mut self) -> Option<FsResponse> {
        self.io.handle_blk_driver();
        None
    }
}
