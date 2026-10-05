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
const FD_FILE: u64 = 0;
const FD_DIR: u64 = 1;
const CLUSTER_SCAN: u16 = 64;
const S_IFDIR: u64 = 0o040000;
const S_IFREG: u64 = 0o100000;
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
    dir_index: u16,
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
    file: Option<OpenFile>,
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
    file: None,
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
    log::info!("fatfs: serving");
    true
}

fn volume_ok() -> bool {
    // SAFETY: `load_block(0)` filled the static block buffer.
    unsafe {
        let bytes = &*block();
        bytes[510] == 0x55
            && bytes[511] == 0xAA
            && u16::from_le_bytes([bytes[11], bytes[12]]) == SECTOR_BYTES as u16
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
    sector[11..13].copy_from_slice(&(SECTOR_BYTES as u16).to_le_bytes());
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
        _ => complete(cmd.id, FS_STATUS_INVALID_COMMAND, Completion::Empty),
    }
}

fn handle_open(id: u64, params: fs_cmd_params_file_open_t) -> bool {
    if params.path.size > 11 {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    }
    let Ok(size) = usize::try_from(params.path.size) else {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    };
    let mut raw = [0u8; 11];
    if !copy_share(params.path.offset, size, &mut raw[..size]) {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    }
    let Some(name) = pack_name(&raw[..size]) else {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    };
    if params.flags == (FS_OPEN_FLAGS_CREATE | FS_OPEN_FLAGS_WRITE_ONLY) {
        create_file(id, &name)
    } else if params.flags == FS_OPEN_FLAGS_READ_ONLY {
        open_read(id, &name)
    } else {
        complete(id, FS_STATUS_ERROR, Completion::Empty)
    }
}

fn file_busy() -> bool {
    unsafe { (*state()).file.is_some() }
}

fn create_file(id: u64, name: &[u8; 11]) -> bool {
    if file_busy() {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, Completion::Empty);
    }
    match find_name(name) {
        Scan::Hit(_) => return complete(id, FS_STATUS_ALREADY_EXISTS, Completion::Empty),
        Scan::Missing => {}
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    }
    let slot = match find_free() {
        Scan::Hit(index) => index,
        Scan::Missing => {
            return complete(id, FS_STATUS_DIRECTORY_IS_FULL, Completion::Empty);
        }
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    };
    let cluster = match alloc_cluster() {
        Scan::Hit(cluster) => cluster,
        Scan::Missing => {
            return complete(id, FS_STATUS_DIRECTORY_IS_FULL, Completion::Empty);
        }
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    };
    if !set_fat(cluster, 0xFFFF)
        || !write_dir(slot, name, cluster, 0, ATTR_ARCHIVE)
        || !flush_block()
    {
        let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
        return false;
    }
    unsafe {
        (*state()).file = Some(OpenFile {
            cluster,
            size: 0,
            dir_index: slot,
        });
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Fd(FD_FILE))
}

fn open_read(id: u64, name: &[u8; 11]) -> bool {
    if file_busy() {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, Completion::Empty);
    }
    match find_name(name) {
        Scan::Hit(hit) => {
            if hit.attr & ATTR_DIR != 0 || hit.cluster < 2 {
                return complete(id, FS_STATUS_ERROR, Completion::Empty);
            }
            unsafe {
                (*state()).file = Some(OpenFile {
                    cluster: hit.cluster,
                    size: hit.size,
                    dir_index: hit.index,
                });
            }
            complete(id, FS_STATUS_SUCCESS, Completion::Fd(FD_FILE))
        }
        Scan::Missing => complete(id, FS_STATUS_NO_FILE, Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            false
        }
    }
}

fn handle_close(id: u64, params: fs_cmd_params_file_close_t) -> bool {
    if params.fd != FD_FILE || unsafe { (*state()).file.is_none() } {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    }
    unsafe {
        (*state()).file = None;
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Empty)
}

