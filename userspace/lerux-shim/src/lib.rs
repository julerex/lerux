//! `redox_*_v1` symbols expected by libredox with the `call` feature (no relibc).
#![no_std]
#![allow(improper_ctypes_definitions, unsafe_op_in_unsafe_fn)]

extern crate alloc;

use alloc::vec::Vec;
use core::slice;
use core::str;

use ioslice::IoSlice;
use redox_protocols::protocol::{ProcKillTarget, SocketCall, WaitFlags};
use redox_rt::sys::{WaitpidTarget, posix_read, posix_write};
use syscall;
use syscall::data::{Stat, StatVfs, TimeSpec};
use syscall::{Error, Result, EINVAL};

type RawResult = usize;

fn path_from_raw(base: *const u8, len: usize) -> &'static str {
    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(base, len)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_open_v1(
    path_base: *const u8,
    path_len: usize,
    flags: u32,
    mode: u16,
) -> RawResult {
    Error::mux((|| {
        let ns = redox_rt::sys::getns()?;
        syscall::openat(
            ns,
            path_from_raw(path_base, path_len),
            flags as usize,
            mode as usize,
        )
    })())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_openat_v1(
    fd: usize,
    path_base: *const u8,
    path_len: usize,
    flags: u32,
    fcntl_flags: u32,
) -> RawResult {
    Error::mux(syscall::openat(
        fd,
        path_from_raw(path_base, path_len),
        flags as usize,
        fcntl_flags as usize,
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_dup_v1(fd: usize, buf: *const u8, len: usize) -> RawResult {
    Error::mux(syscall::dup(fd, slice::from_raw_parts(buf, len)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_dup2_v1(
    old_fd: usize,
    new_fd: usize,
    buf: *const u8,
    len: usize,
) -> RawResult {
    Error::mux(syscall::dup2(old_fd, new_fd, slice::from_raw_parts(buf, len)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_read_v1(fd: usize, dst_base: *mut u8, dst_len: usize) -> RawResult {
    Error::mux(posix_read(fd, slice::from_raw_parts_mut(dst_base, dst_len)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_write_v1(
    fd: usize,
    src_base: *const u8,
    src_len: usize,
) -> RawResult {
    Error::mux(posix_write(fd, slice::from_raw_parts(src_base, src_len)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fchmod_v1(fd: usize, new_mode: u16) -> RawResult {
    Error::mux(syscall::fchmod(fd, new_mode).map(|_| 0))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fchown_v1(fd: usize, new_uid: u32, new_gid: u32) -> RawResult {
    Error::mux(syscall::fchown(fd, new_uid, new_gid).map(|_| 0))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_getdents_v0(
    _fd: usize,
    _buf: *mut u8,
    _buf_len: usize,
    _opaque: u64,
) -> RawResult {
    Error::mux(Err(Error::new(syscall::ENOSYS)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fstat_v1(fd: usize, dst: *mut Stat) -> RawResult {
    Error::mux(syscall::fstat(fd, &mut *dst))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fstatvfs_v1(fd: usize, dst: *mut StatVfs) -> RawResult {
    Error::mux(syscall::fstatvfs(fd, &mut *dst))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fsync_v1(fd: usize) -> RawResult {
    Error::mux(syscall::fsync(fd))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fdatasync_v1(fd: usize) -> RawResult {
    Error::mux(syscall::fsync(fd))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_ftruncate_v0(fd: usize, len: usize) -> RawResult {
    Error::mux(syscall::ftruncate(fd, len))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_futimens_v1(fd: usize, times: *const TimeSpec) -> RawResult {
    Error::mux(syscall::futimens(fd, slice::from_ref(&*times)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_fpath_v1(fd: usize, dst_base: *mut u8, dst_len: usize) -> RawResult {
    Error::mux(syscall::fpath(
        fd,
        slice::from_raw_parts_mut(dst_base, dst_len),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_relpathat_v0(
    _dirfd: usize,
    _fd: usize,
    _dst_base: *mut u8,
    _dst_len: usize,
) -> RawResult {
    Error::mux(Err(Error::new(syscall::ENOSYS)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_close_v1(fd: usize) -> RawResult {
    Error::mux(syscall::close(fd))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_pid_v1() -> RawResult {
    redox_rt::sys::posix_getpid() as _
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_euid_v1() -> RawResult {
    redox_rt::sys::posix_getresugid().euid as _
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_ruid_v1() -> RawResult {
    redox_rt::sys::posix_getresugid().ruid as _
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_egid_v1() -> RawResult {
    redox_rt::sys::posix_getresugid().egid as _
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_rgid_v1() -> RawResult {
    redox_rt::sys::posix_getresugid().rgid as _
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_ens_v0() -> RawResult {
    Error::mux(redox_rt::sys::getens())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_ns_v0() -> RawResult {
    Error::mux(redox_rt::sys::getns())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_proc_credentials_v1(
    cap_fd: usize,
    target_pid: usize,
    buf: &mut [u8],
) -> RawResult {
    Error::mux(redox_rt::sys::get_proc_credentials(cap_fd, target_pid, buf))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_setrens_v1(rns: usize, ens: usize) -> RawResult {
    let _ = rns;
    if ens == 0 {
        let null_namespace: [IoSlice; 2] = [IoSlice::new(b"memory"), IoSlice::new(b"pipe")];
        match redox_rt::sys::mkns(&null_namespace) {
            Ok(new_ns_fd) => {
                if redox_rt::sys::setns(new_ns_fd.take()).is_none() {
                    return Error::mux(Err(Error::new(syscall::EIO)));
                }
            }
            Err(e) => return Error::mux(Err(e)),
        }
    } else if redox_rt::sys::setns(ens).is_none() {
        return Error::mux(Err(Error::new(syscall::EIO)));
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_waitpid_v1(pid: usize, status: *mut i32, options: u32) -> RawResult {
    let mut sts = 0_usize;
    let res = Error::mux(redox_rt::sys::sys_waitpid(
        WaitpidTarget::from_posix_arg(pid as isize),
        &mut sts,
        WaitFlags::from_bits_truncate(options as usize),
    ));
    status.write(sts as i32);
    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_kill_v1(pid: usize, signal: u32) -> RawResult {
    Error::mux(
        redox_rt::sys::posix_kill(ProcKillTarget::from_raw(pid), signal as usize).map(|_| 0),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_sigprocmask_v1(
    _how: u32,
    _new: *const u64,
    _old: *mut u64,
) -> RawResult {
    Error::mux(Err(Error::new(syscall::ENOSYS)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_sigaction_v1(
    _signal: u32,
    _new: *const (),
    _old: *mut (),
) -> RawResult {
    Error::mux(Err(Error::new(syscall::ENOSYS)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_clock_gettime_v1(clock: usize, ts: *mut TimeSpec) -> RawResult {
    Error::mux(syscall::clock_gettime(clock, &mut *ts).map(|_| 0))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_mmap_v1(
    addr: *mut (),
    unaligned_len: usize,
    prot: u32,
    flags: u32,
    fd: usize,
    offset: u64,
) -> RawResult {
    Error::mux(syscall::fmap(
        fd,
        &syscall::Map {
            address: addr as usize,
            offset: offset as usize,
            size: unaligned_len,
            flags: syscall::MapFlags::from_bits_truncate(
                ((prot as usize) << 16) | (flags as usize & 0xffff),
            ),
        },
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_munmap_v1(addr: *mut (), unaligned_len: usize) -> RawResult {
    Error::mux(syscall::funmap(addr as usize, unaligned_len))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_strerror_v1(
    buf: *mut u8,
    buflen: *mut usize,
    error: u32,
) -> RawResult {
    let dst = slice::from_raw_parts_mut(buf, buflen.read());
    Error::mux((|| {
        let src = syscall::error::STR_ERROR
            .get(error as usize)
            .ok_or(Error::new(EINVAL))?;
        buflen.write(src.len());
        let raw_len = core::cmp::min(dst.len(), src.len());
        let len = core::str::from_utf8(&src.as_bytes()[..raw_len])
            .map(|s| s.len())
            .unwrap_or_else(|e| e.valid_up_to());
        dst[..len].copy_from_slice(&src.as_bytes()[..len]);
        Ok(len)
    })())
}

#[repr(C)]
struct libc_iovec {
    iov_base: *mut u8,
    iov_len: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_mkns_v1(
    names: *const libc_iovec,
    num_names: usize,
    flags: u32,
) -> RawResult {
    Error::mux((|| {
        if flags != 0 {
            return Err(Error::new(EINVAL));
        }
        let raw = slice::from_raw_parts(names, num_names);
        let names_ioslice: Vec<IoSlice> = raw
            .iter()
            .map(|iov| {
                IoSlice::new(slice::from_raw_parts(
                    iov.iov_base as *const u8,
                    iov.iov_len,
                ))
            })
            .collect();
        redox_rt::sys::mkns(&names_ioslice).map(|fd| fd.take())
    })())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_sys_call_v0(
    fd: usize,
    payload: *mut u8,
    payload_len: usize,
    flags: usize,
    metadata: *const u64,
    metadata_len: usize,
) -> RawResult {
    Error::mux(redox_rt::sys::sys_call(
        fd,
        slice::from_raw_parts_mut(payload, payload_len),
        syscall::CallFlags::from_bits_retain(flags),
        slice::from_raw_parts(metadata, metadata_len),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_get_socket_token_v0(
    fd: usize,
    payload: *mut u8,
    payload_len: usize,
) -> RawResult {
    let metadata = [SocketCall::GetToken as u64];
    Error::mux(redox_rt::sys::sys_call_ro(
        fd,
        slice::from_raw_parts_mut(payload, payload_len),
        syscall::CallFlags::empty(),
        &metadata,
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_setns_v0(fd: usize) -> RawResult {
    match redox_rt::sys::setns(fd) {
        Some(guard) => guard.take(),
        None => usize::MAX,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_register_scheme_to_ns_v0(
    ns_fd: usize,
    name_base: *const u8,
    name_len: usize,
    cap_fd: usize,
) -> RawResult {
    Error::mux(
        redox_rt::sys::register_scheme_to_ns(
            ns_fd,
            path_from_raw(name_base, name_len),
            cap_fd,
        )
        .map(|_| 0),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_cur_thrfd_v0() -> usize {
    redox_rt::RtTcb::current().thread_fd().as_raw_fd()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn redox_cur_procfd_v0() -> usize {
    redox_rt::current_proc_fd().as_raw_fd()
}

/// Keep libredox `extern "C"` symbols in the final link.
#[used]
static LERUX_SHIM_LINK: unsafe extern "C" fn(*const u8, usize, u32, u16) -> usize = redox_open_v1;
