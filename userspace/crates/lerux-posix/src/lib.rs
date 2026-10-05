//! File-descriptor client for the LionsOS filesystem queue.
//!
//! One [`Files`] value issues `open`, `read`, `write`, `mkdir`, `unlink`,
//! `rename`, `stat`, `open_dir`, `read_dir`, and `close_dir`. Paths and file
//! bytes live in a share region. The
//! command ring and the completion ring are `fs_queue_t` values from
//! `lerux-sddf`. The caller supplies a wake function. The host test serves
//! the queue inside that function. A later guest notifies its server and
//! waits there. The file methods do not branch on which wake is bound.

#![cfg_attr(not(test), no_std)]

use core::{mem::size_of, ptr};

use lerux_sddf::{
    fs_buffer_t, fs_cmd_params_dir_close_t, fs_cmd_params_dir_create_t, fs_cmd_params_dir_open_t,
    fs_cmd_params_dir_read_t, fs_cmd_params_file_close_t, fs_cmd_params_file_open_t,
    fs_cmd_params_file_read_t, fs_cmd_params_file_remove_t, fs_cmd_params_file_write_t,
    fs_cmd_params_rename_t, fs_cmd_params_stat_t, fs_cmd_params_t, fs_cmd_t, fs_cmpl_t,
    fs_command_enqueue, fs_message_dequeue, fs_msg_t, fs_queue_t, fs_stat_t, FS_CMD_DIR_CLOSE,
    FS_CMD_DIR_CREATE, FS_CMD_DIR_OPEN, FS_CMD_DIR_READ, FS_CMD_FILE_CLOSE, FS_CMD_FILE_OPEN,
    FS_CMD_FILE_READ, FS_CMD_FILE_REMOVE, FS_CMD_FILE_WRITE, FS_CMD_RENAME, FS_CMD_STAT,
    FS_OPEN_FLAGS_CREATE, FS_OPEN_FLAGS_READ_ONLY, FS_OPEN_FLAGS_READ_WRITE,
    FS_OPEN_FLAGS_WRITE_ONLY, FS_STATUS_ALLOCATION_ERROR, FS_STATUS_ALREADY_EXISTS,
    FS_STATUS_DIRECTORY_IS_FULL, FS_STATUS_END_OF_DIRECTORY, FS_STATUS_ERROR,
    FS_STATUS_INVALID_BUFFER, FS_STATUS_INVALID_COMMAND, FS_STATUS_INVALID_FD,
    FS_STATUS_INVALID_NAME, FS_STATUS_INVALID_PATH, FS_STATUS_INVALID_READ,
    FS_STATUS_INVALID_WRITE, FS_STATUS_NOT_DIRECTORY, FS_STATUS_NOT_EMPTY, FS_STATUS_NO_FILE,
    FS_STATUS_OUTSTANDING_OPERATIONS, FS_STATUS_SERVER_WAS_DENIED, FS_STATUS_SUCCESS,
    FS_STATUS_TOO_MANY_OPEN_FILES,
};

/// Descriptor table length, including the three numbers this client never stores.
pub const MAX_FDS: usize = 128;

const TABLE_LEN: usize = MAX_FDS - 3;
const SHARE_MIN: usize = 0x8000;
const PATH_LIMIT: usize = 4095;
const PATH_A: usize = 0;
const PATH_B: usize = 4096;
const STAT_AT: usize = 8192;
const DATA_AT: usize = 8336;
const DATA_LEN: usize = SHARE_MIN - DATA_AT;

const _: () = assert!(size_of::<fs_stat_t>() == 136);
const _: () = assert!(STAT_AT + 136 <= DATA_AT);
const _: () = assert!(DATA_AT + DATA_LEN == SHARE_MIN);

/// Pinned errno integer.
///
/// `FILE_ERR` and `EPERM` are both 1. An enum of names cannot round-trip both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Errno(pub i32);