fn handle_write(id: u64, params: fs_cmd_params_file_write_t) -> bool {
    let Some(file) = (unsafe { (*state()).file }) else {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    };
    if params.fd != FD_FILE {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    }
    if params.offset != 0 || params.buf.size == 0 || params.buf.size > SECTOR_BYTES as u64 {
        return complete(id, FS_STATUS_INVALID_WRITE, Completion::Empty);
    }
    let Ok(len) = usize::try_from(params.buf.size) else {
        return complete(id, FS_STATUS_INVALID_WRITE, Completion::Empty);
    };
    let mut data = [0u8; SECTOR_BYTES];
    if !copy_share(params.buf.offset, len, &mut data[..len]) {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    }
    let lba = DATA_LBA + u32::from(file.cluster - 2);
    if !edit_sector(lba, |sector| {
        sector[..len].copy_from_slice(&data[..len]);
    }) {
        let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
        return false;
    }
    let Ok(size) = u32::try_from(len) else {
        return complete(id, FS_STATUS_INVALID_WRITE, Completion::Empty);
    };
    if !update_size(file.dir_index, size) || !flush_block() {
        let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
        return false;
    }
    unsafe {
        if let Some(open) = (*state()).file.as_mut() {
            open.size = size;
        }
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Write(params.buf.size))
}

