#![no_std]
#![no_main]

use core::cell::UnsafeCell;

use lerux_cothread::{
    microkit_cothread_init, microkit_cothread_recv_ntfn, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, NULL_HANDLE, STACK_SLOTS,
};
use lerux_logging::{debug, log};
use lerux_sddf::{
    blk_client_config_t, blk_dequeue_resp, blk_enqueue_req,
    blk_image::{self, BLK_CLIENT_VIRT_CHANNEL, BLK_DATA_VADDR},
    blk_queue_handle_t, blk_queue_init, blk_req_code_t, blk_resp_status_t, blk_storage_info_t,
    blk_storage_is_ready, fs_buffer_t, fs_cmd_params_dir_create_t, fs_cmd_params_dir_open_t,
    fs_cmd_params_dir_read_t, fs_cmd_params_file_close_t, fs_cmd_params_file_open_t,
    fs_cmd_params_file_read_t, fs_cmd_params_file_remove_t, fs_cmd_params_file_write_t,
    fs_cmd_params_rename_t, fs_cmd_params_stat_t, fs_cmpl_t, fs_completion_enqueue,
    fs_image::{self, FS_REGION_SIZE, FS_SERVER_CLIENT_CHANNEL},
    fs_message_dequeue, fs_msg_t, fs_queue_t, fs_server_config_t, fs_stat_t, FS_CMD_DIR_CLOSE,
    FS_CMD_DIR_CREATE, FS_CMD_DIR_OPEN, FS_CMD_DIR_READ, FS_CMD_FILE_CLOSE, FS_CMD_FILE_OPEN,
    FS_CMD_FILE_READ, FS_CMD_FILE_REMOVE, FS_CMD_FILE_WRITE, FS_CMD_RENAME, FS_CMD_STAT,
    FS_OPEN_FLAGS_CREATE, FS_OPEN_FLAGS_READ_ONLY, FS_OPEN_FLAGS_WRITE_ONLY,
    FS_STATUS_ALREADY_EXISTS, FS_STATUS_DIRECTORY_IS_FULL, FS_STATUS_END_OF_DIRECTORY,
    FS_STATUS_ERROR, FS_STATUS_INVALID_BUFFER, FS_STATUS_INVALID_COMMAND, FS_STATUS_INVALID_FD,
    FS_STATUS_INVALID_NAME, FS_STATUS_INVALID_READ, FS_STATUS_INVALID_WRITE, FS_STATUS_NOT_EMPTY,
    FS_STATUS_NO_FILE, FS_STATUS_SUCCESS, FS_STATUS_TOO_MANY_OPEN_FILES, LIONS_FS_MAGIC,
    SDDF_BLK_MAGIC,
};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

const BLK_CH: usize = BLK_CLIENT_VIRT_CHANNEL as usize;
const FS_CH: usize = FS_SERVER_CLIENT_CHANNEL as usize;
const STACK_SIZE: usize = 0x4000;

const SECTOR_BYTES: usize = 512;
const TOTAL_SECTORS: u16 = 8192;
const RESERVED_SECTORS: u16 = 1;
const FAT_COUNT: u8 = 2;
const ROOT_ENTRIES: u16 = 512;
const FAT_SECTORS: u16 = 32;
const FAT1_LBA: u32 = 1;
const FAT2_LBA: u32 = 33;
const ROOT_LBA: u32 = 65;
const DATA_LBA: u32 = 97;
const ENTRIES_PER_SECTOR: u16 = 16;
const DIR_ENTRY: usize = 32;
const TRANSFER: usize = 4096;
const SECTORS_PER_TRANSFER: u32 = 8;
const REQ_ID: u32 = 1;
const ATTR_DIR: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
/// Directory descriptor. File descriptors skip this value.
const FD_DIR: u64 = 1;
/// Open files at once. One more open returns `FS_STATUS_TOO_MANY_OPEN_FILES`.
const FILE_CAP: usize = 4;
/// Last cluster that has a data sector. Cluster 2 is `DATA_LBA`.
const LAST_CLUSTER: u16 = TOTAL_SECTORS - 97 + 1;
const _: () = assert!(DATA_LBA == 97);
const EOC: u16 = 0xFFFF;
const EOC_MIN: u16 = 0xFFF8;
/// A file stays within this many bytes. That covers a fetched HTML body
/// (the 16640-byte Transport Layer Security buffer) and an edit buffer.
/// A write may start at a non-zero offset and may cross a 512-byte sector.
const FILE_BYTES: u64 = 32 * 1024;
const PATH_CAP: usize = 128;
const S_IFDIR: u64 = 0o040_000;
const S_IFREG: u64 = 0o100_000;
const MODE_BITS: u64 = 0o777;

#[repr(C, align(16))]
struct Stack([u8; STACK_SIZE]);

struct Stacks(UnsafeCell<[Stack; STACK_SLOTS]>);

// SAFETY: this protection domain has one kernel thread. The cothreads are the only users.
unsafe impl Sync for Stacks {}

static STACKS: Stacks = Stacks(UnsafeCell::new(
    [const { Stack([0; STACK_SIZE]) }; STACK_SLOTS],
));

#[derive(Clone, Copy)]
struct OpenFile {
    cluster: u16,
    size: u32,
    /// Index of the directory entry. `parent` 0 means this index is in the root.
    dir_index: u16,
    /// Cluster of the parent directory. 0 is the root. A subdirectory cluster is at least 2.
    parent: u16,
}

#[derive(Clone, Copy)]
struct OpenDir {
    cluster: u16,
    index: u16,
    limit: u16,
}

struct DirHit {
    index: u16,
    cluster: u16,
    size: u32,
    attr: u8,
}

struct State {
    blk: blk_queue_handle_t,
    commands: *mut fs_queue_t,
    completions: *mut fs_queue_t,
    share: *mut u8,
    storage: *const blk_storage_info_t,
    block_no: u64,
    block_valid: bool,
    block_dirty: bool,
    files: [Option<OpenFile>; FILE_CAP],
    dir: Option<OpenDir>,
}

struct StateCell(UnsafeCell<State>);

// SAFETY: the worker is the only user after init. The root does not touch this cell.
unsafe impl Sync for StateCell {}

static STATE: StateCell = StateCell(UnsafeCell::new(State {
    blk: blk_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    },
    commands: core::ptr::null_mut(),
    completions: core::ptr::null_mut(),
    share: core::ptr::null_mut(),
    storage: core::ptr::null(),
    block_no: 0,
    block_valid: false,
    block_dirty: false,
    files: [None; FILE_CAP],
    dir: None,
}));

struct Block([u8; TRANSFER]);

struct BlockCell(UnsafeCell<Block>);

// SAFETY: same as `STATE`. The worker is the only user.
unsafe impl Sync for BlockCell {}

static BLOCK: BlockCell = BlockCell(UnsafeCell::new(Block([0; TRANSFER])));

enum Scan<T> {
    Hit(T),
    Missing,
    Failed,
}

enum Completion {
    Empty,
    Fd(u64),
    Read(u64),
    Write(u64),
    DirRead(u64),
}

fn state() -> *mut State {
    STATE.0.get()
}