impl Errno {
    pub const FILE_ERR: i32 = 1;
    pub const EPERM: i32 = 1;
    pub const ENOENT: i32 = 2;
    pub const EBADF: i32 = 9;
    pub const ENOMEM: i32 = 12;
    pub const EACCES: i32 = 13;
    pub const EBUSY: i32 = 16;
    pub const EEXIST: i32 = 17;
    pub const ENOTDIR: i32 = 20;
    pub const EINVAL: i32 = 22;
    pub const EMFILE: i32 = 24;
    pub const ENOSPC: i32 = 28;
    pub const ENAMETOOLONG: i32 = 36;
    pub const ENOTEMPTY: i32 = 39;
}

/// How `open` maps onto the filesystem open flags.
///
/// Create is bit 4 ored with the access mode. Read-only is not a bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    Read,
    Write,
    ReadWrite,
    CreateRead,
    CreateWrite,
    CreateReadWrite,
}

/// Local descriptor. Values 0, 1, and 2 cannot be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fd(u8);

impl Fd {
    /// Descriptor number in `3..=127`.
    pub fn number(self) -> i32 {
        i32::from(self.0)
    }
}

/// Metadata copied out of the share after `FS_CMD_STAT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stat {
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

impl From<fs_stat_t> for Stat {
    fn from(raw: fs_stat_t) -> Self {
        Self {
            dev: raw.dev,
            ino: raw.ino,
            mode: raw.mode,
            nlink: raw.nlink,
            uid: raw.uid,
            gid: raw.gid,
            rdev: raw.rdev,
            size: raw.size,
            blksize: raw.blksize,
            blocks: raw.blocks,
            atime: raw.atime,
            mtime: raw.mtime,
            ctime: raw.ctime,
            atime_nsec: raw.atime_nsec,
            mtime_nsec: raw.mtime_nsec,
            ctime_nsec: raw.ctime_nsec,
            used: raw.used,
        }
    }
}

#[derive(Clone, Copy)]
enum Access {
    Read,
    Write,
    ReadWrite,
}

