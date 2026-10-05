//! Host checks for the file-descriptor client.
//!
//! The fake is the other side of two real filesystem rings. It dequeues with
//! `fs_message_dequeue` and completes with `fs_completion_enqueue`.

use core::{mem::zeroed, ptr};

use lerux_posix::{Errno, Files, Open};
use lerux_sddf::{
    fs_buffer_t, fs_cmd_t, fs_cmpl_t, fs_completion_enqueue, fs_message_dequeue, fs_msg_t,
    fs_queue_t, fs_stat_t, FS_CMD_DIR_CREATE, FS_CMD_FILE_CLOSE, FS_CMD_FILE_OPEN,
    FS_CMD_FILE_READ, FS_CMD_FILE_REMOVE, FS_CMD_FILE_WRITE, FS_CMD_RENAME, FS_CMD_STAT,
    FS_OPEN_FLAGS_CREATE, FS_STATUS_ALLOCATION_ERROR, FS_STATUS_ALREADY_EXISTS,
    FS_STATUS_INVALID_BUFFER, FS_STATUS_INVALID_COMMAND, FS_STATUS_INVALID_FD,
    FS_STATUS_INVALID_NAME, FS_STATUS_NOT_DIRECTORY, FS_STATUS_NO_FILE, FS_STATUS_SUCCESS,
};

const SHARE: usize = 0x8000;
const NAME_MAX: usize = 255;
const BODY_MAX: usize = 512;
const PAYLOAD: &[u8] = b"fs-sddf payload";

struct Entry {
    name: [u8; NAME_MAX],
    name_len: usize,
    body: [u8; BODY_MAX],
    body_len: usize,
    dir: bool,
    server: u64,
}

impl Entry {
    fn named(name: &[u8], dir: bool) -> Self {
        let mut entry = Self {
            name: [0; NAME_MAX],
            name_len: name.len(),
            body: [0; BODY_MAX],
            body_len: 0,
            dir,
            server: 0,
        };
        entry.name[..name.len()].copy_from_slice(name);
        entry
    }

    fn is_name(&self, name: &[u8]) -> bool {
        self.name_len == name.len() && self.name[..name.len()] == *name
    }
}

struct Fake {
    command: *mut fs_queue_t,
    completion: *mut fs_queue_t,
    share: *mut u8,
    files: [Option<Entry>; 4],
    next_server: u64,
    served: u64,
}