fn block() -> *mut [u8; TRANSFER] {
    // SAFETY: `BLOCK` is a static cell. The pointer is formed without a reference.
    unsafe { core::ptr::addr_of_mut!((*BLOCK.0.get()).0) }
}

extern "C" fn worker() {
    if !prepare() {
        return;
    }
    loop {
        microkit_cothread_wait_on_channel(FS_CH);
        if !serve_one() {
            return;
        }
    }
}

fn prepare() -> bool {
    // SAFETY: the driver publishes `ready` before this protection domain runs.
    let ready = unsafe { blk_storage_is_ready((*state()).storage) };
    if !ready {
        log::info!("fatfs: block not ready");
        return false;
    }
    if !load_block(0) {
        return false;
    }
    if !volume_ok() && !format_volume() {
        log::info!("fatfs: format failed");
        return false;
    }
    // A formatted image keeps its files across reset, so the hostname file is created only when missing.
    if !seed_hostname() {
        log::info!("fatfs: hostname seed failed");
        return false;
    }
    log::info!("fatfs: serving");
    true
}

fn volume_ok() -> bool {
    // SAFETY: `load_block(0)` filled the static block buffer.
    unsafe {
        let bytes = &*block();
        bytes[510] == 0x55
            && bytes[511] == 0xAA
            && u16::from_le_bytes([bytes[11], bytes[12]])
                == u16::try_from(SECTOR_BYTES).expect("sector size")
    }
}

fn format_volume() -> bool {
    if !stage_zero(0) {
        return false;
    }
    unsafe {
        let bytes = &mut *block();
        write_boot(&mut bytes[..SECTOR_BYTES]);
        write_fat_header(&mut bytes[SECTOR_BYTES..SECTOR_BYTES + 4]);
    }
    for number in 1..4 {
        if !stage_zero(number) {
            return false;
        }
    }
    if !stage_zero(4) {
        return false;
    }
    unsafe {
        write_fat_header(&mut (&mut *block())[SECTOR_BYTES..SECTOR_BYTES + 4]);
    }
    for number in 5..=12 {
        if !stage_zero(number) {
            return false;
        }
    }
    flush_block()
}

fn write_boot(sector: &mut [u8]) {
    sector.fill(0);
    sector[0] = 0xEB;
    sector[1] = 0x3C;
    sector[2] = 0x90;
    sector[3..11].copy_from_slice(b"LERUXFAT");
    sector[11..13].copy_from_slice(
        &u16::try_from(SECTOR_BYTES)
            .expect("sector size")
            .to_le_bytes(),
    );
    sector[13] = 1;
    sector[14..16].copy_from_slice(&RESERVED_SECTORS.to_le_bytes());
    sector[16] = FAT_COUNT;
    sector[17..19].copy_from_slice(&ROOT_ENTRIES.to_le_bytes());
    sector[19..21].copy_from_slice(&TOTAL_SECTORS.to_le_bytes());
    sector[21] = 0xF8;
    sector[22..24].copy_from_slice(&FAT_SECTORS.to_le_bytes());
    sector[510] = 0x55;
    sector[511] = 0xAA;
}

fn write_fat_header(bytes: &mut [u8]) {
    bytes[0..2].copy_from_slice(&0xFFF8u16.to_le_bytes());
    bytes[2..4].copy_from_slice(&0xFFFFu16.to_le_bytes());
}

fn serve_one() -> bool {
    let mut msg: fs_msg_t = unsafe { core::mem::zeroed() };
    // SAFETY: this protection domain is the consumer of the command ring.
    if unsafe { fs_message_dequeue((*state()).commands, &mut msg) } != 0 {
        return true;
    }
    let cmd = unsafe { msg.cmd };
    match cmd.r#type {
        FS_CMD_FILE_OPEN => handle_open(cmd.id, unsafe { cmd.params.file_open }),
        FS_CMD_FILE_CLOSE => handle_close(cmd.id, unsafe { cmd.params.file_close }),
        FS_CMD_FILE_WRITE => handle_write(cmd.id, unsafe { cmd.params.file_write }),
        FS_CMD_FILE_READ => handle_read(cmd.id, unsafe { cmd.params.file_read }),
        FS_CMD_STAT => handle_stat(cmd.id, unsafe { cmd.params.stat }),
        FS_CMD_RENAME => handle_rename(cmd.id, unsafe { cmd.params.rename }),
        FS_CMD_FILE_REMOVE => handle_remove(cmd.id, unsafe { cmd.params.file_remove }),
        FS_CMD_DIR_CREATE => handle_mkdir(cmd.id, unsafe { cmd.params.dir_create }),
        FS_CMD_DIR_OPEN => handle_dir_open(cmd.id, unsafe { cmd.params.dir_open }),
        FS_CMD_DIR_READ => handle_dir_read(cmd.id, unsafe { cmd.params.dir_read }),
        FS_CMD_DIR_CLOSE => handle_dir_close(cmd.id, unsafe { cmd.params.dir_close.fd }),
        _ => complete(cmd.id, FS_STATUS_INVALID_COMMAND, &Completion::Empty),
    }
}

fn handle_open(id: u64, params: fs_cmd_params_file_open_t) -> bool {
    let (parent, name) = match entry_path(id, params.path) {
        Ok(pair) => pair,
        Err(done) => return done,
    };
    if params.flags == (FS_OPEN_FLAGS_CREATE | FS_OPEN_FLAGS_WRITE_ONLY) {
        create_file(id, parent, &name)
    } else if params.flags == FS_OPEN_FLAGS_READ_ONLY || params.flags == FS_OPEN_FLAGS_WRITE_ONLY {
        if parent == 0 {
            open_read(id, parent, &name)
        } else {
            open_nested_read(id, parent, &name)
        }
    } else {
        complete(id, FS_STATUS_ERROR, &Completion::Empty)
    }
}

fn open_nested_read(id: u64, parent: u16, name: &[u8; 11]) -> bool {
    if file_busy() {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
    }
    occupy(id, parent, name)
}

fn file_busy() -> bool {
    unsafe { (*state()).files.iter().all(Option::is_some) }
}

fn file_held(parent: u16, index: u16) -> bool {
    unsafe {
        (*state())
            .files
            .iter()
            .flatten()
            .any(|file| file.parent == parent && file.dir_index == index)
    }
}

fn create_file(id: u64, parent: u16, name: &[u8; 11]) -> bool {
    if file_busy() {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
    }
    let (slot, cluster) = match reserve_file(id, parent, name) {
        Ok(pair) => pair,
        Err(done) => return done,
    };
    // Both File Allocation Tables record the new end-of-chain mark.
    if !set_fat(cluster, EOC)
        || !write_entry(parent, slot, name, cluster, 0, ATTR_ARCHIVE)
        || !flush_block()
    {
        let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
        return false;
    }
    let Some(fd) = insert_file(OpenFile {
        cluster,
        size: 0,
        dir_index: slot,
        parent,
    }) else {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
    };
    complete(id, FS_STATUS_SUCCESS, &Completion::Fd(fd))
}