impl Access {
    fn can_read(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    fn can_write(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

#[derive(Clone, Copy)]
enum Slot {
    Empty,
    File {
        server: u64,
        access: Access,
        cursor: u64,
    },
    Dir {
        server: u64,
    },
}

enum Flight {
    Idle,
    Waiting(u64),
}

/// Client for one command ring, one completion ring, and one share.
///
/// The share windows are fixed because one command is in flight.
pub struct Files {
    command: *mut fs_queue_t,
    completion: *mut fs_queue_t,
    share: *mut u8,
    share_len: usize,
    wake: fn(*mut ()),
    wake_cx: *mut (),
    next_id: u64,
    flight: Flight,
    slots: [Slot; TABLE_LEN],
}

impl Files {
    /// Bind the rings, the share, and the wake.
    ///
    /// # Safety
    ///
    /// `commands`, `completions`, and `share` must outlive `self` and be valid
    /// for the life of this value. This value is the only client. It is the
    /// only writer of the command-ring tail and the completion-ring head. The
    /// peer is the only writer of the command-ring head and the completion-ring
    /// tail. `share` must hold at least `share_len` bytes. `wake` may run the
    /// peer or wait for it, and it must not call back into this value.
    pub unsafe fn bind(
        commands: *mut fs_queue_t,
        completions: *mut fs_queue_t,
        share: *mut u8,
        share_len: usize,
        wake: fn(*mut ()),
        wake_cx: *mut (),
    ) -> Result<Self, Errno> {
        if commands.is_null() || completions.is_null() || share.is_null() || share_len < SHARE_MIN {
            return Err(Errno(Errno::EINVAL));
        }
        Ok(Self {
            command: commands,
            completion: completions,
            share,
            share_len,
            wake,
            wake_cx,
            next_id: 1,
            flight: Flight::Idle,
            slots: [Slot::Empty; TABLE_LEN],
        })
    }

    /// Send `FS_CMD_FILE_OPEN`. The first success returns descriptor 3.
    pub fn open(&mut self, path: &[u8], how: Open) -> Result<Fd, Errno> {
        self.idle()?;
        let index = self
            .slots
            .iter()
            .position(|slot| matches!(slot, Slot::Empty))
            .ok_or(Errno(Errno::EMFILE))?;
        let path = self.place(path, PATH_A)?;
        let cmpl = self.exchange(open_command(path, wire_flags(how)))?;
        map_status(cmpl.status)?;
        let server = unsafe { cmpl.data.file_open.fd };
        self.slots[index] = Slot::File {
            server,
            access: access_of(how),
            cursor: 0,
        };
        Ok(Fd((index + 3) as u8))
    }

    /// Send `FS_CMD_DIR_OPEN`. The first success returns descriptor 3.
    pub fn open_dir(&mut self, path: &[u8]) -> Result<Fd, Errno> {
        self.idle()?;
        let index = self
            .slots
            .iter()
            .position(|slot| matches!(slot, Slot::Empty))
            .ok_or(Errno(Errno::EMFILE))?;
        let path = self.place(path, PATH_A)?;
        let cmpl = self.exchange(dir_open_command(path))?;
        map_status(cmpl.status)?;
        let server = unsafe { cmpl.data.dir_open.fd };
        self.slots[index] = Slot::Dir { server };
        Ok(Fd((index + 3) as u8))
    }

    /// Send `FS_CMD_DIR_READ` and copy one name out.
    ///
    /// `Ok(None)` is `FS_STATUS_END_OF_DIRECTORY`. That status is not errno 1
    /// for a directory read.
    pub fn read_dir(&mut self, fd: Fd, buf: &mut [u8]) -> Result<Option<usize>, Errno> {
        self.idle()?;
        let server = self.dir_live(fd)?;
        if buf.is_empty() {
            return Err(Errno(Errno::EINVAL));
        }
        let chunk = buf.len().min(DATA_LEN);
        let buf_desc = fs_buffer_t {
            offset: DATA_AT as u64,
            size: chunk as u64,
        };
        let cmpl = self.exchange(dir_read_command(server, buf_desc))?;
        if cmpl.status == FS_STATUS_END_OF_DIRECTORY {
            return Ok(None);
        }
        map_status(cmpl.status)?;
        let n = unsafe { cmpl.data.dir_read.path_len };
        let Some(n) = completed_len(n, chunk) else {
            return Err(Errno(Errno::FILE_ERR));
        };
        unsafe {
            ptr::copy_nonoverlapping(self.share.add(DATA_AT), buf.as_mut_ptr(), n);
        }
        Ok(Some(n))
    }

    /// Send `FS_CMD_FILE_READ` at the cursor and copy the completed bytes out.
    pub fn read(&mut self, fd: Fd, buf: &mut [u8]) -> Result<usize, Errno> {
        self.idle()?;
        let (server, access, mut cursor) = self.live(fd)?;
        if !access.can_read() {
            return Err(Errno(Errno::EACCES));
        }
        if buf.is_empty() {
            return Ok(0);
        }
        let mut done = 0;
        while done < buf.len() {
            let chunk = (buf.len() - done).min(DATA_LEN);
            let buf_desc = fs_buffer_t {
                offset: DATA_AT as u64,
                size: chunk as u64,
            };
            match self.exchange(read_command(server, cursor, buf_desc)) {
                Ok(cmpl) => {
                    if let Err(err) = map_status(cmpl.status) {
                        return keep_or(done, err);
                    }
                    let n = unsafe { cmpl.data.file_read.len_read };
                    let Some(n) = completed_len(n, chunk) else {
                        return keep_or(done, Errno(Errno::FILE_ERR));
                    };
                    unsafe {
                        ptr::copy_nonoverlapping(
                            self.share.add(DATA_AT),
                            buf.as_mut_ptr().add(done),
                            n,
                        );
                    }
                    cursor = cursor.wrapping_add(n as u64);
                    self.set_cursor(fd, cursor);
                    done += n;
                    if n < chunk {
                        break;
                    }
                }
                Err(err) => return keep_or(done, err),
            }
        }
        Ok(done)
    }

    /// Copy bytes into the share and send `FS_CMD_FILE_WRITE` at the cursor.
    pub fn write(&mut self, fd: Fd, buf: &[u8]) -> Result<usize, Errno> {
        self.idle()?;
        let (server, access, mut cursor) = self.live(fd)?;
        if !access.can_write() {
            return Err(Errno(Errno::EACCES));
        }
        if buf.is_empty() {
            return Ok(0);
        }
        let mut done = 0;
        while done < buf.len() {
            let chunk = (buf.len() - done).min(DATA_LEN);
            unsafe {
                ptr::copy_nonoverlapping(buf.as_ptr().add(done), self.share.add(DATA_AT), chunk);
            }
            let buf_desc = fs_buffer_t {
                offset: DATA_AT as u64,
                size: chunk as u64,
            };
            match self.exchange(write_command(server, cursor, buf_desc)) {
                Ok(cmpl) => {
                    if let Err(err) = map_status(cmpl.status) {
                        return keep_or(done, err);
                    }
                    let n = unsafe { cmpl.data.file_write.len_written };
                    let Some(n) = completed_len(n, chunk) else {
                        return keep_or(done, Errno(Errno::FILE_ERR));
                    };
                    cursor = cursor.wrapping_add(n as u64);
                    self.set_cursor(fd, cursor);
                    done += n;
                    if n < chunk {
                        break;
                    }
                }
                Err(err) => return keep_or(done, err),
            }
        }
        Ok(done)
    }

    /// Send `FS_CMD_DIR_CREATE`.
    pub fn mkdir(&mut self, path: &[u8]) -> Result<(), Errno> {
        self.idle()?;
        let path = self.place(path, PATH_A)?;
        let cmpl = self.exchange(dir_create_command(path))?;
        map_status(cmpl.status)
    }

    /// Send `FS_CMD_FILE_REMOVE`.
    pub fn unlink(&mut self, path: &[u8]) -> Result<(), Errno> {
        self.idle()?;
        let path = self.place(path, PATH_A)?;
        let cmpl = self.exchange(remove_command(path))?;
        map_status(cmpl.status)
    }

    /// Send `FS_CMD_RENAME` with both paths.
    pub fn rename(&mut self, from: &[u8], to: &[u8]) -> Result<(), Errno> {
        self.idle()?;
        let old_path = self.place(from, PATH_A)?;
        let new_path = self.place(to, PATH_B)?;
        let cmpl = self.exchange(rename_command(old_path, new_path))?;
        map_status(cmpl.status)
    }

    /// Send `FS_CMD_STAT` and copy the object the server wrote.
    pub fn stat(&mut self, path: &[u8]) -> Result<Stat, Errno> {
        self.idle()?;
        let path = self.place(path, PATH_A)?;
        let cmpl = self.exchange(stat_command(path))?;
        map_status(cmpl.status)?;
        let raw = unsafe { ptr::read_unaligned(self.share.add(STAT_AT).cast::<fs_stat_t>()) };
        Ok(Stat::from(raw))
    }

    /// Send `FS_CMD_FILE_CLOSE`.
    ///
    /// A completion frees the local slot even when the status is not success.
    /// A retry then returns [`Errno::EBADF`] instead of sending `FILE_CLOSE`
    /// again for a descriptor this table has already dropped. A missing
    /// completion leaves the slot open.
    pub fn close(&mut self, fd: Fd) -> Result<(), Errno> {
        self.idle()?;
        let Slot::File { server, .. } = self.slots[index(fd)] else {
            return Err(Errno(Errno::EBADF));
        };
        let cmpl = self.exchange(close_command(server))?;
        self.slots[index(fd)] = Slot::Empty;
        map_status(cmpl.status)
    }

    /// Send `FS_CMD_DIR_CLOSE`.
    ///
    /// A completion frees the local slot even when the status is not success.
    pub fn close_dir(&mut self, fd: Fd) -> Result<(), Errno> {
        self.idle()?;
        let Slot::Dir { server } = self.slots[index(fd)] else {
            return Err(Errno(Errno::EBADF));
        };
        let cmpl = self.exchange(dir_close_command(server))?;
        self.slots[index(fd)] = Slot::Empty;
        map_status(cmpl.status)
    }

    fn idle(&self) -> Result<(), Errno> {
        match self.flight {
            Flight::Idle => Ok(()),
            Flight::Waiting(_) => Err(Errno(Errno::EBUSY)),
        }
    }

    fn live(&self, fd: Fd) -> Result<(u64, Access, u64), Errno> {
        match self.slots[index(fd)] {
            Slot::File {
                server,
                access,
                cursor,
            } => Ok((server, access, cursor)),
            Slot::Empty | Slot::Dir { .. } => Err(Errno(Errno::EBADF)),
        }
    }

    fn dir_live(&self, fd: Fd) -> Result<u64, Errno> {
        match self.slots[index(fd)] {
            Slot::Dir { server } => Ok(server),
            Slot::Empty | Slot::File { .. } => Err(Errno(Errno::EBADF)),
        }
    }

    fn set_cursor(&mut self, fd: Fd, cursor: u64) {
        if let Slot::File {
            cursor: slot_cursor,
            ..
        } = &mut self.slots[index(fd)]
        {
            *slot_cursor = cursor;
        }
    }

    fn place(&mut self, path: &[u8], offset: usize) -> Result<fs_buffer_t, Errno> {
        if path.len() > PATH_LIMIT {
            return Err(Errno(Errno::ENAMETOOLONG));
        }
        let end = offset.checked_add(path.len()).ok_or(Errno(Errno::EINVAL))?;
        if end > self.share_len {
            return Err(Errno(Errno::EINVAL));
        }
        unsafe {
            ptr::copy_nonoverlapping(path.as_ptr(), self.share.add(offset), path.len());
        }
        Ok(fs_buffer_t {
            offset: offset as u64,
            size: path.len() as u64,
        })
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }
        id
    }

    /// Enqueue, wake, and dequeue.
    ///
    /// Two wakes is the bound. A third miss returns errno 1 instead of spinning
    /// when the peer never completes.
    fn exchange(&mut self, mut cmd: fs_cmd_t) -> Result<fs_cmpl_t, Errno> {
        if matches!(self.flight, Flight::Waiting(_)) {
            return Err(Errno(Errno::EBUSY));
        }
        let id = self.alloc_id();
        cmd.id = id;
        self.flight = Flight::Waiting(id);
        let enqueued = unsafe { fs_command_enqueue(self.command, cmd) };
        if enqueued != 0 {
            self.flight = Flight::Idle;
            return Err(Errno(Errno::ENOMEM));
        }
        for _ in 0..2 {
            (self.wake)(self.wake_cx);
            if let Some(cmpl) = self.take_match() {
                self.flight = Flight::Idle;
                return Ok(cmpl);
            }
        }
        self.flight = Flight::Idle;
        Err(Errno(Errno::FILE_ERR))
    }

    fn take_match(&mut self) -> Option<fs_cmpl_t> {
        let Flight::Waiting(id) = self.flight else {
            return None;
        };
        let mut msg = fs_msg_t {
            cmd: zeroed_command(),
        };
        let got = unsafe { fs_message_dequeue(self.completion, &mut msg) };
        if got != 0 {
            return None;
        }
        let cmpl = unsafe { msg.cmpl };
        (cmpl.id == id).then_some(cmpl)
    }
}

fn index(fd: Fd) -> usize {
    fd.0 as usize - 3
}

fn completed_len(n: u64, chunk: usize) -> Option<usize> {
    let n = usize::try_from(n).ok()?;
    (n <= chunk).then_some(n)
}

fn keep_or(done: usize, err: Errno) -> Result<usize, Errno> {
    if done > 0 {
        Ok(done)
    } else {
        Err(err)
    }
}

fn access_of(how: Open) -> Access {
    match how {
        Open::Read | Open::CreateRead => Access::Read,
        Open::Write | Open::CreateWrite => Access::Write,
        Open::ReadWrite | Open::CreateReadWrite => Access::ReadWrite,
    }
}

fn wire_flags(how: Open) -> u64 {
    match how {
        Open::Read => FS_OPEN_FLAGS_READ_ONLY,
        Open::Write => FS_OPEN_FLAGS_WRITE_ONLY,
        Open::ReadWrite => FS_OPEN_FLAGS_READ_WRITE,
        Open::CreateRead => FS_OPEN_FLAGS_CREATE | FS_OPEN_FLAGS_READ_ONLY,
        Open::CreateWrite => FS_OPEN_FLAGS_CREATE | FS_OPEN_FLAGS_WRITE_ONLY,
        Open::CreateReadWrite => FS_OPEN_FLAGS_CREATE | FS_OPEN_FLAGS_READ_WRITE,
    }
}

fn zeroed_params() -> fs_cmd_params_t {
    fs_cmd_params_t { min_size: [0; 48] }
}

fn zeroed_command() -> fs_cmd_t {
    fs_cmd_t {
        id: 0,
        r#type: 0,
        params: zeroed_params(),
    }
}

fn with_params(kind: u64, fill: impl FnOnce(&mut fs_cmd_params_t)) -> fs_cmd_t {
    let mut params = zeroed_params();
    fill(&mut params);
    fs_cmd_t {
        id: 0,
        r#type: kind,
        params,
    }
}

fn open_command(path: fs_buffer_t, flags: u64) -> fs_cmd_t {
    with_params(FS_CMD_FILE_OPEN, |params| {
        params.file_open = fs_cmd_params_file_open_t { path, flags };
    })
}

fn read_command(fd: u64, offset: u64, buf: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_FILE_READ, |params| {
        params.file_read = fs_cmd_params_file_read_t { fd, offset, buf };
    })
}

fn write_command(fd: u64, offset: u64, buf: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_FILE_WRITE, |params| {
        params.file_write = fs_cmd_params_file_write_t { fd, offset, buf };
    })
}

