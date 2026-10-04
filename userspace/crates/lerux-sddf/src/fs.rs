//! LionsOS filesystem command queue.
//!
//! `fs_cmd_t` and `fs_msg_t` are 64 bytes, which `include/lions/fs/protocol.h` asserts.

use core::sync::atomic::AtomicU64;

pub const FS_QUEUE_CAPACITY: usize = 511;
pub const FS_MAX_NAME_LENGTH: usize = 255;
pub const FS_MAX_PATH_LENGTH: usize = 4095;

pub const FS_OPEN_FLAGS_READ_ONLY: u64 = 0;
pub const FS_OPEN_FLAGS_WRITE_ONLY: u64 = 1;
pub const FS_OPEN_FLAGS_READ_WRITE: u64 = 2;
pub const FS_OPEN_FLAGS_CREATE: u64 = 4;

pub const FS_STATUS_SUCCESS: u64 = 0;
pub const FS_STATUS_ERROR: u64 = 1;
pub const FS_STATUS_INVALID_BUFFER: u64 = 2;
pub const FS_STATUS_INVALID_PATH: u64 = 3;
pub const FS_STATUS_INVALID_FD: u64 = 4;
pub const FS_STATUS_ALLOCATION_ERROR: u64 = 5;
pub const FS_STATUS_OUTSTANDING_OPERATIONS: u64 = 6;
pub const FS_STATUS_INVALID_NAME: u64 = 7;
pub const FS_STATUS_TOO_MANY_OPEN_FILES: u64 = 8;
pub const FS_STATUS_SERVER_WAS_DENIED: u64 = 9;
pub const FS_STATUS_INVALID_WRITE: u64 = 10;
pub const FS_STATUS_INVALID_READ: u64 = 11;
pub const FS_STATUS_DIRECTORY_IS_FULL: u64 = 12;
pub const FS_STATUS_INVALID_COMMAND: u64 = 13;
pub const FS_STATUS_END_OF_DIRECTORY: u64 = 14;
pub const FS_STATUS_NO_FILE: u64 = 15;
pub const FS_STATUS_NOT_DIRECTORY: u64 = 16;
pub const FS_STATUS_ALREADY_EXISTS: u64 = 17;
pub const FS_STATUS_NOT_EMPTY: u64 = 18;
pub const FS_STATUS_NUM_STATUSES: u64 = 19;

pub const FS_CMD_INITIALISE: u64 = 0;
pub const FS_CMD_DEINITIALISE: u64 = 1;
pub const FS_CMD_FILE_OPEN: u64 = 2;
pub const FS_CMD_FILE_CLOSE: u64 = 3;
pub const FS_CMD_STAT: u64 = 4;
pub const FS_CMD_FILE_READ: u64 = 5;
pub const FS_CMD_FILE_WRITE: u64 = 6;
pub const FS_CMD_FILE_SIZE: u64 = 7;
pub const FS_CMD_RENAME: u64 = 8;
pub const FS_CMD_FILE_REMOVE: u64 = 9;
pub const FS_CMD_FILE_TRUNCATE: u64 = 10;
pub const FS_CMD_DIR_CREATE: u64 = 11;
pub const FS_CMD_DIR_REMOVE: u64 = 12;
pub const FS_CMD_DIR_OPEN: u64 = 13;
pub const FS_CMD_DIR_CLOSE: u64 = 14;
pub const FS_CMD_FILE_SYNC: u64 = 15;
pub const FS_CMD_DIR_READ: u64 = 16;
pub const FS_CMD_DIR_SEEK: u64 = 17;
pub const FS_CMD_DIR_TELL: u64 = 18;
pub const FS_CMD_DIR_REWIND: u64 = 19;
pub const FS_NUM_COMMANDS: u64 = 20;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_stat_t {
    pub dev: u64,
    pub ino: u64,
    pub mode: u64,
    pub nlink: u64,
    pub uid: u64,
    pub gid: u64,
    pub rdev: u64,
    pub size: u64,
    pub blksize: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub atime_nsec: u64,
    pub mtime_nsec: u64,
    pub ctime_nsec: u64,
    pub used: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_buffer_t {
    pub offset: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_open_t {
    pub path: fs_buffer_t,
    pub flags: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_close_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_stat_t {
    pub path: fs_buffer_t,
    pub buf: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_read_t {
    pub fd: u64,
    pub offset: u64,
    pub buf: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_write_t {
    pub fd: u64,
    pub offset: u64,
    pub buf: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_size_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_rename_t {
    pub old_path: fs_buffer_t,
    pub new_path: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_remove_t {
    pub path: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_truncate_t {
    pub fd: u64,
    pub length: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_create_t {
    pub path: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_remove_t {
    pub path: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_open_t {
    pub path: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_close_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_read_t {
    pub fd: u64,
    pub buf: fs_buffer_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_file_sync_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_seek_t {
    pub fd: u64,
    pub loc: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_tell_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_params_dir_rewind_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union fs_cmd_params_t {
    pub file_open: fs_cmd_params_file_open_t,
    pub file_close: fs_cmd_params_file_close_t,
    pub stat: fs_cmd_params_stat_t,
    pub file_read: fs_cmd_params_file_read_t,
    pub file_write: fs_cmd_params_file_write_t,
    pub file_size: fs_cmd_params_file_size_t,
    pub rename: fs_cmd_params_rename_t,
    pub file_remove: fs_cmd_params_file_remove_t,
    pub file_truncate: fs_cmd_params_file_truncate_t,
    pub dir_create: fs_cmd_params_dir_create_t,
    pub dir_remove: fs_cmd_params_dir_remove_t,
    pub dir_open: fs_cmd_params_dir_open_t,
    pub dir_close: fs_cmd_params_dir_close_t,
    pub dir_read: fs_cmd_params_dir_read_t,
    pub file_sync: fs_cmd_params_file_sync_t,
    pub dir_seek: fs_cmd_params_dir_seek_t,
    pub dir_tell: fs_cmd_params_dir_tell_t,
    pub dir_rewind: fs_cmd_params_dir_rewind_t,
    pub min_size: [u8; 48],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmd_t {
    pub id: u64,
    pub r#type: u64,
    pub params: fs_cmd_params_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_file_open_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_file_read_t {
    pub len_read: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_file_write_t {
    pub len_written: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_file_size_t {
    pub size: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_dir_open_t {
    pub fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_dir_read_t {
    pub path_len: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_data_dir_tell_t {
    pub location: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union fs_cmpl_data_t {
    pub file_open: fs_cmpl_data_file_open_t,
    pub file_read: fs_cmpl_data_file_read_t,
    pub file_write: fs_cmpl_data_file_write_t,
    pub file_size: fs_cmpl_data_file_size_t,
    pub dir_open: fs_cmpl_data_dir_open_t,
    pub dir_read: fs_cmpl_data_dir_read_t,
    pub dir_tell: fs_cmpl_data_dir_tell_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fs_cmpl_t {
    pub id: u64,
    pub status: u64,
    pub data: fs_cmpl_data_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union fs_msg_t {
    pub cmd: fs_cmd_t,
    pub cmpl: fs_cmpl_t,
}

/// Command or completion ring. `head` and `tail` are free-running counters.
#[repr(C)]
pub struct fs_queue_t {
    pub head: AtomicU64,
    pub tail: AtomicU64,
    pub padding: [u8; 48],
    pub buffer: [fs_msg_t; FS_QUEUE_CAPACITY],
}
