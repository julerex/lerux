//! Phase 79: agent tools over FsRequest, HttpRequest GET, and a tiny batch runner.
//!
//! Execute reads an on-disk command list (same idea as shell `run`) and applies
//! `echo` lines inside this PD. The agent cannot PPC the shell (priority 1;
//! ADR-006). No `fork`/`exec`.

use alloc::vec::Vec;

use lerux_interface_types::{
    AgentToolKind, FsRequest, FsResponse, HttpRequest, HttpResponse, AGENT_WORK_DIR,
    AGENT_WORK_HELLO, MAX_FS_DATA, MAX_FS_PATH, SECTOR_SIZE,
};
use lerux_ipc::{FsClient, HttpClient};
use lerux_logging::log;

use crate::REQUEST_SERVER;

const FS: FsClient = FsClient::new(crate::FS_SERVER);
const TOOL_RESULT_CAP: usize = 512;

pub fn seed_workspace() {
    let _ = FS.call(FsRequest::mkdir(AGENT_WORK_DIR));
    let handle = FS
        .create_or_open(AGENT_WORK_HELLO)
        .expect("agent seed create");
    write_all(handle, b"hello\n").expect("agent seed write");
    log::info!("lerux-agent: workspace ok");
}

pub fn run_tool(kind: AgentToolKind, arg: &[u8]) -> Result<Vec<u8>, ()> {
    let out = match kind {
        AgentToolKind::Read => read_path(arg)?,
        AgentToolKind::Write => write_path(arg)?,
        AgentToolKind::Edit => edit_path(arg)?,
        AgentToolKind::ListDir => list_dir(arg)?,
        AgentToolKind::Search => search(arg)?,
        AgentToolKind::Execute => execute(arg)?,
        AgentToolKind::WebFetch => web_fetch(arg)?,
    };
    Ok(truncate(out))
}

fn truncate(mut v: Vec<u8>) -> Vec<u8> {
    if v.len() > TOOL_RESULT_CAP {
        v.truncate(TOOL_RESULT_CAP);
    }
    v
}

fn read_path(path: &[u8]) -> Result<Vec<u8>, ()> {
    let handle = match FS.call(FsRequest::open(path)) {
        FsResponse::Handle { id } => id,
        _ => return Err(()),
    };
    let size = match FS.call(FsRequest::stat(path)) {
        FsResponse::Stat {
            size,
            is_dir: false,
        } => size as usize,
        _ => return Err(()),
    };
    read_all(handle, size)
}

fn write_path(arg: &[u8]) -> Result<Vec<u8>, ()> {
    let (path, rest) = split_once(arg, b'|').ok_or(())?;
    let handle = FS.create_or_open(path).map_err(|_| ())?;
    write_all(handle, rest)?;
    Ok(Vec::from(b"ok".as_slice()))
}

fn edit_path(arg: &[u8]) -> Result<Vec<u8>, ()> {
    let (path, rest) = split_once(arg, b'|').ok_or(())?;
    let (from, to) = split_once(rest, b'|').unwrap_or((rest, b""));
    let mut body = read_path(path)?;
    if let Some(idx) = find_subslice(&body, from) {
        let mut out = Vec::new();
        out.extend_from_slice(&body[..idx]);
        out.extend_from_slice(to);
        out.extend_from_slice(&body[idx + from.len()..]);
        body = out;
    }
    let handle = FS.create_or_open(path).map_err(|_| ())?;
    write_all(handle, &body)?;
    Ok(body)
}

fn list_dir(path: &[u8]) -> Result<Vec<u8>, ()> {
    match FS.call(FsRequest::list_dir(path)) {
        FsResponse::DirList { count, entries } => {
            let mut out = Vec::new();
            for (i, entry) in entries.iter().enumerate().take(count as usize) {
                if i > 0 {
                    out.push(b'\n');
                }
                let n = entry.name_len as usize;
                out.extend_from_slice(&entry.name[..n]);
            }
            Ok(out)
        }
        _ => Err(()),
    }
}