fn close_command(fd: u64) -> fs_cmd_t {
    with_params(FS_CMD_FILE_CLOSE, |params| {
        params.file_close = fs_cmd_params_file_close_t { fd };
    })
}

fn dir_create_command(path: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_DIR_CREATE, |params| {
        params.dir_create = fs_cmd_params_dir_create_t { path };
    })
}

fn dir_open_command(path: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_DIR_OPEN, |params| {
        params.dir_open = fs_cmd_params_dir_open_t { path };
    })
}

fn dir_read_command(fd: u64, buf: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_DIR_READ, |params| {
        params.dir_read = fs_cmd_params_dir_read_t { fd, buf };
    })
}

fn dir_close_command(fd: u64) -> fs_cmd_t {
    with_params(FS_CMD_DIR_CLOSE, |params| {
        params.dir_close = fs_cmd_params_dir_close_t { fd };
    })
}

fn remove_command(path: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_FILE_REMOVE, |params| {
        params.file_remove = fs_cmd_params_file_remove_t { path };
    })
}

fn rename_command(old_path: fs_buffer_t, new_path: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_RENAME, |params| {
        params.rename = fs_cmd_params_rename_t { old_path, new_path };
    })
}

fn stat_command(path: fs_buffer_t) -> fs_cmd_t {
    with_params(FS_CMD_STAT, |params| {
        params.stat = fs_cmd_params_stat_t {
            path,
            buf: fs_buffer_t {
                offset: STAT_AT as u64,
                size: size_of::<fs_stat_t>() as u64,
            },
        };
    })
}

