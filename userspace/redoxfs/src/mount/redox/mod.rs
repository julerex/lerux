use redox_scheme::{
    scheme::{SchemeState, SchemeSync},
    RequestKind, Response, SignalBehavior, Socket,
};
use std::io;
use std::path::Path;
use std::sync::atomic::Ordering;

use crate::{Disk, FileSystem, IS_UMT};

use self::scheme::FileScheme;

pub mod resource;
pub mod scheme;

fn serve_sync_scheme<'sock, D: Disk>(
    socket: &'sock Socket,
    scheme: &mut FileScheme<'sock, D>,
    state: &mut SchemeState,
) -> io::Result<()> {
    while IS_UMT.load(Ordering::SeqCst) == 0 {
        let req = match socket.next_request(SignalBehavior::Restart)? {
            None => break,
            Some(req) => {
                match req.kind() {
                    RequestKind::Call(r) => r,
                    RequestKind::SendFd(sendfd_request) => {
                        let result = scheme.on_sendfd(&sendfd_request);
                        let response = Response::new(result, sendfd_request);

                        if !socket.write_response(response, SignalBehavior::Restart)? {
                            break;
                        }
                        continue;
                    }
                    RequestKind::OnClose { id } => {
                        scheme.on_close(id);
                        state.on_close(id);
                        continue;
                    }
                    RequestKind::OnDetach { id, pid } => {
                        let Ok(inode) = scheme.inode(id) else {
                            log::warn!("RequestKind::OnDetach with invalid `id`");
                            continue;
                        };
                        state.on_detach(id, inode, pid);
                        continue;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        };
        let response = req.handle_sync(scheme, state);

        if !socket.write_response(response, SignalBehavior::Restart)? {
            break;
        }
    }

    scheme.fs.cleanup()?;
    Ok(())
}

//FIXME: mut callback is not mut
#[allow(unused_mut)]

pub fn mount<D, P, T, F>(filesystem: FileSystem<D>, mountpoint: P, mut callback: F) -> io::Result<T>
where
    D: Disk,
    P: AsRef<Path>,
    F: FnOnce(&Path) -> T,
{
    let mountpoint = mountpoint.as_ref();
    let socket = Socket::create()?;

    let scheme_name = format!("{}", mountpoint.display());
    let mounted_path = format!("/scheme/{}", mountpoint.display());

    let mut state = SchemeState::new();
    let mut scheme = FileScheme::new(scheme_name, mounted_path.clone(), filesystem, &socket)?;

    redox_scheme::scheme::register_sync_scheme(
        &socket,
        &format!("{}", mountpoint.display()),
        &mut scheme,
    )?;

    let res = callback(Path::new(&mounted_path));

    serve_sync_scheme(&socket, &mut scheme, &mut state)?;

    Ok(res)
}

/// Mount via init's scheme registration (INIT_NOTIFY + register_scheme_to_ns).
#[cfg(target_os = "redox")]
pub fn mount_via_init<D, P, F>(
    filesystem: FileSystem<D>,
    mountpoint: P,
    scheme_daemon: daemon::SchemeDaemon,
    callback: F,
) -> io::Result<()>
where
    D: Disk,
    P: AsRef<Path>,
    F: FnOnce(&Path),
{
    let mountpoint = mountpoint.as_ref();
    let socket = Socket::create()?;

    let scheme_name = format!("{}", mountpoint.display());
    let mounted_path = format!("/scheme/{}", mountpoint.display());

    let mut state = SchemeState::new();
    let mut scheme = FileScheme::new(scheme_name, mounted_path.clone(), filesystem, &socket)?;

    scheme_daemon
        .ready_sync_scheme(&socket, &mut scheme)
        .map_err(|err| io::Error::from_raw_os_error(err.errno))?;

    callback(Path::new(&mounted_path));

    serve_sync_scheme(&socket, &mut scheme, &mut state)
}
