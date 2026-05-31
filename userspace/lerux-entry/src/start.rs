use core::cell::UnsafeCell;

use linked_list_allocator::LockedHeap;
use redox_rt::auxv_defs::{AT_REDOX_NS_FD, AT_REDOX_PROC_FD, AT_REDOX_THR_FD};
use redox_rt::proc::FdGuard;
use redox_rt::{initialize, Tcb};
use syscall::data::GlobalSchemes;
use syscall::flag::O_CLOEXEC;

use crate::stack::{auxv_lookup_at_sp, env_fd_at_sp, env_var, Stack};

mod offsets {
    unsafe extern "C" {
        static __text_start: u8;
        static __text_end: u8;
        static __rodata_start: u8;
        static __rodata_end: u8;
        static __data_start: u8;
        static __bss_end: u8;
    }
    pub fn text() -> (usize, usize) {
        unsafe {
            (
                &__text_start as *const u8 as usize,
                &__text_end as *const u8 as usize,
            )
        }
    }
    pub fn rodata() -> (usize, usize) {
        unsafe {
            (
                &__rodata_start as *const u8 as usize,
                &__rodata_end as *const u8 as usize,
            )
        }
    }
    pub fn data_and_bss() -> (usize, usize) {
        unsafe {
            (
                &__data_start as *const u8 as usize,
                &__bss_end as *const u8 as usize,
            )
        }
    }
}

const HEAP_SIZE: usize = 1024 * 1024;

struct AllocState(UnsafeCell<Option<LockedHeap>>);
unsafe impl Sync for AllocState {}
static ALLOC_STATE: AllocState = AllocState(UnsafeCell::new(None));

struct LeruxAllocator;
unsafe impl core::alloc::GlobalAlloc for LeruxAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        heap().lock().allocate_first_fit(layout).ok().map_or(ptr::null_mut(), |n| n.as_ptr())
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if !ptr.is_null() {
            heap().lock().deallocate(core::ptr::NonNull::new_unchecked(ptr), layout)
        }
    }
}
use core::ptr;
#[global_allocator]
static GLOBAL: LeruxAllocator = LeruxAllocator;

fn heap() -> &'static LockedHeap {
    unsafe {
        let state = &mut *ALLOC_STATE.0.get();
        state.get_or_insert_with(|| {
            let base = syscall::fmap(
                !0,
                &syscall::Map {
                    offset: 0,
                    size: HEAP_SIZE,
                    flags: syscall::MapFlags::PROT_READ
                        | syscall::MapFlags::PROT_WRITE
                        | syscall::MapFlags::MAP_PRIVATE,
                    address: 0,
                },
            )
            .expect("lerux-entry: heap map");
            let mut h = LockedHeap::empty();
            h.lock().init(base as *mut u8, HEAP_SIZE);
            h
        })
    }
}

unsafe extern "C" {
    fn lerux_rt_main() -> !;
}

fn parse_fd_env(stack: &Stack, key: &str) -> Option<usize> {
    let val = env_var(stack, key)?;
    core::str::from_utf8(val).ok()?.parse().ok()
}

fn upper_fd(raw: usize) -> Result<redox_rt::proc::FdGuardUpper, ()> {
    FdGuard::new(raw).to_upper().map_err(|_| ())
}

/// After `fexec`, auxv/env parsing can fail if the stack layout differs; reopen init via proc.
fn thr_fd_from_proc_env(sp: usize) -> Option<usize> {
    let proc = env_fd_at_sp(sp, "__LERUX_PROC_FD")?;
    let upper = upper_fd(proc).ok()?;
    let thr = upper.dup(b"thread-0").ok()?;
    upper_fd(thr.as_raw_fd()).ok().map(|f| f.as_raw_fd())
}

fn thr_fd_from_proc_scheme() -> Option<usize> {
    let proc_scheme = syscall::UPPER_FDTBL_TAG + GlobalSchemes::Proc as usize;
    let init = syscall::openat(proc_scheme, "init", O_CLOEXEC, 0).ok()?;
    FdGuard::new(init)
        .dup(b"thread-0")
        .ok()
        .map(|fd| fd.as_raw_fd())
}