impl Fake {
    fn serve_one(&mut self) {
        let mut msg = fs_msg_t {
            cmd: empty_command(),
        };
        if unsafe { fs_message_dequeue(self.command, &mut msg) } != 0 {
            return;
        }
        self.served += 1;
        let cmd = unsafe { msg.cmd };
        match cmd.r#type {
            FS_CMD_FILE_OPEN => self.on_open(cmd),
            FS_CMD_FILE_CLOSE => self.on_close(cmd),
            FS_CMD_FILE_READ => self.on_read(cmd),
            FS_CMD_FILE_WRITE => self.on_write(cmd),
            FS_CMD_DIR_CREATE => self.on_mkdir(cmd),
            FS_CMD_FILE_REMOVE => self.on_remove(cmd),
            FS_CMD_RENAME => self.on_rename(cmd),
            FS_CMD_STAT => self.on_stat(cmd),
            _ => self.complete_none(cmd.id, FS_STATUS_INVALID_COMMAND),
        }
    }

    fn on_open(&mut self, cmd: fs_cmd_t) {
        let params = unsafe { cmd.params.file_open };
        let Some(name) = self.copy_name(params.path) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_NAME);
            return;
        };
        let create = params.flags & FS_OPEN_FLAGS_CREATE != 0;
        if let Some(index) = self.find_name(&name) {
            if self.files[index].as_ref().is_some_and(|entry| entry.dir) {
                self.complete_none(cmd.id, FS_STATUS_NOT_DIRECTORY);
                return;
            }
            let server = self.alloc_server();
            if let Some(entry) = self.files[index].as_mut() {
                entry.server = server;
            }
            self.complete_fd(cmd.id, server);
            return;
        }
        if !create {
            self.complete_none(cmd.id, FS_STATUS_NO_FILE);
            return;
        }
        let Some(index) = self.free_slot() else {
            self.complete_none(cmd.id, FS_STATUS_ALLOCATION_ERROR);
            return;
        };
        let mut entry = Entry::named(&name, false);
        entry.server = self.alloc_server();
        let server = entry.server;
        self.files[index] = Some(entry);
        self.complete_fd(cmd.id, server);
    }

    fn on_close(&mut self, cmd: fs_cmd_t) {
        let fd = unsafe { cmd.params.file_close.fd };
        let Some(index) = self.find_server(fd) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_FD);
            return;
        };
        if let Some(entry) = self.files[index].as_mut() {
            entry.server = 0;
        }
        self.complete_none(cmd.id, FS_STATUS_SUCCESS);
    }

    fn on_read(&mut self, cmd: fs_cmd_t) {
        let params = unsafe { cmd.params.file_read };
        let Some(index) = self.find_server(params.fd) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_FD);
            return;
        };
        let mut tmp = [0u8; BODY_MAX];
        let n = {
            let entry = self.files[index].as_ref().expect("server slot is occupied");
            let start = usize::try_from(params.offset).unwrap_or(usize::MAX);
            if start >= entry.body_len {
                0
            } else {
                let n = (entry.body_len - start).min(usize::try_from(params.buf.size).unwrap_or(0));
                tmp[..n].copy_from_slice(&entry.body[start..start + n]);
                n
            }
        };
        if n == 0 {
            self.complete_read(cmd.id, 0);
            return;
        }
        if !self.write_share(params.buf, &tmp[..n]) {
            self.complete_none(cmd.id, FS_STATUS_INVALID_BUFFER);
            return;
        }
        self.complete_read(cmd.id, n as u64);
    }

    fn on_write(&mut self, cmd: fs_cmd_t) {
        let params = unsafe { cmd.params.file_write };
        let Some(index) = self.find_server(params.fd) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_FD);
            return;
        };
        let start = usize::try_from(params.offset).unwrap_or(usize::MAX);
        if start >= BODY_MAX {
            self.complete_write(cmd.id, 0);
            return;
        }
        let room = BODY_MAX - start;
        let n = usize::try_from(params.buf.size).unwrap_or(0).min(room);
        let mut tmp = [0u8; BODY_MAX];
        if n > 0 && !self.read_share(params.buf, &mut tmp[..n], n) {
            self.complete_none(cmd.id, FS_STATUS_INVALID_BUFFER);
            return;
        }
        let entry = self.files[index].as_mut().expect("server slot is occupied");
        if n > 0 {
            entry.body[start..start + n].copy_from_slice(&tmp[..n]);
        }
        entry.body_len = entry.body_len.max(start + n);
        self.complete_write(cmd.id, n as u64);
    }

    fn on_mkdir(&mut self, cmd: fs_cmd_t) {
        let path = unsafe { cmd.params.dir_create.path };
        let Some(name) = self.copy_name(path) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_NAME);
            return;
        };
        if self.find_name(&name).is_some() {
            self.complete_none(cmd.id, FS_STATUS_ALREADY_EXISTS);
            return;
        }
        let Some(index) = self.free_slot() else {
            self.complete_none(cmd.id, FS_STATUS_ALLOCATION_ERROR);
            return;
        };
        self.files[index] = Some(Entry::named(&name, true));
        self.complete_none(cmd.id, FS_STATUS_SUCCESS);
    }

    fn on_remove(&mut self, cmd: fs_cmd_t) {
        let path = unsafe { cmd.params.file_remove.path };
        let Some(name) = self.copy_name(path) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_NAME);
            return;
        };
        let Some(index) = self.find_name(&name) else {
            self.complete_none(cmd.id, FS_STATUS_NO_FILE);
            return;
        };
        self.files[index] = None;
        self.complete_none(cmd.id, FS_STATUS_SUCCESS);
    }

    fn on_rename(&mut self, cmd: fs_cmd_t) {
        let params = unsafe { cmd.params.rename };
        let Some(old) = self.copy_name(params.old_path) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_NAME);
            return;
        };
        let Some(new) = self.copy_name(params.new_path) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_NAME);
            return;
        };
        let Some(index) = self.find_name(&old) else {
            self.complete_none(cmd.id, FS_STATUS_NO_FILE);
            return;
        };
        if old != new && self.find_name(&new).is_some() {
            self.complete_none(cmd.id, FS_STATUS_ALREADY_EXISTS);
            return;
        }
        if let Some(entry) = self.files[index].as_mut() {
            entry.name = [0; NAME_MAX];
            entry.name[..new.len()].copy_from_slice(&new);
            entry.name_len = new.len();
        }
        self.complete_none(cmd.id, FS_STATUS_SUCCESS);
    }

    fn on_stat(&mut self, cmd: fs_cmd_t) {
        let params = unsafe { cmd.params.stat };
        let Some(name) = self.copy_name(params.path) else {
            self.complete_none(cmd.id, FS_STATUS_INVALID_NAME);
            return;
        };
        let Some(index) = self.find_name(&name) else {
            self.complete_none(cmd.id, FS_STATUS_NO_FILE);
            return;
        };
        let size = self.files[index]
            .as_ref()
            .map(|entry| entry.body_len)
            .unwrap_or(0);
        let mut raw: fs_stat_t = unsafe { zeroed() };
        raw.size = size as u64;
        if !self.write_stat(params.buf, raw) {
            self.complete_none(cmd.id, FS_STATUS_INVALID_BUFFER);
            return;
        }
        self.complete_none(cmd.id, FS_STATUS_SUCCESS);
    }

    fn alloc_server(&mut self) -> u64 {
        let server = self.next_server;
        self.next_server += 1;
        server
    }

    fn find_name(&self, name: &[u8]) -> Option<usize> {
        self.files
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|entry| entry.is_name(name)))
    }

    fn find_server(&self, server: u64) -> Option<usize> {
        self.files.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|entry| entry.server == server && server != 0)
        })
    }

    fn free_slot(&self) -> Option<usize> {
        self.files.iter().position(|slot| slot.is_none())
    }

    fn copy_name(&self, buf: fs_buffer_t) -> Option<Vec<u8>> {
        let len = usize::try_from(buf.size).ok()?;
        if len > NAME_MAX {
            return None;
        }
        let mut name = vec![0; len];
        if len > 0 && !self.read_share(buf, &mut name, len) {
            return None;
        }
        Some(name)
    }

    fn read_share(&self, buf: fs_buffer_t, dst: &mut [u8], n: usize) -> bool {
        let Some(off) = share_off(buf.offset, n) else {
            return false;
        };
        if dst.len() < n {
            return false;
        }
        unsafe {
            ptr::copy_nonoverlapping(self.share.add(off), dst.as_mut_ptr(), n);
        }
        true
    }

    fn write_share(&self, buf: fs_buffer_t, src: &[u8]) -> bool {
        let Some(off) = share_off(buf.offset, src.len()) else {
            return false;
        };
        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), self.share.add(off), src.len());
        }
        true
    }

    fn write_stat(&self, buf: fs_buffer_t, raw: fs_stat_t) -> bool {
        let Some(off) = share_off(buf.offset, core::mem::size_of::<fs_stat_t>()) else {
            return false;
        };
        unsafe {
            ptr::write_unaligned(self.share.add(off).cast::<fs_stat_t>(), raw);
        }
        true
    }

    fn complete_fd(&mut self, id: u64, fd: u64) {
        self.finish(id, FS_STATUS_SUCCESS, Complete::Fd(fd));
    }

    fn complete_read(&mut self, id: u64, len: u64) {
        self.finish(id, FS_STATUS_SUCCESS, Complete::Read(len));
    }

    fn complete_write(&mut self, id: u64, len: u64) {
        self.finish(id, FS_STATUS_SUCCESS, Complete::Write(len));
    }

    fn complete_none(&mut self, id: u64, status: u64) {
        self.finish(id, status, Complete::None);
    }

    fn finish(&mut self, id: u64, status: u64, data: Complete) {
        let mut cmpl: fs_cmpl_t = unsafe { zeroed() };
        cmpl.id = id;
        cmpl.status = status;
        match data {
            Complete::Fd(fd) => cmpl.data.file_open.fd = fd,
            Complete::Read(len) => cmpl.data.file_read.len_read = len,
            Complete::Write(len) => cmpl.data.file_write.len_written = len,
            Complete::None => {}
        }
        let _ = unsafe { fs_completion_enqueue(self.completion, cmpl) };
    }
}