fn search(arg: &[u8]) -> Result<Vec<u8>, ()> {
    let (dir, needle) = split_once(arg, b'|').unwrap_or((AGENT_WORK_DIR, arg));
    let listing = match FS.call(FsRequest::list_dir(dir)) {
        FsResponse::DirList { count, entries } => (count, entries),
        _ => return Err(()),
    };
    let mut hits = Vec::new();
    for entry in listing.1.iter().take(listing.0 as usize) {
        if entry.is_dir {
            continue;
        }
        let name = &entry.name[..entry.name_len as usize];
        let mut path = Vec::from(dir);
        if !path.ends_with(b"/") {
            path.push(b'/');
        }
        path.extend_from_slice(name);
        if path.len() > MAX_FS_PATH {
            continue;
        }
        if let Ok(body) = read_path(&path)
            && find_subslice(&body, needle).is_some()
        {
            if !hits.is_empty() {
                hits.push(b'\n');
            }
            hits.extend_from_slice(&path);
        }
    }
    Ok(hits)
}

fn execute(path: &[u8]) -> Result<Vec<u8>, ()> {
    let body = read_path(path)?;
    let mut out = Vec::new();
    for line in body.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line).trim_ascii();
        if line.is_empty() || line.starts_with(b"#") {
            continue;
        }
        if let Some(rest) = line.strip_prefix(b"echo ") {
            out.extend_from_slice(rest.trim_ascii());
            out.push(b'\n');
        }
    }
    Ok(out)
}

fn web_fetch(url: &[u8]) -> Result<Vec<u8>, ()> {
    let client = HttpClient::new(REQUEST_SERVER);
    match client.call(HttpRequest::get(url)) {
        HttpResponse::Ok => {}
        _ => return Err(()),
    }
    match client.call(HttpRequest::Finish) {
        HttpResponse::Status { code: 200 } => {}
        _ => {
            let _ = client.call(HttpRequest::Close);
            return Err(());
        }
    }
    let mut out = Vec::new();
    loop {
        match client.call(HttpRequest::Recv) {
            HttpResponse::Data {
                data_len,
                data,
                last,
            } => {
                out.extend_from_slice(&data[..data_len as usize]);
                if last {
                    break;
                }
            }
            _ => {
                let _ = client.call(HttpRequest::Close);
                return Err(());
            }
        }
    }
    let _ = client.call(HttpRequest::Close);
    Ok(out)
}

fn write_all(handle: u8, data: &[u8]) -> Result<(), ()> {
    let mut offset = 0u32;
    while (offset as usize) < data.len() {
        let sector_left = SECTOR_SIZE - (offset as usize % SECTOR_SIZE);
        let end = (offset as usize + MAX_FS_DATA.min(sector_left)).min(data.len());
        match FS.call(FsRequest::write(
            handle,
            offset,
            &data[offset as usize..end],
        )) {
            FsResponse::Ok => {}
            _ => return Err(()),
        }
        offset = end as u32;
    }
    Ok(())
}

fn read_all(handle: u8, len: usize) -> Result<Vec<u8>, ()> {
    let mut out = Vec::new();
    let mut offset = 0u32;
    while (offset as usize) < len {
        let chunk = MAX_FS_DATA.min(len - offset as usize) as u16;
        match FS.call(FsRequest::Read {
            handle,
            offset,
            len: chunk,
        }) {
            FsResponse::Data { data_len, data } => {
                out.extend_from_slice(&data[..data_len as usize]);
                offset += u32::from(data_len);
                if data_len == 0 {
                    break;
                }
            }
            _ => return Err(()),
        }
    }
    Ok(out)
}

fn split_once(arg: &[u8], sep: u8) -> Option<(&[u8], &[u8])> {
    let i = arg.iter().position(|&b| b == sep)?;
    Some((&arg[..i], &arg[i + 1..]))
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}
