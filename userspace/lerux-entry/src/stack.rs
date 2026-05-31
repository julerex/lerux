//! Initial stack layout after exec (argc, argv, envp, auxv).

use core::ptr;

use redox_rt::auxv_defs::AT_REDOX_THR_FD;

#[repr(C)]
pub struct Stack {
    pub argc: isize,
    pub argv0: *const u8,
}

impl Stack {
    pub unsafe fn from_sp(sp: usize) -> &'static Self {
        &*(sp as *const Self)
    }

    pub fn argv(&self) -> *const *const u8 {
        ptr::from_ref(&self.argv0)
    }

    pub fn envp(&self) -> *const *const u8 {
        unsafe { self.argv().offset(self.argc + 1) }
    }

    pub fn thread_fd(&self) -> usize {
        auxv_lookup(self, AT_REDOX_THR_FD).expect("AT_REDOX_THR_FD missing from auxv")
    }

    pub fn arg(&self, n: usize) -> Option<&[u8]> {
        if n >= self.argc as usize {
            return None;
        }
        let ptr = unsafe { *self.argv().add(n) };
        Some(unsafe { core::slice::from_raw_parts(ptr, cstr_len(ptr)) })
    }

    /// `argv[1]` from bootstrap: `"thr_fd,proc_fd,ns_fd"`.
    pub fn lerux_fds(&self) -> Option<(usize, usize, usize)> {
        self.arg(1).and_then(parse_fd_triple)
    }

    /// Read `argv[1]` even when `argc` is wrong (bootstrap passes fds as the second arg).
    pub fn lerux_fds_at_sp(sp: usize) -> Option<(usize, usize, usize)> {
        let ptr = unsafe { *((sp as *const usize).add(2)) };
        if ptr < 0x1000 {
            return None;
        }
        let s = unsafe { core::slice::from_raw_parts(ptr as *const u8, cstr_len(ptr as *const u8)) };
        parse_fd_triple(s)
    }
}

fn parse_fd_triple(s: &[u8]) -> Option<(usize, usize, usize)> {
    let s = core::str::from_utf8(s).ok()?;
    let mut it = s.split(',');
    let thr = it.next()?.parse().ok()?;
    let proc = it.next()?.parse().ok()?;
    let ns = it.next()?.parse().ok()?;
    Some((thr, proc, ns))
}

fn cstr_len(ptr: *const u8) -> usize {
    unsafe {
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        len
    }
}

/// Value bytes without the terminating NUL (env strings are `KEY=value\0`).
pub fn env_var<'a>(stack: &'a Stack, key: &str) -> Option<&'a [u8]> {
    let mut envp = stack.envp();
    loop {
        let entry = unsafe { *envp };
        if entry.is_null() {
            return None;
        }
        let s = unsafe { core::slice::from_raw_parts(entry, cstr_len(entry)) };
        if let Some(eq) = s.iter().position(|&b| b == b'=') {
            if &s[..eq] == key.as_bytes() {
                return Some(&s[eq + 1..]);
            }
        }
        envp = unsafe { envp.add(1) };
    }
}

pub fn env_fd_at_sp(sp: usize, key: &str) -> Option<usize> {
    let mut i = 1usize;
    while unsafe { *((sp as *const usize).add(i)) } != 0 {
        i += 1;
    }
    i += 1;
    while unsafe { *((sp as *const usize).add(i)) } != 0 {
        let ptr = unsafe { *((sp as *const usize).add(i)) };
        if ptr >= 0x1000 {
            let entry =
                unsafe { core::slice::from_raw_parts(ptr as *const u8, cstr_len(ptr as *const u8)) };
            if let Some(eq) = entry.iter().position(|&b| b == b'=') {
                if &entry[..eq] == key.as_bytes() {
                    return core::str::from_utf8(&entry[eq + 1..])
                        .ok()?
                        .parse()
                        .ok();
                }
            }
        }
        i += 1;
    }
    None
}

fn auxv_start_index(sp: usize) -> usize {
    let mut i = 1usize;
    while unsafe { *((sp as *const usize).add(i)) } != 0 {
        i += 1;
    }
    i += 1;
    while unsafe { *((sp as *const usize).add(i)) } != 0 {
        i += 1;
    }
    i + 1
}

/// Walk auxv by index from `sp` (does not trust `argc`).
pub fn auxv_lookup_at_sp(sp: usize, typ: usize) -> Option<usize> {
    let mut i = auxv_start_index(sp);
    loop {
        let a = unsafe { *((sp as *const usize).add(i)) };
        let b = unsafe { *((sp as *const usize).add(i + 1)) };
        if a == 0 && b == 0 {
            return None;
        }
        if a == typ {
            return Some(b);
        }
        if b == typ {
            return Some(a);
        }
        i += 2;
    }
}

/// Walk the aux vector after `envp`. Accept either `(tag, value)` or `(value, tag)` pairs.
pub fn auxv_lookup(stack: &Stack, typ: usize) -> Option<usize> {
    let sp = stack as *const Stack as usize;
    auxv_lookup_at_sp(sp, typ)
}