fn map_status(status: u64) -> Result<(), Errno> {
    let errno = match status {
        FS_STATUS_SUCCESS => return Ok(()),
        FS_STATUS_ERROR | FS_STATUS_END_OF_DIRECTORY | FS_STATUS_SERVER_WAS_DENIED => {
            Errno::FILE_ERR
        }
        FS_STATUS_INVALID_BUFFER | FS_STATUS_INVALID_NAME | FS_STATUS_INVALID_COMMAND => {
            Errno::EINVAL
        }
        FS_STATUS_INVALID_PATH | FS_STATUS_NO_FILE => Errno::ENOENT,
        FS_STATUS_INVALID_FD => Errno::EBADF,
        FS_STATUS_ALLOCATION_ERROR => Errno::ENOMEM,
        FS_STATUS_OUTSTANDING_OPERATIONS => Errno::EBUSY,
        FS_STATUS_TOO_MANY_OPEN_FILES => Errno::EMFILE,
        FS_STATUS_INVALID_WRITE | FS_STATUS_INVALID_READ => Errno::EACCES,
        FS_STATUS_DIRECTORY_IS_FULL => Errno::ENOSPC,
        FS_STATUS_NOT_DIRECTORY => Errno::ENOTDIR,
        FS_STATUS_ALREADY_EXISTS => Errno::EEXIST,
        FS_STATUS_NOT_EMPTY => Errno::ENOTEMPTY,
        _ => Errno::FILE_ERR,
    };
    Err(Errno(errno))
}
