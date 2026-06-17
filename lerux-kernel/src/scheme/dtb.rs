//! The `dtb:` scheme: exposing the device tree blob to userspace.
//!
//! On ARM/RISC-V style platforms the firmware hands the kernel a **device tree
//! blob (DTB)** describing the hardware (CPUs, memory, peripherals). This scheme
//! lets userspace drivers read that description so they can discover and
//! configure devices. It is the device-tree analog of the `acpi:` scheme on PCs,
//! and is feature-gated since it only applies to DTB platforms.
//!
//! See also: [`docs/kernel/architecture.md`] sections 7-8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::spin::Once;
use alloc::boxed::Box;

use super::{CallerCtx, HandleMap, KernelScheme, OpenResult, StrOrBytes};
use crate::{
    dtb::DTB_BINARY,
    scheme::InternalFlags,
    sync::{CleanLockToken, RwLock, L1},
    syscall::{
        data::Stat,
        error::*,
        flag::{MODE_FILE, O_STAT},
        usercopy::UserSliceWo,
    },
};

pub struct DtbScheme;

#[derive(Eq, PartialEq)]
enum HandleKind {
    RawData,
    SchemeRoot,
}

struct Handle {
    kind: HandleKind,
    stat: bool,
}

#[allow(uninhabited_static)]
static HANDLES: RwLock<L1, HandleMap<Handle>> = RwLock::new(HandleMap::new());
static DATA: Once<Box<[u8]>> = Once::new();

impl DtbScheme {
    pub fn init() {
        let mut data_init = false;

        DATA.call_once(|| {
            data_init = true;

            Box::from(DTB_BINARY.get().copied().unwrap_or(&[]))
        });

        if !data_init {
            error!("DtbScheme::new called multiple times");
        }
    }
}

impl KernelScheme for DtbScheme {
    fn scheme_root(&self, token: &mut CleanLockToken) -> Result<usize> {
        let id = HANDLES.write(token.token()).insert(Handle {
            kind: HandleKind::SchemeRoot,
            stat: false,
        });
        Ok(id)
    }
    fn kopenat(
        &self,
        id: usize,
        user_buf: StrOrBytes,
        _flags: usize,
        _fcntl_flags: u32,
        _ctx: CallerCtx,
        token: &mut CleanLockToken,
    ) -> Result<OpenResult> {
        if !matches!(
            HANDLES.read(token.token()).get(id)?.kind,
            HandleKind::SchemeRoot
        ) {
            return Err(Error::new(EACCES));
        }

        let path = user_buf
            .as_str()
            .or(Err(Error::new(EINVAL)))?
            .trim_matches('/');

        if path.is_empty() {
            let id = HANDLES.write(token.token()).insert(Handle {
                kind: HandleKind::RawData,
                stat: _flags & O_STAT == O_STAT,
            });
            return Ok(OpenResult::SchemeLocal(id, InternalFlags::POSITIONED));
        }

        Err(Error::new(ENOENT))
    }

    fn fsize(&self, id: usize, token: &mut CleanLockToken) -> Result<u64> {
        let mut handles = HANDLES.write(token.token());
        let handle = handles.get_mut(id)?;

        if handle.stat {
            return Err(Error::new(EBADF));
        }

        let file_len = match handle.kind {
            HandleKind::RawData => DATA.get().ok_or(Error::new(EBADFD))?.len(),
            HandleKind::SchemeRoot => return Err(Error::new(EBADF)),
        };

        Ok(file_len as u64)
    }

    fn close(&self, id: usize, token: &mut CleanLockToken) -> Result<()> {
        HANDLES.write(token.token()).remove(id)?;
        Ok(())
    }

    fn kreadoff(
        &self,
        id: usize,
        dst_buf: UserSliceWo,
        offset: u64,
        _flags: u32,
        _stored_flags: u32,
        token: &mut CleanLockToken,
    ) -> Result<usize> {
        let mut handles = HANDLES.write(token.token());
        let handle = handles.get_mut(id)?;

        if handle.stat {
            return Err(Error::new(EBADF));
        }

        let data = match handle.kind {
            HandleKind::RawData => DATA.get().ok_or(Error::new(EBADFD))?,
            HandleKind::SchemeRoot => return Err(Error::new(EBADF)),
        };

        let src_offset = core::cmp::min(offset.try_into().unwrap(), data.len());
        let src_buf = data
            .get(src_offset..)
            .expect("expected data to be at least data.len() bytes long");

        dst_buf.copy_common_bytes_from_slice(src_buf)
    }

    fn kfpath(&self, _id: usize, buf: UserSliceWo, _token: &mut CleanLockToken) -> Result<usize> {
        //TODO: construct useful path?
        buf.copy_common_bytes_from_slice("/scheme/kernel.dtb/".as_bytes())
    }

    fn kfstat(&self, id: usize, buf: UserSliceWo, token: &mut CleanLockToken) -> Result<()> {
        let handles = HANDLES.read(token.token());
        let handle = handles.get(id)?;
        buf.copy_exactly(&match handle.kind {
            HandleKind::RawData => {
                let data = DATA.get().ok_or(Error::new(EBADFD))?;
                Stat {
                    st_mode: MODE_FILE,
                    st_uid: 0,
                    st_gid: 0,
                    st_size: data.len().try_into().unwrap_or(u64::MAX),
                    ..Default::default()
                }
            }
            HandleKind::SchemeRoot => return Err(Error::new(EBADF)),
        })?;

        Ok(())
    }
}