enum Complete {
    Fd(u64),
    Read(u64),
    Write(u64),
    None,
}

fn share_off(offset: u64, len: usize) -> Option<usize> {
    let off = usize::try_from(offset).ok()?;
    let end = off.checked_add(len)?;
    (end <= SHARE).then_some(off)
}

fn empty_command() -> fs_cmd_t {
    fs_cmd_t {
        id: 0,
        r#type: 0,
        params: lerux_sddf::fs_cmd_params_t { min_size: [0; 48] },
    }
}

fn empty_queue() -> Box<fs_queue_t> {
    let queue = Box::<fs_queue_t>::new_zeroed();
    // SAFETY: an all-zero ring is empty. The fields are integers and atomics.
    unsafe { queue.assume_init() }
}

fn wake(ctx: *mut ()) {
    unsafe { (*ctx.cast::<Fake>()).serve_one() }
}

struct Harness {
    files: Files,
    fake: *mut Fake,
    _fake: Box<Fake>,
    _command: Box<fs_queue_t>,
    _completion: Box<fs_queue_t>,
    _share: Box<[u8; SHARE]>,
}

impl Harness {
    fn new() -> Self {
        let mut command = empty_queue();
        let mut completion = empty_queue();
        let mut share = Box::new([0u8; SHARE]);
        let command_ptr = command.as_mut() as *mut fs_queue_t;
        let completion_ptr = completion.as_mut() as *mut fs_queue_t;
        let share_ptr = share.as_mut_ptr();
        let fake = Box::new(Fake {
            command: command_ptr,
            completion: completion_ptr,
            share: share_ptr,
            files: [None, None, None, None],
            next_server: 1,
            served: 0,
        });
        let fake_ptr = &*fake as *const Fake as *mut Fake;
        let files = unsafe {
            Files::bind(
                command_ptr,
                completion_ptr,
                share_ptr,
                SHARE,
                wake,
                fake_ptr.cast(),
            )
            .expect("share covers the fixed windows")
        };
        Self {
            files,
            fake: fake_ptr,
            _fake: fake,
            _command: command,
            _completion: completion,
            _share: share,
        }
    }