fn reserve_file(id: u64, parent: u16, name: &[u8; 11]) -> Result<(u16, u16), bool> {
    match lookup(parent, name) {
        Scan::Hit(_) => return Err(complete(id, FS_STATUS_ALREADY_EXISTS, &Completion::Empty)),
        Scan::Missing => {}
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return Err(false);
        }
    }
    let slot = match find_free_at(parent) {
        Scan::Hit(index) => index,
        Scan::Missing => {
            return Err(complete(
                id,
                FS_STATUS_DIRECTORY_IS_FULL,
                &Completion::Empty,
            ));
        }
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return Err(false);
        }
    };
    match alloc_cluster() {
        Scan::Hit(cluster) => Ok((slot, cluster)),
        Scan::Missing => Err(complete(
            id,
            FS_STATUS_DIRECTORY_IS_FULL,
            &Completion::Empty,
        )),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            Err(false)
        }
    }
}

fn open_read(id: u64, parent: u16, name: &[u8; 11]) -> bool {
    if file_busy() {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
    }
    occupy(id, parent, name)
}

fn occupy(id: u64, parent: u16, name: &[u8; 11]) -> bool {
    match lookup(parent, name) {
        Scan::Hit(hit) => {
            if hit.attr & ATTR_DIR != 0 || hit.cluster < 2 {
                return complete(id, FS_STATUS_ERROR, &Completion::Empty);
            }
            if file_held(parent, hit.index) {
                return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
            }
            let Some(fd) = insert_file(OpenFile {
                cluster: hit.cluster,
                size: hit.size,
                dir_index: hit.index,
                parent,
            }) else {
                return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
            };
            complete(id, FS_STATUS_SUCCESS, &Completion::Fd(fd))
        }
        Scan::Missing => complete(id, FS_STATUS_NO_FILE, &Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            false
        }
    }
}

fn handle_close(id: u64, params: fs_cmd_params_file_close_t) -> bool {
    let Some(slot) = slot_of(params.fd) else {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    };
    let open = unsafe { (*state()).files[slot].take() };
    if open.is_none() {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    }
    complete(id, FS_STATUS_SUCCESS, &Completion::Empty)
}

fn handle_write(id: u64, params: fs_cmd_params_file_write_t) -> bool {
    let Some(file) = open_file(params.fd) else {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    };
    let Some(len) = write_len(file.size, params.offset, params.buf.size) else {
        return complete(id, FS_STATUS_INVALID_WRITE, &Completion::Empty);
    };
    let Some((src, _)) = share_span(params.buf) else {
        return complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty);
    };
    let end = params.offset + len as u64;
    if !ensure_span(file.cluster, end) || !write_span(file.cluster, params.offset, src, len) {
        let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
        return false;
    }
    let new_size = u32::try_from(end.max(u64::from(file.size))).unwrap_or(file.size);
    if !update_size(file.parent, file.dir_index, new_size) || !flush_block() {
        let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
        return false;
    }
    note_size(params.fd, new_size);
    complete(id, FS_STATUS_SUCCESS, &Completion::Write(len as u64))
}

fn write_len(size: u32, offset: u64, len: u64) -> Option<usize> {
    if len == 0 || len > FILE_BYTES {
        return None;
    }
    let len = usize::try_from(len).ok()?;
    let end = offset.checked_add(len as u64)?;
    if offset > u64::from(size) || end > FILE_BYTES {
        return None;
    }
    Some(len)
}

fn handle_read(id: u64, params: fs_cmd_params_file_read_t) -> bool {
    let Some(file) = open_file(params.fd) else {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    };
    let Some((buf_off, buf_len)) = share_span(params.buf) else {
        return complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty);
    };
    let file_size = u64::from(file.size);
    if params.offset > file_size {
        return complete(id, FS_STATUS_INVALID_READ, &Completion::Empty);
    }
    let available = file_size - params.offset;
    let copied = buf_len.min(usize::try_from(available).unwrap_or(0));
    if copied > 0
        && let Err(done) = stage_file_bytes(id, &file, params.offset, buf_off, copied)
    {
        return done;
    }
    let Ok(len_read) = u64::try_from(copied) else {
        return complete(id, FS_STATUS_ERROR, &Completion::Empty);
    };
    complete(id, FS_STATUS_SUCCESS, &Completion::Read(len_read))
}

fn share_span(buf: fs_buffer_t) -> Option<(usize, usize)> {
    let buf_len = usize::try_from(buf.size).ok()?;
    let buf_off = usize::try_from(buf.offset).ok()?;
    let share_len = usize::try_from(FS_REGION_SIZE).ok()?;
    let end = buf_off.checked_add(buf_len)?;
    if end > share_len {
        return None;
    }
    Some((buf_off, buf_len))
}

fn stage_file_bytes(
    id: u64,
    file: &OpenFile,
    file_off: u64,
    buf_off: usize,
    copied: usize,
) -> Result<(), bool> {
    let mut done = 0;
    while done < copied {
        let pos = file_off + done as u64;
        let cluster = match cluster_at(file.cluster, pos) {
            Scan::Hit(cluster) => cluster,
            Scan::Missing | Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
                return Err(false);
            }
        };
        let lba = DATA_LBA + u32::from(cluster - 2);
        if !load_block(u64::from(lba / SECTORS_PER_TRANSFER)) {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return Err(false);
        }
        let within = sector_within(pos);
        let chunk = (SECTOR_BYTES - within).min(copied - done);
        let sector = ((lba % SECTORS_PER_TRANSFER) as usize) * SECTOR_BYTES + within;
        // SAFETY: the share range was checked, and the sector slice sits in the static block.
        unsafe {
            core::ptr::copy_nonoverlapping(
                (*block()).as_ptr().add(sector),
                (*state()).share.add(buf_off + done),
                chunk,
            );
        }
        done += chunk;
    }
    Ok(())
}

fn find_name(name: &[u8; 11]) -> Scan<DirHit> {
    for index in 0..ROOT_ENTRIES {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_entry(index, &mut entry) {
            return Scan::Failed;
        }
        if entry[0] == 0x00 {
            return Scan::Missing;
        }
        if entry[0] == 0xE5 {
            continue;
        }
        if entry[..11] == *name {
            return Scan::Hit(hit_from(index, &entry));
        }
    }
    Scan::Missing
}

fn hit_from(index: u16, entry: &[u8; DIR_ENTRY]) -> DirHit {
    DirHit {
        index,
        cluster: u16::from_le_bytes([entry[26], entry[27]]),
        size: u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]),
        attr: entry[11],
    }
}

fn find_free() -> Scan<u16> {
    for index in 0..ROOT_ENTRIES {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_entry(index, &mut entry) {
            return Scan::Failed;
        }
        if entry[0] == 0x00 || entry[0] == 0xE5 {
            return Scan::Hit(index);
        }
    }
    Scan::Missing
}

fn read_entry(index: u16, dest: &mut [u8; DIR_ENTRY]) -> bool {
    let lba = ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR);
    if !load_block(u64::from(lba / SECTORS_PER_TRANSFER)) {
        return false;
    }
    let offset = ((lba % SECTORS_PER_TRANSFER) as usize) * SECTOR_BYTES
        + usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY;
    // SAFETY: the directory entry is inside the loaded transfer block.
    unsafe {
        dest.copy_from_slice(&(&*block())[offset..offset + DIR_ENTRY]);
    }
    true
}