fn handle_read(id: u64, params: fs_cmd_params_file_read_t) -> bool {
    let Some(file) = (unsafe { (*state()).file }) else {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    };
    if params.fd != FD_FILE {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    }
    if params.offset != 0 {
        return complete(id, FS_STATUS_INVALID_READ, Completion::Empty);
    }
    let Ok(buf_len) = usize::try_from(params.buf.size) else {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    };
    let Ok(buf_off) = usize::try_from(params.buf.offset) else {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    };
    let Ok(share_len) = usize::try_from(FS_REGION_SIZE) else {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    };
    if buf_off
        .checked_add(buf_len)
        .is_none_or(|end| end > share_len)
    {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    }
    let file_size = usize::try_from(file.size).unwrap_or(0);
    let copied = buf_len.min(file_size);
    if copied > 0 {
        if file.cluster < 2 {
            return complete(id, FS_STATUS_ERROR, Completion::Empty);
        }
        let lba = DATA_LBA + u32::from(file.cluster - 2);
        if !load_block(u64::from(lba / SECTORS_PER_TRANSFER)) {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
        let offset = ((lba % SECTORS_PER_TRANSFER) as usize) * SECTOR_BYTES;
        // SAFETY: the share range was checked, and the sector sits inside the static block buffer.
        unsafe {
            core::ptr::copy_nonoverlapping(
                (*block()).as_ptr().add(offset),
                (*state()).share.add(buf_off),
                copied,
            );
        }
    }
    let Ok(len_read) = u64::try_from(copied) else {
        return complete(id, FS_STATUS_ERROR, Completion::Empty);
    };
    complete(id, FS_STATUS_SUCCESS, Completion::Read(len_read))
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

fn put_entry(entry: &mut [u8], name: &[u8; 11], cluster: u16, size: u32, attr: u8) {
    entry.fill(0);
    entry[..11].copy_from_slice(name);
    entry[11] = attr;
    entry[26..28].copy_from_slice(&cluster.to_le_bytes());
    entry[28..32].copy_from_slice(&size.to_le_bytes());
}

fn update_size(index: u16, size: u32) -> bool {
    let lba = ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR);
    let off = usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY;
    edit_sector(lba, |sector| {
        sector[off + 28..off + 32].copy_from_slice(&size.to_le_bytes());
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
    for cluster in 2..CLUSTER_SCAN {
        match read_fat(cluster) {
            Scan::Hit(0) => return Scan::Hit(cluster),
            Scan::Hit(_) => {}
            Scan::Missing => return Scan::Missing,
            Scan::Failed => return Scan::Failed,
        }
    }
    Scan::Missing
}

fn handle_stat(id: u64, params: fs_cmd_params_stat_t) -> bool {
    if params.buf.size < core::mem::size_of::<fs_stat_t>() as u64 {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    }
    let mut raw = [0u8; 11];
    let hit = if is_root_path_buf(params.path) {
        None
    } else {
        let Some(name) = read_packed_name(params.path, &mut raw) else {
            return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
        };
        match find_name(&name) {
            Scan::Hit(hit) => Some(hit),
            Scan::Missing => return complete(id, FS_STATUS_NO_FILE, Completion::Empty),
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
                return false;
            }
        }
    };
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
    if !write_share(params.buf, &stat) {
        return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Empty)
}

fn handle_rename(id: u64, params: fs_cmd_params_rename_t) -> bool {
    let mut old_raw = [0u8; 11];
    let mut new_raw = [0u8; 11];
    let Some(old) = read_packed_name(params.old_path, &mut old_raw) else {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    };
    let Some(new) = read_packed_name(params.new_path, &mut new_raw) else {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    };
    let hit = match find_name(&old) {
        Scan::Hit(hit) => hit,
        Scan::Missing => return complete(id, FS_STATUS_NO_FILE, Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    };
    if old != new {
        match find_name(&new) {
            Scan::Hit(_) => return complete(id, FS_STATUS_ALREADY_EXISTS, Completion::Empty),
            Scan::Missing => {}
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
                return false;
            }
        }
    }
    if !rename_entry(hit.index, &new) || !flush_block() {
        let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
        return false;
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Empty)
}

fn handle_remove(id: u64, params: fs_cmd_params_file_remove_t) -> bool {
    let mut raw = [0u8; 11];
    let Some(name) = read_packed_name(params.path, &mut raw) else {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    };
    let hit = match find_name(&name) {
        Scan::Hit(hit) => hit,
        Scan::Missing => return complete(id, FS_STATUS_NO_FILE, Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    };
    if hit.attr & ATTR_DIR != 0 {
        match directory_empty(hit.cluster) {
            Scan::Hit(true) => {}
            Scan::Hit(false) => return complete(id, FS_STATUS_NOT_EMPTY, Completion::Empty),
            Scan::Missing => return complete(id, FS_STATUS_ERROR, Completion::Empty),
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
                return false;
            }
        }
    }
    let free = if hit.cluster >= 2 {
        set_fat(hit.cluster, 0)
    } else {
        true
    };
    if !free || !mark_deleted(hit.index) || !flush_block() {
        let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
        return false;
    }
    unsafe {
        if (*state())
            .file
            .as_ref()
            .is_some_and(|file| file.dir_index == hit.index)
        {
            (*state()).file = None;
        }
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Empty)
}

fn handle_mkdir(id: u64, params: fs_cmd_params_dir_create_t) -> bool {
    let mut raw = [0u8; 11];
    let Some(name) = read_packed_name(params.path, &mut raw) else {
        return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
    };
    match find_name(&name) {
        Scan::Hit(_) => return complete(id, FS_STATUS_ALREADY_EXISTS, Completion::Empty),
        Scan::Missing => {}
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    }
    let slot = match find_free() {
        Scan::Hit(index) => index,
        Scan::Missing => return complete(id, FS_STATUS_DIRECTORY_IS_FULL, Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    };
    let cluster = match alloc_cluster() {
        Scan::Hit(cluster) => cluster,
        Scan::Missing => return complete(id, FS_STATUS_DIRECTORY_IS_FULL, Completion::Empty),
        Scan::Failed => {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
    };
    if !set_fat(cluster, 0xFFFF)
        || !format_directory(cluster)
        || !write_dir(slot, &name, cluster, 0, ATTR_DIR)
        || !flush_block()
    {
        let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
        return false;
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Empty)
}

fn handle_dir_open(id: u64, params: fs_cmd_params_dir_open_t) -> bool {
    if unsafe { (*state()).dir.is_some() } {
        return complete(id, FS_STATUS_TOO_MANY_OPEN_FILES, Completion::Empty);
    }
    let opened = if is_root_path_buf(params.path) {
        OpenDir {
            cluster: 0,
            index: 0,
            limit: ROOT_ENTRIES,
        }
    } else {
        let mut raw = [0u8; 11];
        let Some(name) = read_packed_name(params.path, &mut raw) else {
            return complete(id, FS_STATUS_INVALID_NAME, Completion::Empty);
        };
        match find_name(&name) {
            Scan::Hit(hit) if hit.attr & ATTR_DIR != 0 && hit.cluster >= 2 => OpenDir {
                cluster: hit.cluster,
                index: 0,
                limit: ENTRIES_PER_SECTOR,
            },
            Scan::Hit(_) | Scan::Missing => {
                return complete(id, FS_STATUS_NO_FILE, Completion::Empty);
            }
            Scan::Failed => {
                let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
                return false;
            }
        }
    };
    unsafe {
        (*state()).dir = Some(opened);
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Fd(FD_DIR))
}

fn handle_dir_read(id: u64, params: fs_cmd_params_dir_read_t) -> bool {
    let Some(dir) = (unsafe { (*state()).dir }) else {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    };
    if params.fd != FD_DIR {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    }
    let mut index = dir.index;
    while index < dir.limit {
        let mut entry = [0u8; DIR_ENTRY];
        if !read_placed(dir.cluster, index, &mut entry) {
            let _ = complete(id, FS_STATUS_ERROR, Completion::Empty);
            return false;
        }
        index += 1;
        if entry[0] == 0x00 {
            unsafe {
                if let Some(open) = (*state()).dir.as_mut() {
                    open.index = index;
                }
            }
            return complete(id, FS_STATUS_END_OF_DIRECTORY, Completion::DirRead(0));
        }
        if entry[0] == 0xE5 || is_dot(&entry) {
            continue;
        }
        let (shown, len) = display_name(&entry);
        if (len as u64) > params.buf.size {
            return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
        }
        if !copy_to_share(params.buf.offset, &shown[..len]) {
            return complete(id, FS_STATUS_INVALID_BUFFER, Completion::Empty);
        }
        unsafe {
            if let Some(open) = (*state()).dir.as_mut() {
                open.index = index;
            }
        }
        return complete(id, FS_STATUS_SUCCESS, Completion::DirRead(len as u64));
    }
    complete(id, FS_STATUS_END_OF_DIRECTORY, Completion::DirRead(0))
}

fn handle_dir_close(id: u64, fd: u64) -> bool {
    if fd != FD_DIR || unsafe { (*state()).dir.is_none() } {
        return complete(id, FS_STATUS_INVALID_FD, Completion::Empty);
    }
    unsafe {
        (*state()).dir = None;
    }
    complete(id, FS_STATUS_SUCCESS, Completion::Empty)
}

fn is_root_path_buf(path: fs_buffer_t) -> bool {
    if path.size == 0 {
        return true;
    }
    if path.size > 11 {
        return false;
    }
    let Ok(size) = usize::try_from(path.size) else {
        return false;
    };
    let mut raw = [0u8; 11];
    if !copy_share(path.offset, size, &mut raw[..size]) {
        return false;
    }
    raw[..size] == *b"." || raw[..size] == *b"/"
}

fn read_packed_name(path: fs_buffer_t, raw: &mut [u8; 11]) -> Option<[u8; 11]> {
    if path.size > 11 {
        return None;
    }
    let size = usize::try_from(path.size).ok()?;
    if !copy_share(path.offset, size, &mut raw[..size]) {
        return None;
    }
    pack_name(&raw[..size])
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

fn format_directory(cluster: u16) -> bool {
    let lba = DATA_LBA + u32::from(cluster - 2);
    let dot = dot_name(1);
    let dotdot = dot_name(2);
    edit_sector(lba, |sector| {
        sector.fill(0);
        put_entry(&mut sector[..DIR_ENTRY], &dot, cluster, 0, ATTR_DIR);
        put_entry(
            &mut sector[DIR_ENTRY..DIR_ENTRY * 2],
            &dotdot,
            0,
            0,
            ATTR_DIR,
        );
    })
}

fn rename_entry(index: u16, name: &[u8; 11]) -> bool {
    let lba = ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR);
    let off = usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY;
    edit_sector(lba, |sector| {
        sector[off..off + 11].copy_from_slice(name);
    })
}

fn mark_deleted(index: u16) -> bool {
    let lba = ROOT_LBA + u32::from(index / ENTRIES_PER_SECTOR);
    let off = usize::from(index % ENTRIES_PER_SECTOR) * DIR_ENTRY;
    edit_sector(lba, |sector| {
        sector[off] = 0xE5;
    })
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

fn complete(id: u64, status: u64, data: Completion) -> bool {
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
            cmpl.data.file_open.fd = fd;
            cmpl.data.dir_open.fd = fd;
        }
        Completion::Read(len) => cmpl.data.file_read.len_read = len,
        Completion::Write(len) => cmpl.data.file_write.len_written = len,
        Completion::DirRead(len) => cmpl.data.dir_read.path_len = len,
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