    fn served(&self) -> u64 {
        unsafe { (*self.fake).served }
    }
}

#[test]
fn read_returns_the_written_bytes() {
    let mut harness = Harness::new();
    let files = &mut harness.files;
    let created = files.open(b"SMOKE.TXT", Open::CreateWrite).expect("create");
    assert_eq!(created.number(), 3);
    let denied = files.read(created, &mut [0; 4]);
    assert_eq!(denied, Err(Errno(Errno::EACCES)));
    assert_eq!(files.write(created, PAYLOAD).expect("write"), PAYLOAD.len());
    files.close(created).expect("close");

    let opened = files.open(b"SMOKE.TXT", Open::Read).expect("open");
    let mut buf = [0u8; 32];
    let n = files.read(opened, &mut buf).expect("read");
    assert_eq!(n, 15);
    assert_eq!(&buf[..n], PAYLOAD);
    assert_eq!(files.stat(b"SMOKE.TXT").expect("stat").size, 15);
}

#[test]
fn missing_open_returns_enoent() {
    let mut harness = Harness::new();
    let err = harness.files.open(b"missing", Open::Read).unwrap_err();
    assert_eq!(err, Errno(Errno::ENOENT));
    assert_eq!(err.0, 2);
}

#[test]
fn rename_then_unlink() {
    let mut harness = Harness::new();
    let files = &mut harness.files;
    let fd = files.open(b"a", Open::CreateWrite).expect("create");
    files.write(fd, PAYLOAD).expect("write");
    files.close(fd).expect("close");
    files.rename(b"a", b"b").expect("rename");
    assert_eq!(
        files.open(b"a", Open::Read).unwrap_err(),
        Errno(Errno::ENOENT)
    );
    let moved = files.open(b"b", Open::Read).expect("open moved");
    let mut buf = [0u8; 32];
    let n = files.read(moved, &mut buf).expect("read moved");
    assert_eq!(&buf[..n], PAYLOAD);
    files.close(moved).expect("close moved");
    files.unlink(b"b").expect("unlink");
    assert_eq!(
        files.open(b"b", Open::Read).unwrap_err(),
        Errno(Errno::ENOENT)
    );
}

#[test]
fn mkdir_then_stat() {
    let mut harness = Harness::new();
    harness.files.mkdir(b"box").expect("mkdir");
    assert_eq!(harness.files.stat(b"box").expect("stat").size, 0);
    assert_eq!(
        harness.files.mkdir(b"box").unwrap_err(),
        Errno(Errno::EEXIST)
    );
}

#[test]
fn long_path_sends_nothing() {
    let mut harness = Harness::new();
    let path = vec![b'a'; 4096];
    let err = harness.files.open(&path, Open::Read).unwrap_err();
    assert_eq!(err, Errno(Errno::ENAMETOOLONG));
    assert_eq!(err.0, 36);
    assert_eq!(harness.served(), 0);
}
