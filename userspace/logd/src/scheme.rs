use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use libredox::Fd;
use libredox::flag::{O_CREAT, O_WRONLY};
use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult, SendFdRequest};
use scheme_utils::{FpathWriter, HandleMap};
use syscall::error::*;
use syscall::schemev2::NewFdFlags;

pub enum LogHandle {
    Log { context: String },
    AddSink,
    SchemeRoot,
}

pub struct LogScheme {
    kernel_debug: Fd,
    sink_fds: Vec<Fd>,
    backlog: VecDeque<Vec<u8>>,
    line_bufs: BTreeMap<usize, Vec<u8>>,
    handles: HandleMap<LogHandle>,
}

impl LogScheme {
    pub fn new() -> Self {
        let kernel_debug =
            Fd::open("/scheme/debug", O_WRONLY | O_CREAT, 0).expect("logd: debug scheme");
        LogScheme {
            kernel_debug,
            sink_fds: Vec::new(),
            backlog: VecDeque::new(),
            line_bufs: BTreeMap::new(),
            handles: HandleMap::new(),
        }
    }

    fn flush_line(
        kernel_debug: &mut Fd,
        sink_fds: &mut Vec<Fd>,
        backlog: &mut VecDeque<Vec<u8>>,
        line: &[u8],
    ) {
        let _ = kernel_debug.write(line);
        for sink in sink_fds.iter_mut() {
            let _ = sink.write(line);
        }
        backlog.push_back(line.to_vec());
        while backlog.len() > 1000 {
            backlog.pop_front();
        }
    }

    fn write_logs(
        kernel_debug: &mut Fd,
        sink_fds: &mut Vec<Fd>,
        backlog: &mut VecDeque<Vec<u8>>,
        handle_buf: &mut Vec<u8>,
        context: &str,
        buf: &[u8],
    ) {
        let mut i = 0;
        while i < buf.len() {
            if handle_buf.is_empty() && !context.is_empty() {
                handle_buf.extend_from_slice(context.as_bytes());
                handle_buf.extend_from_slice(b": ");
            }
            handle_buf.push(buf[i]);
            if buf[i] == b'\n' {
                Self::flush_line(kernel_debug, sink_fds, backlog, handle_buf);
                handle_buf.clear();
            }
            i += 1;
        }
    }
}

impl SchemeSync for LogScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        Ok(self.handles.insert(LogHandle::SchemeRoot))
    }

    fn openat(
        &mut self,
        dirfd: usize,
        path: &str,
        _flags: usize,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<OpenResult> {
        if !matches!(self.handles.get(dirfd)?, LogHandle::SchemeRoot) {
            return Err(Error::new(EACCES));
        }
        let id = if path == "add_sink" {
            self.handles.insert(LogHandle::AddSink)
        } else {
            self.handles.insert(LogHandle::Log {
                context: path.to_string(),
            })
        };
        Ok(OpenResult::ThisScheme {
            number: id,
            flags: NewFdFlags::empty(),
        })
    }

    fn read(
        &mut self,
        _id: usize,
        _buf: &mut [u8],
        _offset: u64,
        _flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        Ok(0)
    }

    fn write(
        &mut self,
        id: usize,
        buf: &[u8],
        _offset: u64,
        _flags: u32,
        ctx: &CallerCtx,
    ) -> Result<usize> {
        let context = match self.handles.get(id)? {
            LogHandle::Log { context } => context.clone(),
            LogHandle::SchemeRoot | LogHandle::AddSink => return Err(Error::new(EBADF)),
        };
        let handle_buf = self.line_bufs.entry(ctx.pid).or_insert_with(Vec::new);
        Self::write_logs(
            &mut self.kernel_debug,
            &mut self.sink_fds,
            &mut self.backlog,
            handle_buf,
            &context,
            buf,
        );
        Ok(buf.len())
    }

    fn on_sendfd(&mut self, _sendfd_request: &SendFdRequest) -> Result<usize> {
        Err(Error::new(ENOSYS))
    }

    fn fcntl(&mut self, _id: usize, _cmd: usize, _arg: usize, _ctx: &CallerCtx) -> Result<usize> {
        Ok(0)
    }

    fn fpath(&mut self, id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        FpathWriter::with(buf, "log", |w| {
            w.push_str(match self.handles.get(id)? {
                LogHandle::Log { context } => context,
                LogHandle::AddSink => "add_sink",
                LogHandle::SchemeRoot => return Err(Error::new(EBADF)),
            })?;
            Ok(())
        })
    }

    fn fsync(&mut self, _id: usize, _ctx: &CallerCtx) -> Result<()> {
        Ok(())
    }

    fn on_close(&mut self, id: usize) {
        self.handles.remove(id);
    }
}
