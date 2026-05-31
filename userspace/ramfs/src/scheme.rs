use alloc::string::String;
use alloc::vec::Vec;

use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult};
use scheme_utils::FpathWriter;
use syscall::data::Stat;
use syscall::schemev2::NewFdFlags;
use syscall::{Error, Result, EBADF, MODE_CHR};

pub struct RamfsScheme {
    name: String,
}

impl RamfsScheme {
    pub fn new(name: &str) -> Self {
        RamfsScheme {
            name: String::from(name),
        }
    }
}

impl SchemeSync for RamfsScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        Ok(0)
    }

    fn openat(
        &mut self,
        _dirfd: usize,
        path: &str,
        _flags: usize,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<OpenResult> {
        let _ = path;
        Ok(OpenResult::ThisScheme {
            number: 0,
            flags: NewFdFlags::empty(),
        })
    }

    fn read(
        &mut self,
        _id: usize,
        buf: &mut [u8],
        _offset: u64,
        _flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        buf.fill(0);
        Ok(buf.len())
    }

    fn write(
        &mut self,
        _id: usize,
        buf: &[u8],
        _offset: u64,
        _flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        let _ = buf;
        Ok(buf.len())
    }

    fn fpath(&mut self, _id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        FpathWriter::with(buf, &self.name, |_| Ok(()))
    }

    fn fstat(&mut self, _id: usize, stat: &mut Stat, _ctx: &CallerCtx) -> Result<()> {
        stat.st_mode = MODE_CHR | 0o644;
        Ok(())
    }

    fn on_close(&mut self, _id: usize) {}
}
