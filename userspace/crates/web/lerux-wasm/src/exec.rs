//! Instantiate an ordinary WebAssembly module and call `start` or Content `render`.
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
///
/// # Errors
///
/// Returns [`Error::Module`] when `wasm` is not a module, an import cannot be
/// linked, or `start` fails without a trap. Returns [`Error::Export`] when
/// `start` is missing. Returns [`Error::Trap`] when `start` traps.
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
    let length = usize::try_from(buf_len).expect("buf_len is non-negative");
    let mut buf = vec![0; length];
    let n = host(&mut caller).read(&path, &mut buf);
    if n < 0 {
        return n;
    }
    let count = usize::try_from(n).expect("read count is non-negative");
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

/// Instantiate `wasm` and call Content `render(input_len) -> i64`.
///
/// The module has no `lerux` imports. On success, `output` receives the
/// returned range and the result is that byte count. Bit 63 of the `i64`
/// is a rejection: the host copies nothing.
///
/// # Errors
///
/// Returns [`Error::Module`] when `wasm` is not a module this engine can
/// instantiate, or the returned range is outside linear memory. Returns
/// [`Error::Export`] when `render` or the required input getter is missing.
/// Returns [`Error::Trap`] when `render` traps, the input is longer than the
/// declared capacity, or a UTF-8 input is not UTF-8. Returns
/// [`Error::Rejected`] when `render` sets the rejection bit. Returns
/// [`Error::Truncated`] when `input` does not fit in an `i32` or `output`
/// is shorter than the rendered bytes.
pub fn render(wasm: &[u8], input: &[u8], output: &mut [u8]) -> Result<usize, Error> {
    let input_len = i32::try_from(input.len()).map_err(|_| Error::Truncated)?;
    let engine = Engine::default();
    let module = Module::new(&engine, wasm).map_err(|_| Error::Module)?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|_| Error::Module)?;
    let memory = instance.get_memory(&store, "memory").ok_or(Error::Module)?;
    write_input(instance, &mut store, memory, input)?;
    let packed = call_render(instance, &mut store, input_len)?;
    copy_output(&store, memory, packed, output)
}

fn write_input(
    instance: wasmi::Instance,
    store: &mut Store<()>,
    memory: Memory,
    input: &[u8],
) -> Result<(), Error> {
    let utf8 = export_i32(instance, store, "input_utf8_cap")?;
    let bytes = export_i32(instance, store, "input_bytes_cap")?;
    let Some(cap) = one_cap(utf8, bytes, input)? else {
        return Ok(());
    };
    fits(cap, input.len())?;
    if input.is_empty() {
        return Ok(());
    }
    let ptr = export_i32(instance, store, "input_ptr")?.ok_or(Error::Export)?;
    copy_at(store, memory, ptr, input)
}

fn one_cap(utf8: Option<i32>, bytes: Option<i32>, input: &[u8]) -> Result<Option<i32>, Error> {
    match (utf8, bytes) {
        (Some(cap), None) => {
            if core::str::from_utf8(input).is_err() {
                return Err(Error::Trap);
            }
            Ok(Some(cap))
        }
        (None, Some(cap)) => Ok(Some(cap)),
        (None, None) if input.is_empty() => Ok(None),
        _ => Err(Error::Export),
    }
}

fn fits(cap: i32, len: usize) -> Result<(), Error> {
    let cap = usize::try_from(cap).map_err(|_| Error::Module)?;
    if len > cap {
        Err(Error::Trap)
    } else {
        Ok(())
    }
}

fn export_i32(
    instance: wasmi::Instance,
    store: &mut Store<()>,
    name: &str,
) -> Result<Option<i32>, Error> {
    let Ok(func) = instance.get_typed_func::<(), i32>(&*store, name) else {
        return Ok(None);
    };
    func.call(store, ())
        .map(Some)
        .map_err(|err| trap_or_module(&err))
}

fn copy_at(store: &mut Store<()>, memory: Memory, ptr: i32, bytes: &[u8]) -> Result<(), Error> {
    let start = usize::try_from(ptr).map_err(|_| Error::Module)?;
    let end = start.checked_add(bytes.len()).ok_or(Error::Module)?;
    let slot = memory
        .data_mut(store)
        .get_mut(start..end)
        .ok_or(Error::Module)?;
    slot.copy_from_slice(bytes);
    Ok(())
}

fn call_render(
    instance: wasmi::Instance,
    store: &mut Store<()>,
    input_len: i32,
) -> Result<i64, Error> {
    let func = instance
        .get_typed_func::<i32, i64>(&*store, "render")
        .map_err(|_| Error::Export)?;
    func.call(store, input_len)
        .map_err(|err| trap_or_module(&err))
}

fn copy_output(
    store: &Store<()>,
    memory: Memory,
    packed: i64,
    output: &mut [u8],
) -> Result<usize, Error> {
    let bits = u64::try_from(packed).map_err(|_| Error::Rejected)?;
    let ptr = usize::try_from(bits >> 32).map_err(|_| Error::Module)?;
    let len = usize::try_from(bits & 0xffff_ffff).map_err(|_| Error::Module)?;
    if len > output.len() {
        return Err(Error::Truncated);
    }
    let end = ptr.checked_add(len).ok_or(Error::Module)?;
    let src = memory.data(store).get(ptr..end).ok_or(Error::Module)?;
    output[..len].copy_from_slice(src);
    Ok(len)
}

fn trap_or_module(err: &wasmi::Error) -> Error {
    if err.as_trap_code().is_some() {
        Error::Trap
    } else {
        Error::Module
    }
}