fn write_dir(index: u16, name: &[u8; 11], cluster: u16, size: u32, attr: u8) -> bool {
    let lba = ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR);
    let off = usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY;
    edit_sector(lba, |sector| {
        put_entry(&mut sector[off..off + DIR_ENTRY], name, cluster, size, attr);
    })
}

fn lookup(parent: u16, name: &[u8; 11]) -> Scan<DirHit> {
    if parent == 0 {
        find_name(name)
    } else {
        find_in_cluster(parent, name)
    }
}

fn find_free_at(parent: u16) -> Scan<u16> {
    if parent == 0 {
        find_free()
    } else {
        find_free_in_cluster(parent)
    }
}

fn write_entry(
    parent: u16,
    index: u16,
    name: &[u8; 11],
    cluster: u16,
    size: u32,
    attr: u8,
) -> bool {
    if parent == 0 {
        write_dir(index, name, cluster, size, attr)
    } else {
        write_in_cluster(parent, index, name, cluster, size, attr)
    }
}

fn put_entry(entry: &mut [u8], name: &[u8; 11], cluster: u16, size: u32, attr: u8) {
    entry.fill(0);
    entry[..11].copy_from_slice(name);
    entry[11] = attr;
    entry[26..28].copy_from_slice(&cluster.to_le_bytes());
    entry[28..32].copy_from_slice(&size.to_le_bytes());
}

fn update_size(parent: u16, index: u16, size: u32) -> bool {
    let (lba, off) = if parent == 0 {
        (
            ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR),
            usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY,
        )
    } else if parent < 2 || index >= ENTRIES_PER_SECTOR {
        return false;
    } else {
        (
            DATA_LBA + u32::from(parent - 2),
            usize::from(index) * DIR_ENTRY,
        )
    };
    edit_sector(lba, |sector| {
        sector[off + 28..off + 32].copy_from_slice(&size.to_le_bytes());
    })
}

fn seed_hostname() -> bool {
    let Some(dir_name) = pack_name(b"config") else {
        return false;
    };
    let Some(file_name) = pack_name(b"hostname") else {
        return false;
    };
    let dir_cluster = match find_name(&dir_name) {
        Scan::Hit(hit) => {
            if hit.attr & ATTR_DIR == 0 || hit.cluster < 2 {
                return false;
            }
            hit.cluster
        }
        Scan::Missing => match create_directory(&dir_name) {
            Some(cluster) => cluster,
            None => return false,
        },
        Scan::Failed => return false,
    };
    match find_in_cluster(dir_cluster, &file_name) {
        Scan::Hit(_) => true,
        Scan::Missing => write_seed_file(dir_cluster, &file_name),
        Scan::Failed => false,
    }
}

fn create_directory(name: &[u8; 11]) -> Option<u16> {
    let slot = match find_free() {
        Scan::Hit(index) => index,
        Scan::Missing | Scan::Failed => return None,
    };
    let cluster = match alloc_cluster() {
        Scan::Hit(cluster) => cluster,
        Scan::Missing | Scan::Failed => return None,
    };
    if !set_fat(cluster, 0xFFFF)
        || !format_directory(cluster, 0)
        || !write_dir(slot, name, cluster, 0, ATTR_DIR)
        || !flush_block()
    {
        return None;
    }
    Some(cluster)
}

fn write_seed_file(parent: u16, name: &[u8; 11]) -> bool {
    let slot = match find_free_in_cluster(parent) {
        Scan::Hit(index) => index,
        Scan::Missing | Scan::Failed => return false,
    };
    let cluster = match alloc_cluster() {
        Scan::Hit(cluster) => cluster,
        Scan::Missing | Scan::Failed => return false,
    };
    let bytes = b"lerux";
    let lba = DATA_LBA + u32::from(cluster - 2);
    set_fat(cluster, 0xFFFF)
        && edit_sector(lba, |sector| {
            sector.fill(0);
            sector[..bytes.len()].copy_from_slice(bytes);
        })
        && write_in_cluster(
            parent,
            slot,
            name,
            cluster,
            u32::try_from(bytes.len()).expect("hostname length"),
            ATTR_ARCHIVE,
        )
        && flush_block()
}

fn find_in_cluster(cluster: u16, name: &[u8; 11]) -> Scan<DirHit> {
    if cluster < 2 {
        return Scan::Failed;
    }
    for index in 0..ENTRIES_PER_SECTOR {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_placed(cluster, index, &mut entry) {
            return Scan::Failed;
        }
        if entry[0] == 0x00 {
            return Scan::Missing;
        }
        if entry[0] == 0xE5 {
            continue;
        }
        if entry[..11] == *name {
            return Scan::Hit(hit_from(index, &entry));
        }
    }
    Scan::Missing
}

fn find_free_in_cluster(cluster: u16) -> Scan<u16> {
    if cluster < 2 {
        return Scan::Failed;
    }
    for index in 0..ENTRIES_PER_SECTOR {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_placed(cluster, index, &mut entry) {
            return Scan::Failed;
        }
        if entry[0] == 0x00 || entry[0] == 0xE5 {
            return Scan::Hit(index);
        }
    }
    Scan::Missing
}

fn write_in_cluster(
    parent: u16,
    index: u16,
    name: &[u8; 11],
    cluster: u16,
    size: u32,
    attr: u8,
) -> bool {
    if parent < 2 || index >= ENTRIES_PER_SECTOR {
        return false;
    }
    let lba = DATA_LBA + u32::from(parent - 2);
    let off = usize::from(index) * DIR_ENTRY;
    edit_sector(lba, |sector| {
        put_entry(&mut sector[off..off + DIR_ENTRY], name, cluster, size, attr);
    })
}

fn set_fat(cluster: u16, value: u16) -> bool {
    let byte = usize::from(cluster) * 2;
    let sector_index = u32::try_from(byte / SECTOR_BYTES).unwrap_or(0);
    let within = byte % SECTOR_BYTES;
    for base in [FAT1_LBA, FAT2_LBA] {
        if !edit_sector(base + sector_index, |sector| {
            sector[within..within + 2].copy_from_slice(&value.to_le_bytes());
        }) {
            return false;
        }
    }
    true
}

