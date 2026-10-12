//! Instantiate an ordinary WebAssembly module and call its `start` export.
//!
//! `wasmi` is built without `std`. B-tree collections avoid a random source,
//! which a protection domain does not have. Portable dispatch avoids a tail
//! call on the custom target.

use alloc::{vec, vec::Vec};

use wasmi::{Caller, Engine, Linker, Memory, Module, Store};

use crate::{Error, Imports};

/// Pointer to the caller's host for the duration of [`run`](super::run).
///
/// The executor calls imports on the thread that owns the host. The pointer is
/// not sent anywhere else.
struct HostPtr<H>(*mut H);

// SAFETY: `run` holds `&mut H` for the call, and imports run only on that thread.
unsafe impl<H> Send for HostPtr<H> {}

// SAFETY: same exclusive call as [`Send`]. The store does not share the pointer.
unsafe impl<H> Sync for HostPtr<H> {}

/// Instantiate `wasm` and call `start`.
///
/// `host` is borrowed for this whole function. Imports run synchronously before
/// it returns, so the pointer in the store stays valid.
pub fn run<H: Imports + 'static>(wasm: &[u8], host: &mut H) -> Result<(), Error> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm).map_err(|_| Error::Module)?;
    let mut store = Store::new(&engine, HostPtr(host));
    let mut linker = Linker::new(&engine);
    linker
        .func_wrap("lerux", "log", log_import::<H>)
        .map_err(|_| Error::Module)?;
    linker
        .func_wrap("lerux", "read", read_import::<H>)
        .map_err(|_| Error::Module)?;
    linker
        .func_wrap("lerux", "socket", socket_import::<H>)
        .map_err(|_| Error::Module)?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|_| Error::Module)?;
    let start = instance
        .get_typed_func::<(), ()>(&store, "start")
        .map_err(|_| Error::Export)?;
    start.call(&mut store, ()).map_err(|err| {
        if err.as_trap_code().is_some() {
            Error::Trap
        } else {
            Error::Module
        }
    })?;
    Ok(())
}

fn log_import<H: Imports>(mut caller: Caller<'_, HostPtr<H>>, ptr: i32, len: i32) {
    let Some(bytes) = load_bytes(&caller, ptr, len) else {
        return;
    };
    host(&mut caller).log(&bytes);
}

fn read_import<H: Imports>(
    mut caller: Caller<'_, HostPtr<H>>,
    path_ptr: i32,
    path_len: i32,
    buf_ptr: i32,
    buf_len: i32,
) -> i32 {
    let Some(path) = load_bytes(&caller, path_ptr, path_len) else {
        return -1;
    };
    if buf_ptr < 0 || buf_len < 0 {
        return -1;
    }
    let length = buf_len as usize;
    let mut buf = vec![0; length];
    let n = host(&mut caller).read(&path, &mut buf);
    if n < 0 {
        return n;
    }
    let count = n as usize;
    if count > buf.len() || !store_bytes(&mut caller, buf_ptr, &buf[..count]) {
        return -1;
    }
    n
}

fn socket_import<H: Imports>(mut caller: Caller<'_, HostPtr<H>>) -> i32 {
    host(&mut caller).socket()
}

fn host<'a, H: Imports>(caller: &'a mut Caller<'_, HostPtr<H>>) -> &'a mut H {
    let ptr = caller.data_mut().0;
    // SAFETY: `run` keeps the host borrowed, and this import runs before `run` returns.
    unsafe { &mut *ptr }
}

fn memory_of<T>(caller: &Caller<'_, T>) -> Option<Memory> {
    caller
        .get_export("memory")
        .and_then(wasmi::Extern::into_memory)
}

fn load_bytes<H>(caller: &Caller<'_, HostPtr<H>>, ptr: i32, len: i32) -> Option<Vec<u8>> {
    if ptr < 0 || len < 0 {
        return None;
    }
    let memory = memory_of(caller)?;
    let data = memory.data(caller);
    let start = usize::try_from(ptr).ok()?;
    let length = usize::try_from(len).ok()?;
    let end = start.checked_add(length)?;
    data.get(start..end).map(<[u8]>::to_vec)
}

fn store_bytes<H>(caller: &mut Caller<'_, HostPtr<H>>, ptr: i32, bytes: &[u8]) -> bool {
    if ptr < 0 {
        return false;
    }
    let Some(memory) = memory_of(caller) else {
        return false;
    };
    let data = memory.data_mut(&mut *caller);
    let Ok(start) = usize::try_from(ptr) else {
        return false;
    };
    let Some(end) = start.checked_add(bytes.len()) else {
        return false;
    };
    let Some(slot) = data.get_mut(start..end) else {
        return false;
    };
    slot.copy_from_slice(bytes);
    true
}