fn proc_fd_from_proc_scheme() -> Option<usize> {
    let proc_scheme = syscall::UPPER_FDTBL_TAG + GlobalSchemes::Proc as usize;
    syscall::openat(proc_scheme, "init", O_CLOEXEC, 0).ok()
}

/// Fallback when `envp`/`auxv` layout does not match expectations (still on the exec stack).
fn scan_stack_pairs(sp: usize, typ: usize) -> Option<usize> {
    const WORDS: usize = 256;
    for i in 0..WORDS {
        unsafe {
            let a = *((sp as *const usize).add(i));
            if i + 1 >= WORDS {
                break;
            }
            let b = *((sp as *const usize).add(i + 1));
            if a == 0 && b == 0 {
                break;
            }
            if a == typ {
                return Some(b);
            }
            if b == typ {
                return Some(a);
            }
        }
    }
    None
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lerux_entry_start() -> ! {
    unsafe {
        let sp: usize;
        core::arch::asm!("mov {}, rsp", out(reg) sp);
        let stack = Stack::from_sp(sp);

        let thr_fd = {
            let raw = thr_fd_from_proc_scheme()
                .or_else(|| env_fd_at_sp(sp, "__LERUX_THR_FD"))
                .or_else(|| thr_fd_from_proc_env(sp))
                .or_else(|| auxv_lookup_at_sp(sp, AT_REDOX_THR_FD));
            match raw {
                Some(raw) => upper_fd(raw).unwrap_or_else(|_| {
                    syscall::write(1, b"lerux-entry: to_upper thr_fd failed\n").ok();
                    core::arch::asm!("ud2", options(noreturn));
                }),
                None => {
                    syscall::write(1, b"lerux-entry: no thread fd\n").ok();
                    core::arch::asm!("ud2", options(noreturn));
                }
            }
        };

        // Minimal TLS (same approach as initialize_freestanding).
        let page = &mut *(syscall::fmap(
            !0,
            &syscall::Map {
                offset: 0,
                size: syscall::PAGE_SIZE,
                flags: syscall::MapFlags::PROT_READ
                    | syscall::MapFlags::PROT_WRITE
                    | syscall::MapFlags::MAP_PRIVATE,
                address: 0,
            },
        )
        .expect("lerux-entry: TLS map") as *mut Tcb);
        page.tcb_ptr = page;
        page.tcb_len = syscall::PAGE_SIZE;
        page.tls_end = page as *mut Tcb as *mut u8;
        page.os_specific.thr_fd.get().write(Some(thr_fd));
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            let tcb_addr = page as *mut Tcb as usize;
            redox_rt::tcb_activate(&page.os_specific, tcb_addr, 0);
        }

        let proc_fd = Stack::lerux_fds_at_sp(sp)
            .or_else(|| stack.lerux_fds())
            .map(|(_, proc, _)| proc)
            .or_else(|| env_fd_at_sp(sp, "__LERUX_PROC_FD"))
            .or_else(|| parse_fd_env(stack, "__LERUX_PROC_FD"))
            .or_else(|| auxv_lookup_at_sp(sp, AT_REDOX_PROC_FD))
            .or_else(|| scan_stack_pairs(sp, AT_REDOX_PROC_FD))
            .or_else(proc_fd_from_proc_scheme)
            .and_then(|fd| upper_fd(fd).ok());
        let ns_fd = Stack::lerux_fds_at_sp(sp)
            .or_else(|| stack.lerux_fds())
            .map(|(_, _, ns)| ns)
            .or_else(|| env_fd_at_sp(sp, "__LERUX_NS_FD"))
            .or_else(|| parse_fd_env(stack, "__LERUX_NS_FD"))
            .or_else(|| auxv_lookup_at_sp(sp, AT_REDOX_NS_FD))
            .or_else(|| scan_stack_pairs(sp, AT_REDOX_NS_FD))
            .filter(|&fd| fd != usize::MAX)
            .and_then(|fd| upper_fd(fd).ok());
        let _ = heap();

        // fexec already mapped the executable with correct permissions.
        STACK = stack;
        lerux_rt_main()
    }
}

static mut STACK: &'static Stack = &Stack {
    argc: 0,
    argv0: core::ptr::null(),
};

pub fn stack() -> &'static Stack {
    unsafe { STACK }
}