fn read_fat(cluster: u16) -> Scan<u16> {
    let byte = usize::from(cluster) * 2;
    let sector_index = u32::try_from(byte / SECTOR_BYTES).unwrap_or(0);
    let within = byte % SECTOR_BYTES;
    let lba = FAT1_LBA + sector_index;
    if !load_block(u64::from(lba / SECTORS_PER_TRANSFER)) {
        return Scan::Failed;
    }
    let offset = ((lba % SECTORS_PER_TRANSFER) as usize) * SECTOR_BYTES + within;
    // SAFETY: the File Allocation Table entry sits inside the loaded transfer block.
    let bytes = unsafe { &*block() };
    Scan::Hit(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn alloc_cluster() -> Scan<u16> {
    for cluster in 2..=LAST_CLUSTER {
        match read_fat(cluster) {
            Scan::Hit(0) => return Scan::Hit(cluster),
            Scan::Hit(_) => {}
            Scan::Missing => return Scan::Missing,
            Scan::Failed => return Scan::Failed,
        }
    }
    Scan::Missing
}

fn linked(value: u16) -> bool {
    (2..EOC_MIN).contains(&value)
}

fn sector_within(pos: u64) -> usize {
    let sector = u64::try_from(SECTOR_BYTES).expect("sector size");
    usize::try_from(pos % sector).expect("offset in sector")
}

fn stat_of(hit: Option<DirHit>) -> fs_stat_t {
    // SAFETY: `fs_stat_t` is plain data. Zero is a valid stat.
    let mut stat: fs_stat_t = unsafe { core::mem::zeroed() };
    stat.blksize = SECTOR_BYTES as u64;
    match hit {
        None => {
            stat.mode = S_IFDIR | MODE_BITS;
            stat.ino = 1;
        }
        Some(hit) => {
            stat.size = u64::from(hit.size);
            stat.ino = u64::from(hit.index) + 2;
            stat.mode = if hit.attr & ATTR_DIR != 0 {
                S_IFDIR | MODE_BITS
            } else {
                S_IFREG | MODE_BITS
            };
        }
    }
    stat
}

fn cluster_at(first: u16, offset: u64) -> Scan<u16> {
    if first < 2 {
        return Scan::Failed;
    }
    let mut skip = offset / SECTOR_BYTES as u64;
    let mut cluster = first;
    while skip > 0 {
        match read_fat(cluster) {
            Scan::Hit(next) if linked(next) => {
                cluster = next;
                skip -= 1;
            }
            Scan::Hit(_) | Scan::Missing => return Scan::Missing,
            Scan::Failed => return Scan::Failed,
        }
    }
    Scan::Hit(cluster)
}

fn ensure_span(first: u16, bytes: u64) -> bool {
    let need = if bytes == 0 {
        0
    } else {
        bytes.div_ceil(SECTOR_BYTES as u64)
    };
    if need == 0 {
        return true;
    }
    let mut cluster = first;
    let mut have = 1u64;
    while have < need {
        match read_fat(cluster) {
            Scan::Hit(next) if linked(next) => {
                cluster = next;
                have += 1;
            }
            Scan::Hit(_) => {
                let next = match alloc_cluster() {
                    Scan::Hit(cluster) => cluster,
                    Scan::Missing | Scan::Failed => return false,
                };
                // Mark the new cluster in both tables before the previous one points at it.
                if !set_fat(next, EOC) || !set_fat(cluster, next) {
                    return false;
                }
                cluster = next;
                have += 1;
            }
            Scan::Missing | Scan::Failed => return false,
        }
    }
    true
}

fn write_span(first: u16, offset: u64, src: usize, len: usize) -> bool {
    let mut done = 0;
    while done < len {
        let pos = offset + done as u64;
        let cluster = match cluster_at(first, pos) {
            Scan::Hit(cluster) => cluster,
            Scan::Missing | Scan::Failed => return false,
        };
        let within = sector_within(pos);
        let chunk = (SECTOR_BYTES - within).min(len - done);
        let mut tmp = [0u8; SECTOR_BYTES];
        let Ok(step) = u64::try_from(done) else {
            return false;
        };
        let Ok(src) = u64::try_from(src) else {
            return false;
        };
        let Some(base) = src.checked_add(step) else {
            return false;
        };
        if !copy_share(base, chunk, &mut tmp[..chunk]) {
            return false;
        }
        let lba = DATA_LBA + u32::from(cluster - 2);
        if !edit_sector(lba, |sector| {
            sector[within..within + chunk].copy_from_slice(&tmp[..chunk]);
        }) {
            return false;
        }
        done += chunk;
    }
    true
}

fn free_chain(start: u16) -> bool {
    if start < 2 {
        return true;
    }
    let mut cluster = start;
    for _ in 0..LAST_CLUSTER {
        let next = match read_fat(cluster) {
            Scan::Hit(value) => value,
            Scan::Missing | Scan::Failed => return false,
        };
        if !set_fat(cluster, 0) {
            return false;
        }
        if !linked(next) {
            return true;
        }
        cluster = next;
    }
    false
}

fn handle_stat(id: u64, params: fs_cmd_params_stat_t) -> bool {
    if params.buf.size < core::mem::size_of::<fs_stat_t>() as u64 {
        return complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty);
    }
    let found = match read_walk(id, params.path) {
        Ok(found) => found,
        Err(done) => return done,
    };
    let hit = match found {
        Path::Root => None,
        Path::Entry { parent, name } => match lookup(parent, &name) {
            Scan::Hit(hit) => Some(hit),
            Scan::Missing => return complete(id, FS_STATUS_NO_FILE, &Completion::Empty),
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
                return false;
            }
        },
        Path::Missing => return complete(id, FS_STATUS_NO_FILE, &Completion::Empty),
        Path::Bad | Path::NotDir => {
            return complete(id, FS_STATUS_INVALID_NAME, &Completion::Empty)
        }
        Path::Io => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    };
    let stat = stat_of(hit);
    if !write_share(params.buf, &stat) {
        return complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty);
    }
    complete(id, FS_STATUS_SUCCESS, &Completion::Empty)
}

fn handle_rename(id: u64, params: fs_cmd_params_rename_t) -> bool {
    let (old_parent, old_name) = match entry_path(id, params.old_path) {
        Ok(pair) => pair,
        Err(done) => return done,
    };
    let (new_parent, new_name) = match entry_path(id, params.new_path) {
        Ok(pair) => pair,
        Err(done) => return done,
    };
    if old_parent != new_parent {
        return complete(id, FS_STATUS_ERROR, &Completion::Empty);
    }
    let hit = match lookup(old_parent, &old_name) {
        Scan::Hit(hit) => hit,
        Scan::Missing => return complete(id, FS_STATUS_NO_FILE, &Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    };
    if old_name != new_name {
        match lookup(new_parent, &new_name) {
            Scan::Hit(_) => return complete(id, FS_STATUS_ALREADY_EXISTS, &Completion::Empty),
            Scan::Missing => {}
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
                return false;
            }
        }
    }
    if !rename_entry(old_parent, hit.index, &new_name) || !flush_block() {
        let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
        return false;
    }
    complete(id, FS_STATUS_SUCCESS, &Completion::Empty)
}

fn handle_remove(id: u64, params: fs_cmd_params_file_remove_t) -> bool {
    let (parent, name) = match entry_path(id, params.path) {
        Ok(pair) => pair,
        Err(done) => return done,
    };
    let hit = match lookup(parent, &name) {
        Scan::Hit(hit) => hit,
        Scan::Missing => return complete(id, FS_STATUS_NO_FILE, &Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    };
    if hit.attr & ATTR_DIR != 0 {
        match directory_empty(hit.cluster) {
            Scan::Hit(true) => {}
            Scan::Hit(false) => return complete(id, FS_STATUS_NOT_EMPTY, &Completion::Empty),
            Scan::Missing => return complete(id, FS_STATUS_ERROR, &Completion::Empty),
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
                return false;
            }
        }
    }
    if !free_chain(hit.cluster) || !mark_deleted(parent, hit.index) || !flush_block() {
        let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
        return false;
    }
    drop_open(parent, hit.index);
    complete(id, FS_STATUS_SUCCESS, &Completion::Empty)
}

fn handle_mkdir(id: u64, params: fs_cmd_params_dir_create_t) -> bool {
    let (parent, name) = match entry_path(id, params.path) {
        Ok(pair) => pair,
        Err(done) => return done,
    };
    match lookup(parent, &name) {
        Scan::Hit(_) => return complete(id, FS_STATUS_ALREADY_EXISTS, &Completion::Empty),
        Scan::Missing => {}
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    }
    let slot = match find_free_at(parent) {
        Scan::Hit(index) => index,
        Scan::Missing => return complete(id, FS_STATUS_DIRECTORY_IS_FULL, &Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    };
    let cluster = match alloc_cluster() {
        Scan::Hit(cluster) => cluster,
        Scan::Missing => return complete(id, FS_STATUS_DIRECTORY_IS_FULL, &Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    };
    if !set_fat(cluster, EOC)
        || !format_directory(cluster, parent)
        || !write_entry(parent, slot, &name, cluster, 0, ATTR_DIR)
        || !flush_block()
    {
        let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
        return false;
    }
    complete(id, FS_STATUS_SUCCESS, &Completion::Empty)
}

fn handle_dir_open(id: u64, params: fs_cmd_params_dir_open_t) -> bool {
    if unsafe { (*state()).dir.is_some() } {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, &Completion::Empty);
    }
    let found = match read_walk(id, params.path) {
        Ok(found) => found,
        Err(done) => return done,
    };
    let opened = match found {
        Path::Root => OpenDir {
            cluster: 0,
            index: 0,
            limit: ROOT_ENTRIES,
        },
        Path::Entry { parent, name } => match lookup(parent, &name) {
            Scan::Hit(hit) if hit.attr & ATTR_DIR != 0 && hit.cluster >= 2 => OpenDir {
                cluster: hit.cluster,
                index: 0,
                limit: ENTRIES_PER_SECTOR,
            },
            Scan::Hit(_) | Scan::Missing => {
                return complete(id, FS_STATUS_NO_FILE, &Completion::Empty);
            }
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
                return false;
            }
        },
        Path::Bad => return complete(id, FS_STATUS_INVALID_NAME, &Completion::Empty),
        Path::Missing | Path::NotDir => return complete(id, FS_STATUS_NO_FILE, &Completion::Empty),
        Path::Io => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
    };
    unsafe {
        (*state()).dir = Some(opened);
    }
    complete(id, FS_STATUS_SUCCESS, &Completion::Fd(FD_DIR))
}

fn handle_dir_read(id: u64, params: fs_cmd_params_dir_read_t) -> bool {
    let Some(dir) = (unsafe { (*state()).dir }) else {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    };
    if params.fd != FD_DIR {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    }
    let mut index = dir.index;
    while index < dir.limit {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_placed(dir.cluster, index, &mut entry) {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            return false;
        }
        index += 1;
        if entry[0] == 0x00 {
            unsafe {
                if let Some(open) = (*state()).dir.as_mut() {
                    open.index = index;
                }
            }
            return complete(id, FS_STATUS_END_OF_DIRECTORY, &Completion::DirRead(0));
        }
        if entry[0] == 0xE5 || is_dot(&entry) {
            continue;
        }
        let (shown, len) = display_name(&entry);
        if (len as u64) > params.buf.size {
            return complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty);
        }
        if !copy_to_share(params.buf.offset, &shown[..len]) {
            return complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty);
        }
        unsafe {
            if let Some(open) = (*state()).dir.as_mut() {
                open.index = index;
            }
        }
        return complete(id, FS_STATUS_SUCCESS, &Completion::DirRead(len as u64));
    }
    complete(id, FS_STATUS_END_OF_DIRECTORY, &Completion::DirRead(0))
}

fn handle_dir_close(id: u64, fd: u64) -> bool {
    if fd != FD_DIR || unsafe { (*state()).dir.is_none() } {
        return complete(id, FS_STATUS_INVALID_FD, &Completion::Empty);
    }
    unsafe {
        (*state()).dir = None;
    }
    complete(id, FS_STATUS_SUCCESS, &Completion::Empty)
}

enum SharePath<'a> {
    Ok(&'a [u8]),
    TooLong,
    BadShare,
}

enum Path {
    Root,
    Entry { parent: u16, name: [u8; 11] },
    Bad,
    Missing,
    NotDir,
    Io,
}

fn share_path(buf: fs_buffer_t, raw: &mut [u8; PATH_CAP]) -> SharePath<'_> {
    let Ok(size) = usize::try_from(buf.size) else {
        return SharePath::TooLong;
    };
    if size > PATH_CAP {
        return SharePath::TooLong;
    }
    if size > 0 && !copy_share(buf.offset, size, &mut raw[..size]) {
        return SharePath::BadShare;
    }
    SharePath::Ok(&raw[..size])
}

fn read_walk(id: u64, buf: fs_buffer_t) -> Result<Path, bool> {
    let mut raw = [0u8; PATH_CAP];
    let path = match share_path(buf, &mut raw) {
        SharePath::Ok(path) => path,
        SharePath::TooLong => {
            return Err(complete(id, FS_STATUS_INVALID_NAME, &Completion::Empty));
        }
        SharePath::BadShare => {
            return Err(complete(id, FS_STATUS_INVALID_BUFFER, &Completion::Empty));
        }
    };
    Ok(walk(path))
}

fn entry_path(id: u64, buf: fs_buffer_t) -> Result<(u16, [u8; 11]), bool> {
    match read_walk(id, buf)? {
        Path::Entry { parent, name } => Ok((parent, name)),
        Path::Missing => Err(complete(id, FS_STATUS_NO_FILE, &Completion::Empty)),
        Path::NotDir => Err(complete(id, FS_STATUS_ERROR, &Completion::Empty)),
        Path::Io => {
            let _ = complete(id, FS_STATUS_ERROR, &Completion::Empty);
            Err(false)
        }
        Path::Root | Path::Bad => Err(complete(id, FS_STATUS_INVALID_NAME, &Completion::Empty)),
    }
}

fn walk(path: &[u8]) -> Path {
    if path.is_empty() || path == b"." || path == b"/" {
        return Path::Root;
    }
    let mut rest = path.strip_prefix(b"/").unwrap_or(path);
    if rest.is_empty() {
        return Path::Root;
    }
    if rest.ends_with(b"/") {
        return Path::Bad;
    }
    let mut parent = 0u16;
    loop {
        let Some((comp, next)) = split_comp(rest) else {
            return Path::Bad;
        };
        let Some(name) = pack_name(comp) else {
            return Path::Bad;
        };
        if next.is_empty() {
            return Path::Entry { parent, name };
        }
        match lookup(parent, &name) {
            Scan::Hit(hit) if hit.attr & ATTR_DIR != 0 && hit.cluster >= 2 => {
                parent = hit.cluster;
            }
            Scan::Hit(_) => return Path::NotDir,
            Scan::Missing => return Path::Missing,
            Scan::Failed => return Path::Io,
        }
        rest = next;
    }
}

fn split_comp(path: &[u8]) -> Option<(&[u8], &[u8])> {
    if path.is_empty() {
        return None;
    }
    let Some(slash) = path.iter().position(|byte| *byte == b'/') else {
        return Some((path, b""));
    };
    let comp = &path[..slash];
    if comp.is_empty() {
        return None;
    }
    Some((comp, &path[slash + 1..]))
}

fn read_placed(cluster: u16, index: u16, dest: &mut [u8; DIR_ENTRY]) -> bool {
    if cluster == 0 {
        return read_entry(index, dest);
    }
    let lba = DATA_LBA + u32::from(cluster - 2);
    if !load_block(u64::from(lba / SECTORS_PER_TRANSFER)) {
        return false;
    }
    let offset =
        ((lba % SECTORS_PER_TRANSFER) as usize) * SECTOR_BYTES + usize::from(index) * DIR_ENTRY;
    // SAFETY: one directory cluster is one sector, and the entry sits inside it.
    unsafe {
        dest.copy_from_slice(&(&*block())[offset..offset + DIR_ENTRY]);
    }
    true
}

fn directory_empty(cluster: u16) -> Scan<bool> {
    if cluster < 2 {
        return Scan::Missing;
    }
    for index in 0..ENTRIES_PER_SECTOR {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_placed(cluster, index, &mut entry) {
            return Scan::Failed;
        }
        if entry[0] == 0x00 {
            return Scan::Hit(true);
        }
        if entry[0] == 0xE5 || is_dot(&entry) {
            continue;
        }
        return Scan::Hit(false);
    }
    Scan::Hit(true)
}

fn format_directory(cluster: u16, parent: u16) -> bool {
    let lba = DATA_LBA + u32::from(cluster - 2);
    let dot = dot_name(1);
    let dotdot = dot_name(2);
    edit_sector(lba, |sector| {
        sector.fill(0);
        put_entry(&mut sector[..DIR_ENTRY], &dot, cluster, 0, ATTR_DIR);
        put_entry(
            &mut sector[DIR_ENTRY..DIR_ENTRY * 2],
            &dotdot,
            parent,
            0,
            ATTR_DIR,
        );
    })
}

fn rename_entry(parent: u16, index: u16, name: &[u8; 11]) -> bool {
    let Some((lba, off)) = entry_at(parent, index) else {
        return false;
    };
    edit_sector(lba, |sector| {
        sector[off..off + 11].copy_from_slice(name);
    })
}

fn mark_deleted(parent: u16, index: u16) -> bool {
    let Some((lba, off)) = entry_at(parent, index) else {
        return false;
    };
    edit_sector(lba, |sector| {
        sector[off] = 0xE5;
    })
}

fn entry_at(parent: u16, index: u16) -> Option<(u32, usize)> {
    if parent == 0 {
        if index >= ROOT_ENTRIES {
            return None;
        }
        let lba = ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR);
        let off = usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY;
        return Some((lba, off));
    }
    if parent < 2 || index >= ENTRIES_PER_SECTOR {
        return None;
    }
    let lba = DATA_LBA + u32::from(parent - 2);
    Some((lba, usize::from(index) * DIR_ENTRY))
}

fn fd_of(slot: usize) -> u64 {
    let fd = slot as u64;
    if fd >= FD_DIR {
        fd + 1
    } else {
        fd
    }
}

fn slot_of(fd: u64) -> Option<usize> {
    if fd == FD_DIR {
        return None;
    }
    let raw = if fd > FD_DIR { fd - 1 } else { fd };
    let slot = usize::try_from(raw).ok()?;
    (slot < FILE_CAP).then_some(slot)
}

fn insert_file(file: OpenFile) -> Option<u64> {
    unsafe {
        let files = &mut (*state()).files;
        let slot = files.iter().position(Option::is_none)?;
        files[slot] = Some(file);
        Some(fd_of(slot))
    }
}

fn open_file(fd: u64) -> Option<OpenFile> {
    let slot = slot_of(fd)?;
    unsafe { (*state()).files[slot] }
}

fn note_size(fd: u64, size: u32) {
    let Some(slot) = slot_of(fd) else {
        return;
    };
    unsafe {
        if let Some(file) = (*state()).files[slot].as_mut() {
            file.size = size;
        }
    }
}

fn drop_open(parent: u16, index: u16) {
    unsafe {
        for entry in &mut (*state()).files {
            if entry
                .as_ref()
                .is_some_and(|file| file.parent == parent && file.dir_index == index)
            {
                *entry = None;
            }
        }
    }
}

fn is_dot(entry: &[u8; DIR_ENTRY]) -> bool {
    entry[0] == b'.' && (entry[1] == b' ' || entry[1] == b'.')
}

fn dot_name(dots: usize) -> [u8; 11] {
    let mut name = [b' '; 11];
    for byte in name.iter_mut().take(dots) {
        *byte = b'.';
    }
    name
}

fn display_name(entry: &[u8; DIR_ENTRY]) -> ([u8; 12], usize) {
    let mut shown = [0u8; 12];
    let mut len = 0;
    for &byte in &entry[..8] {
        if byte == b' ' {
            break;
        }
        shown[len] = byte;
        len += 1;
    }
    if entry[8] != b' ' {
        shown[len] = b'.';
        len += 1;
        for &byte in &entry[8..11] {
            if byte == b' ' {
                break;
            }
            shown[len] = byte;
            len += 1;
        }
    }
    (shown, len)
}

fn write_share<T: Copy>(buf: fs_buffer_t, value: &T) -> bool {
    let size = core::mem::size_of::<T>();
    if buf.size < size as u64 {
        return false;
    }
    let Ok(offset) = usize::try_from(buf.offset) else {
        return false;
    };
    let Ok(share_len) = usize::try_from(FS_REGION_SIZE) else {
        return false;
    };
    if offset.checked_add(size).is_none_or(|end| end > share_len) {
        return false;
    }
    // SAFETY: `offset` is inside the mapped share and holds one `T`.
    unsafe {
        core::ptr::write_unaligned((*state()).share.add(offset).cast::<T>(), *value);
    }
    true
}

fn copy_to_share(offset: u64, src: &[u8]) -> bool {
    let Ok(offset) = usize::try_from(offset) else {
        return false;
    };
    let Ok(share_len) = usize::try_from(FS_REGION_SIZE) else {
        return false;
    };
    if offset
        .checked_add(src.len())
        .is_none_or(|end| end > share_len)
    {
        return false;
    }
    // SAFETY: the destination range is inside the mapped share.
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), (*state()).share.add(offset), src.len());
    }
    true
}

fn copy_share(offset: u64, size: usize, dest: &mut [u8]) -> bool {
    if dest.len() != size {
        return false;
    }
    let Ok(offset) = usize::try_from(offset) else {
        return false;
    };
    let Ok(share_len) = usize::try_from(FS_REGION_SIZE) else {
        return false;
    };
    let Some(end) = offset.checked_add(size) else {
        return false;
    };
    if end > share_len {
        return false;
    }
    // SAFETY: `offset..end` is inside the mapped share.
    unsafe {
        core::ptr::copy_nonoverlapping((*state()).share.add(offset), dest.as_mut_ptr(), size);
    }
    true
}

fn pack_name(raw: &[u8]) -> Option<[u8; 11]> {
    if raw.is_empty() || raw.len() > 11 {
        return None;
    }
    let mut name = [b' '; 11];
    let Some(dot) = raw.iter().position(|byte| *byte == b'.') else {
        if raw.len() > 8 {
            return None;
        }
        for (index, byte) in raw.iter().enumerate() {
            name[index] = upper(*byte);
        }
        return Some(name);
    };
    let base = &raw[..dot];
    let ext = &raw[dot + 1..];
    if base.is_empty() || base.len() > 8 || ext.len() > 3 {
        return None;
    }
    for (index, byte) in base.iter().enumerate() {
        name[index] = upper(*byte);
    }
    for (index, byte) in ext.iter().enumerate() {
        name[8 + index] = upper(*byte);
    }
    Some(name)
}

fn upper(byte: u8) -> u8 {
    if byte.is_ascii_lowercase() {
        byte.to_ascii_uppercase()
    } else {
        byte
    }
}

fn complete(id: u64, status: u64, data: &Completion) -> bool {
    // End of directory is a normal directory read. Logging it writes the debug
    // UART while the serial driver owns the device.
    if status != FS_STATUS_SUCCESS && status != FS_STATUS_END_OF_DIRECTORY {
        log::info!("fatfs: status {status}");
    }
    let mut cmpl: fs_cmpl_t = unsafe { core::mem::zeroed() };
    cmpl.id = id;
    cmpl.status = status;
    match data {
        Completion::Empty => {}
        Completion::Fd(fd) => {
            cmpl.data.file_open.fd = *fd;
            cmpl.data.dir_open.fd = *fd;
        }
        Completion::Read(len) => cmpl.data.file_read.len_read = *len,
        Completion::Write(len) => cmpl.data.file_write.len_written = *len,
        Completion::DirRead(len) => cmpl.data.dir_read.path_len = *len,
    }
    // SAFETY: this protection domain is the producer of the completion ring.
    if unsafe { fs_completion_enqueue((*state()).completions, cmpl) } != 0 {
        log::info!("fatfs: completion queue full");
        return false;
    }
    Channel::new(FS_CH).notify();
    true
}

fn load_block(number: u64) -> bool {
    let (valid, current) = unsafe {
        let state = &*state();
        (state.block_valid, state.block_no)
    };
    if valid && current == number {
        return true;
    }
    if !flush_block() || !blk_transfer(blk_req_code_t::BLK_REQ_READ, number, false) {
        return false;
    }
    unsafe {
        let state = &mut *state();
        state.block_no = number;
        state.block_valid = true;
        state.block_dirty = false;
    }
    true
}

fn flush_block() -> bool {
    let (dirty, number) = unsafe {
        let state = &*state();
        (state.block_dirty, state.block_no)
    };
    if !dirty {
        return true;
    }
    if !blk_transfer(blk_req_code_t::BLK_REQ_WRITE, number, true) {
        return false;
    }
    unsafe {
        (*state()).block_dirty = false;
    }
    true
}

fn stage_zero(number: u64) -> bool {
    if !flush_block() {
        return false;
    }
    unsafe {
        (*block()).fill(0);
        let state = &mut *state();
        state.block_no = number;
        state.block_valid = true;
        state.block_dirty = true;
    }
    true
}

fn edit_sector(lba: u32, edit: impl FnOnce(&mut [u8])) -> bool {
    if !load_block(u64::from(lba / SECTORS_PER_TRANSFER)) {
        return false;
    }
    let offset = ((lba % SECTORS_PER_TRANSFER) as usize) * SECTOR_BYTES;
    unsafe {
        edit(&mut (&mut *block())[offset..offset + SECTOR_BYTES]);
        (*state()).block_dirty = true;
    }
    true
}

fn blk_transfer(code: blk_req_code_t, block_no: u64, write: bool) -> bool {
    let data = BLK_DATA_VADDR as *mut u8;
    // SAFETY: the template maps the data region at `BLK_DATA_VADDR` for one transfer block.
    unsafe {
        if write {
            core::ptr::copy_nonoverlapping(block().cast::<u8>(), data, TRANSFER);
        }
        if blk_enqueue_req(&(*state()).blk, code, 0, block_no, 1, REQ_ID) != 0 {
            log::info!("fatfs: block transfer failed");
            return false;
        }
    }
    Channel::new(BLK_CH).notify();
    loop {
        microkit_cothread_wait_on_channel(BLK_CH);
        let mut status = blk_resp_status_t::BLK_RESP_OK;
        let mut success = 0u16;
        let mut id = 0u32;
        // SAFETY: this protection domain is the consumer of the client response queue.
        let dequeued =
            unsafe { blk_dequeue_resp(&(*state()).blk, &mut status, &mut success, &mut id) };
        if dequeued != 0 {
            continue;
        }
        if status != blk_resp_status_t::BLK_RESP_OK || success != 1 || id != REQ_ID {
            log::info!("fatfs: block transfer failed");
            return false;
        }
        break;
    }
    if !write {
        // SAFETY: the driver wrote one transfer block at the start of the data region.
        unsafe {
            core::ptr::copy_nonoverlapping(data, block().cast::<u8>(), TRANSFER);
        }
    }
    true
}

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    let blk_len = core::mem::size_of::<blk_client_config_t>();
    let fs_len = core::mem::size_of::<fs_server_config_t>();
    assert_eq!(CONFIG.len(), blk_len + fs_len);
    // SAFETY: the build script wrote `blk_client_config_t` followed by `fs_server_config_t`.
    let blk =
        unsafe { blk_image::blk_config_from_bytes::<blk_client_config_t>(&CONFIG[..blk_len]) };
    let fs = unsafe { fs_image::fs_config_from_bytes::<fs_server_config_t>(&CONFIG[blk_len..]) };
    assert_eq!(blk.magic, SDDF_BLK_MAGIC);
    assert_eq!(fs.magic, LIONS_FS_MAGIC);
    assert_eq!(blk.virt.id, BLK_CLIENT_VIRT_CHANNEL);
    assert_eq!(fs.client.id, FS_SERVER_CLIENT_CHANNEL);
    let mut queues = blk_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps the client queues and the storage-info page here.
    unsafe {
        blk_queue_init(
            &mut queues,
            blk.virt.req_queue.vaddr.cast(),
            blk.virt.resp_queue.vaddr.cast(),
            u32::from(blk.virt.num_buffers),
        );
        let state = &mut *state();
        state.blk = queues;
        state.commands = fs.client.command_queue.vaddr.cast();
        state.completions = fs.client.completion_queue.vaddr.cast();
        state.share = fs.client.share.vaddr;
        state.storage = blk.virt.storage_info.vaddr.cast();
    }
    // SAFETY: `STACKS` is exclusive to this init and the spawned worker.
    let stacks = unsafe { &mut *STACKS.0.get() };
    let ptrs = core::array::from_fn(|index| stacks[index].0.as_mut_ptr());
    // SAFETY: the static stacks outlive the protection domain and do not overlap.
    unsafe { microkit_cothread_init(STACK_SIZE, &ptrs) };
    let handle = microkit_cothread_spawn(worker, core::ptr::null_mut());
    assert_ne!(handle, NULL_HANDLE, "worker cothread slot is free");
    microkit_cothread_yield();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(Channel::new(BLK_CH)) {
            microkit_cothread_recv_ntfn(BLK_CH);
        }
        if channels.contains(Channel::new(FS_CH)) {
            microkit_cothread_recv_ntfn(FS_CH);
        }
        Ok(())
    }
}
