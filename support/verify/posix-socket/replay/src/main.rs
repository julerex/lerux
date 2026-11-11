//! Same steps as `PosixSocket.tla`.
//!
//! Socket and file calls go to `lerux_posix`. A branch that moves has to
//! change this file and the TLA module together. The module does not execute
//! this program.

use std::{
    collections::{HashSet, VecDeque},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{Arc, Mutex, MutexGuard},
};

use lerux_posix::{Errno, Files, Open, SockFd, Sockets, Tcp};
use lerux_sddf::{
    fs_cmd_t, fs_cmpl_t, fs_completion_enqueue, fs_message_dequeue, fs_msg_t, fs_queue_t,
    FS_CMD_FILE_OPEN, FS_STATUS_SUCCESS,
};

const HOST: [u8; 4] = [10, 0, 2, 2];
const PORT: u16 = 8443;
const LISTEN_PORT: u16 = 8080;
const SHARE: usize = 0x8000;

const FIRST_FD: i32 = 3;
const SECOND_SOCKET: i32 = 24;
const LISTEN_UNSUPPORTED: i32 = 95;
const LISTEN_IN_USE: i32 = 98;
const SECOND_CONNECT: i32 = 106;
const SEND_BEFORE: i32 = 107;
const BUSY: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Phase {
    Empty,
    Idle,
    Listening,
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct World {
    phase: Phase,
    peer: bool,
}

struct Step {
    name: &'static str,
    errno: i32,
    next: World,
}

struct Shape {
    ebusy: i32,
    emfile: i32,
    eopnotsupp: i32,
    eaddrinuse: i32,
    eisconn: i32,
    enotconn: i32,
    stdio_stored: bool,
}

struct Sample {
    first_fd: i32,
    open_fd: i32,
    second_socket: i32,
    listen_unsupported: i32,
    listen_in_use: i32,
    second_connect: i32,
    send_before: i32,
    busy: i32,
}

struct Break {
    verdict: &'static str,
    action: &'static str,
    detail: String,
}

enum Fault {
    Property(Break),
    Probe(String),
}

struct Inner {
    ready: bool,
    polls: u32,
    block: bool,
    bound: Option<u16>,
}

struct ConnectPipe {
    inner: Arc<Mutex<Inner>>,
}

struct ListenPipe {
    inner: Arc<Mutex<Inner>>,
}

struct BoundPipe {
    inner: Arc<Mutex<Inner>>,
}

struct Slot {
    command: *mut fs_queue_t,
    completion: *mut fs_queue_t,
    files: *mut Files,
    armed: bool,
    nested: i32,
}

struct Harness {
    files: Box<Files>,
    slot: Box<Slot>,
    _command: Box<fs_queue_t>,
    _completion: Box<fs_queue_t>,
    _share: Vec<u8>,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("probe") => match probe() {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                println!("error={message}");
                ExitCode::from(2)
            }
        },
        Some("check") => match audit() {
            Ok(states) => {
                println!("states={states}");
                println!(
                    "trace=Socket ListenDefault ListenBound ListenOk SendEarly PeerArrives Accept Connect ConnectAgain Shutdown Close OpenFile Busy CloseFile"
                );
                println!("verdict=ok");
                ExitCode::SUCCESS
            }
            Err(Fault::Property(broken)) => {
                println!("trace={}", broken.action);
                println!("{}", broken.detail);
                println!("verdict={}", broken.verdict);
                ExitCode::from(1)
            }
            Err(Fault::Probe(message)) => {
                println!("error={message}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("usage: posix-socket probe|check");
            ExitCode::from(2)
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn probe() -> Result<String, String> {
    let shape = read_shape(&repo_root())?;
    let sample = sample_calls()?;
    Ok(format!(
        "first_fd={}\nsecond_socket={}\nlisten_unsupported={}\nlisten_in_use={}\nsecond_connect={}\nsend_before={}\nbusy={}\nstdio_stored={}\n",
        sample.first_fd,
        sample.second_socket,
        sample.listen_unsupported,
        sample.listen_in_use,
        sample.second_connect,
        sample.send_before,
        sample.busy,
        shape.stdio_stored
    ))
}

fn audit() -> Result<usize, Fault> {
    let shape = read_shape(&repo_root()).map_err(Fault::Probe)?;
    let sample = sample_calls().map_err(Fault::Probe)?;
    expect_pair("EMFILE", shape.emfile, SECOND_SOCKET)?;
    expect_pair("EOPNOTSUPP", shape.eopnotsupp, LISTEN_UNSUPPORTED)?;
    expect_pair("EADDRINUSE", shape.eaddrinuse, LISTEN_IN_USE)?;
    expect_pair("EISCONN", shape.eisconn, SECOND_CONNECT)?;
    expect_pair("ENOTCONN", shape.enotconn, SEND_BEFORE)?;
    expect_pair("EBUSY", shape.ebusy, BUSY)?;
    if shape.stdio_stored {
        return Err(Fault::Property(Break::table(
            "Stdio",
            "descriptor 0, 1, or 2 is constructed",
        )));
    }
    expect_errno_value("Socket", FIRST_FD, sample.first_fd)?;
    expect_errno_value("OpenFile", FIRST_FD, sample.open_fd)?;
    expect_errno_value("Socket", SECOND_SOCKET, sample.second_socket)?;
    expect_errno_value(
        "ListenDefault",
        LISTEN_UNSUPPORTED,
        sample.listen_unsupported,
    )?;
    expect_errno_value("ListenBound", LISTEN_IN_USE, sample.listen_in_use)?;
    expect_errno_value("ConnectAgain", SECOND_CONNECT, sample.second_connect)?;
    expect_errno_value("SendEarly", SEND_BEFORE, sample.send_before)?;
    expect_errno_value("Busy", BUSY, sample.busy)?;
    let states = explore()?;
    if states > 10_000 {
        return Err(Fault::Probe("more than 10000 states".to_string()));
    }
    check_files()?;
    Ok(states)
}

fn expect_pair(action: &'static str, parsed: i32, want: i32) -> Result<(), Fault> {
    if parsed == want {
        Ok(())
    } else {
        Err(Fault::Property(Break::errno(action, want, parsed)))
    }
}

fn expect_errno_value(action: &'static str, want: i32, got: i32) -> Result<(), Fault> {
    if got == want {
        Ok(())
    } else {
        Err(Fault::Property(Break::errno(action, want, got)))
    }
}

impl Break {
    fn errno(action: &'static str, want: i32, got: i32) -> Self {
        Self {
            verdict: "errno",
            action,
            detail: format!("want={want} got={got}"),
        }
    }

    fn table(action: &'static str, detail: impl Into<String>) -> Self {
        Self {
            verdict: "table",
            action,
            detail: detail.into(),
        }
    }
}

fn sample_calls() -> Result<Sample, String> {
    let files = file_numbers()?;
    Ok(Sample {
        first_fd: sample_first_socket(),
        open_fd: files.0,
        second_socket: sample_second_socket(),
        listen_unsupported: sample_listen_default(),
        listen_in_use: sample_listen_bound(),
        second_connect: sample_second_connect(),
        send_before: sample_send_before(),
        busy: files.1,
    })
}

fn code_of<T>(result: Result<T, Errno>) -> i32 {
    match result {
        Ok(_) => 0,
        Err(err) => err.0,
    }
}

fn sample_first_socket() -> i32 {
    let mut sockets = Sockets::new(ConnectPipe::new());
    match sockets.socket() {
        Ok(fd) => fd.number(),
        Err(err) => err.0,
    }
}

fn sample_second_socket() -> i32 {
    let mut sockets = Sockets::new(ConnectPipe::new());
    if sockets.socket().is_err() {
        return -1;
    }
    code_of(sockets.socket())
}

fn sample_listen_default() -> i32 {
    let mut sockets = Sockets::new(ConnectPipe::new());
    let Ok(fd) = sockets.socket() else {
        return -1;
    };
    code_of(sockets.listen(fd, LISTEN_PORT))
}

fn sample_listen_bound() -> i32 {
    let mut sockets = Sockets::new(BoundPipe::new());
    let Ok(fd) = sockets.socket() else {
        return -1;
    };
    code_of(sockets.listen(fd, LISTEN_PORT))
}

fn sample_second_connect() -> i32 {
    let mut sockets = Sockets::new(ConnectPipe::new());
    let Ok(fd) = sockets.socket() else {
        return -1;
    };
    if sockets.connect(fd, HOST, PORT).is_err() {
        return -1;
    }
    code_of(sockets.connect(fd, HOST, PORT))
}

fn sample_send_before() -> i32 {
    let mut sockets = Sockets::new(ConnectPipe::new());
    let Ok(fd) = sockets.socket() else {
        return -1;
    };
    code_of(sockets.send(fd, b"x"))
}

fn explore() -> Result<usize, Fault> {
    let init = World {
        phase: Phase::Empty,
        peer: false,
    };
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    seen.insert(init);
    queue.push_back(init);
    while let Some(world) = queue.pop_front() {
        if seen.len() > 10_000 {
            return Err(Fault::Probe("more than 10000 states".to_string()));
        }
        for step in socket_steps(&world) {
            run_step(&world, &step)?;
            if seen.insert(step.next) {
                queue.push_back(step.next);
            }
        }
    }
    Ok(seen.len())
}

fn socket_steps(world: &World) -> Vec<Step> {
    let mut steps = Vec::new();
    let same = *world;
    steps.push(Step {
        name: "Socket",
        errno: if world.phase == Phase::Empty {
            0
        } else {
            SECOND_SOCKET
        },
        next: if world.phase == Phase::Empty {
            World {
                phase: Phase::Idle,
                peer: false,
            }
        } else {
            same
        },
    });
    if world.phase == Phase::Idle {
        steps.push(Step {
            name: "ListenOk",
            errno: 0,
            next: World {
                phase: Phase::Listening,
                peer: false,
            },
        });
        steps.push(Step {
            name: "ListenDefault",
            errno: LISTEN_UNSUPPORTED,
            next: same,
        });
        steps.push(Step {
            name: "ListenBound",
            errno: LISTEN_IN_USE,
            next: same,
        });
        steps.push(Step {
            name: "Connect",
            errno: 0,
            next: World {
                phase: Phase::Open,
                peer: true,
            },
        });
        steps.push(Step {
            name: "SendEarly",
            errno: SEND_BEFORE,
            next: same,
        });
    }
    if world.phase == Phase::Listening && !world.peer {
        steps.push(Step {
            name: "PeerArrives",
            errno: 0,
            next: World {
                phase: Phase::Listening,
                peer: true,
            },
        });
        steps.push(Step {
            name: "SendEarly",
            errno: SEND_BEFORE,
            next: same,
        });
    }
    if world.phase == Phase::Listening && world.peer {
        steps.push(Step {
            name: "Accept",
            errno: 0,
            next: World {
                phase: Phase::Open,
                peer: true,
            },
        });
        steps.push(Step {
            name: "SendEarly",
            errno: SEND_BEFORE,
            next: same,
        });
    }
    if world.phase == Phase::Open {
        steps.push(Step {
            name: "ConnectAgain",
            errno: SECOND_CONNECT,
            next: same,
        });
        steps.push(Step {
            name: "Shutdown",
            errno: 0,
            next: World {
                phase: Phase::Closed,
                peer: false,
            },
        });
    }
    if world.phase != Phase::Empty {
        steps.push(Step {
            name: "Close",
            errno: 0,
            next: World {
                phase: Phase::Empty,
                peer: false,
            },
        });
    }
    steps
}

fn run_step(world: &World, step: &Step) -> Result<(), Fault> {
    if !matches!(
        step.errno,
        0 | SECOND_SOCKET | LISTEN_UNSUPPORTED | LISTEN_IN_USE | SECOND_CONNECT | SEND_BEFORE
    ) {
        return Err(Fault::Property(Break::errno(step.name, 0, step.errno)));
    }
    let result = match step.name {
        "Socket" => socket_edge(world),
        "ListenOk" => listen_ok(),
        "ListenDefault" => listen_default(),
        "ListenBound" => listen_bound(),
        "PeerArrives" => peer_arrives(world),
        "Accept" => accept(),
        "Connect" => connect(),
        "ConnectAgain" => connect_again(),
        "SendEarly" => send_early(world),
        "Shutdown" => shutdown(),
        "Close" => close_socket(world),
        _ => Err(Break::table(step.name, "no socket edge")),
    };
    result.map_err(Fault::Property)
}

fn socket_edge(world: &World) -> Result<(), Break> {
    match world.phase {
        Phase::Empty => {
            let mut sockets = Sockets::new(ConnectPipe::new());
            first_socket("Socket", &mut sockets).map(|_| ())
        }
        Phase::Idle => {
            let (mut sockets, _fd) = staged_connect(Phase::Idle)?;
            expect_occupied("Socket", &mut sockets)
        }
        Phase::Listening => {
            let (mut sockets, _fd, _inner) = staged_listen()?;
            expect_occupied("Socket", &mut sockets)
        }
        Phase::Open => {
            let (mut sockets, _fd) = staged_connect(Phase::Open)?;
            expect_occupied("Socket", &mut sockets)
        }
        Phase::Closed => {
            let (mut sockets, _fd) = staged_connect(Phase::Closed)?;
            expect_occupied("Socket", &mut sockets)
        }
    }
}

fn listen_ok() -> Result<(), Break> {
    let inner = Arc::new(Mutex::new(Inner::open()));
    let mut sockets = Sockets::new(ListenPipe {
        inner: Arc::clone(&inner),
    });
    let fd = first_socket("ListenOk", &mut sockets)?;
    match sockets.listen(fd, LISTEN_PORT) {
        Ok(()) => {}
        Err(err) => return Err(Break::errno("ListenOk", 0, err.0)),
    }
    if fd.number() != FIRST_FD {
        return Err(Break::table("ListenOk", format!("number {}", fd.number())));
    }
    let bound = lock(&inner).bound;
    if bound != Some(LISTEN_PORT) {
        return Err(Break::table("ListenOk", "port was not bound"));
    }
    expect_occupied("ListenOk", &mut sockets)
}

fn listen_default() -> Result<(), Break> {
    let mut sockets = Sockets::new(ConnectPipe::new());
    let fd = first_socket("ListenDefault", &mut sockets)?;
    expect_code(
        "ListenDefault",
        LISTEN_UNSUPPORTED,
        sockets.listen(fd, LISTEN_PORT),
    )?;
    still_idle("ListenDefault", &mut sockets, fd)
}

fn listen_bound() -> Result<(), Break> {
    let mut sockets = Sockets::new(BoundPipe::new());
    let fd = first_socket("ListenBound", &mut sockets)?;
    expect_code(
        "ListenBound",
        LISTEN_IN_USE,
        sockets.listen(fd, LISTEN_PORT),
    )?;
    still_idle("ListenBound", &mut sockets, fd)
}

fn peer_arrives(world: &World) -> Result<(), Break> {
    if world.phase == Phase::Listening && !world.peer {
        Ok(())
    } else {
        Err(Break::table(
            "PeerArrives",
            "peer arrived off a listening socket",
        ))
    }
}

fn accept() -> Result<(), Break> {
    let inner = Arc::new(Mutex::new(Inner::open()));
    let mut sockets = Sockets::new(ListenPipe {
        inner: Arc::clone(&inner),
    });
    let fd = first_socket("Accept", &mut sockets)?;
    sockets
        .listen(fd, LISTEN_PORT)
        .map_err(|err| Break::errno("Accept", 0, err.0))?;
    sockets
        .accept(fd)
        .map_err(|err| Break::errno("Accept", 0, err.0))?;
    let after = lock(&inner);
    if after.polls == 0 || !after.ready {
        return Err(Break::table(
            "Accept",
            "accept returned before the peer was connected",
        ));
    }
    drop(after);
    if fd.number() != FIRST_FD {
        return Err(Break::table("Accept", format!("number {}", fd.number())));
    }
    accept_blocked()
}

fn accept_blocked() -> Result<(), Break> {
    let inner = Arc::new(Mutex::new(Inner::blocked()));
    let mut sockets = Sockets::new(ListenPipe {
        inner: Arc::clone(&inner),
    });
    let fd = first_socket("Accept", &mut sockets)?;
    sockets
        .listen(fd, LISTEN_PORT)
        .map_err(|err| Break::errno("Accept", 0, err.0))?;
    if sockets.accept(fd).is_ok() || lock(&inner).ready {
        return Err(Break::table(
            "Accept",
            "accept returned before the peer was connected",
        ));
    }
    Ok(())
}

fn connect() -> Result<(), Break> {
    let mut sockets = Sockets::new(ConnectPipe::new());
    let fd = first_socket("Connect", &mut sockets)?;
    sockets
        .connect(fd, HOST, PORT)
        .map_err(|err| Break::errno("Connect", 0, err.0))?;
    match sockets.send(fd, b"x") {
        Ok(1) => Ok(()),
        Ok(n) => Err(Break::table("Connect", format!("sent {n}"))),
        Err(err) => Err(Break::errno("Connect", 0, err.0)),
    }
}

fn connect_again() -> Result<(), Break> {
    let (mut sockets, fd) = staged_connect(Phase::Open)?;
    expect_code(
        "ConnectAgain",
        SECOND_CONNECT,
        sockets.connect(fd, HOST, PORT),
    )
}

fn send_early(world: &World) -> Result<(), Break> {
    match world.phase {
        Phase::Idle => {
            let mut sockets = Sockets::new(ConnectPipe::new());
            let fd = first_socket("SendEarly", &mut sockets)?;
            expect_code("SendEarly", SEND_BEFORE, sockets.send(fd, b"x"))?;
            still_idle("SendEarly", &mut sockets, fd)
        }
        Phase::Listening => {
            let (mut sockets, fd, _inner) = staged_listen()?;
            expect_code("SendEarly", SEND_BEFORE, sockets.send(fd, b"x"))?;
            sockets.accept(fd).map_err(|err| {
                Break::table("SendEarly", format!("left listening errno {}", err.0))
            })?;
            Ok(())
        }
        Phase::Empty | Phase::Open | Phase::Closed => Err(Break::table(
            "SendEarly",
            "send-before from a connected phase",
        )),
    }
}

fn shutdown() -> Result<(), Break> {
    let (mut sockets, fd) = staged_connect(Phase::Open)?;
    sockets
        .shutdown(fd)
        .map_err(|err| Break::errno("Shutdown", 0, err.0))?;
    expect_occupied("Shutdown", &mut sockets)
}

fn close_socket(world: &World) -> Result<(), Break> {
    match world.phase {
        Phase::Idle | Phase::Open | Phase::Closed => {
            let (mut sockets, fd) = staged_connect(world.phase)?;
            sockets
                .close(fd)
                .map_err(|err| Break::errno("Close", 0, err.0))?;
            expect_empty("Close", &mut sockets)
        }
        Phase::Listening => {
            let (mut sockets, fd, _inner) = staged_listen()?;
            sockets
                .close(fd)
                .map_err(|err| Break::errno("Close", 0, err.0))?;
            expect_empty("Close", &mut sockets)
        }
        Phase::Empty => Err(Break::table("Close", "close of an empty table")),
    }
}

fn first_socket<T: Tcp>(action: &'static str, sockets: &mut Sockets<T>) -> Result<SockFd, Break> {
    match sockets.socket() {
        Ok(fd) if fd.number() == FIRST_FD => Ok(fd),
        Ok(fd) => Err(Break::table(action, format!("first {}", fd.number()))),
        Err(err) => Err(Break::errno(action, 0, err.0)),
    }
}

fn expect_code<T>(action: &'static str, want: i32, result: Result<T, Errno>) -> Result<(), Break> {
    match result {
        Err(err) if err.0 == want => Ok(()),
        Err(err) => Err(Break::errno(action, want, err.0)),
        Ok(_) => Err(Break::errno(action, want, 0)),
    }
}

fn expect_occupied<T: Tcp>(action: &'static str, sockets: &mut Sockets<T>) -> Result<(), Break> {
    match sockets.socket() {
        Err(err) if err.0 == SECOND_SOCKET => Ok(()),
        Err(err) => Err(Break::errno(action, SECOND_SOCKET, err.0)),
        Ok(fd) => Err(Break::table(action, format!("allocated {}", fd.number()))),
    }
}

fn expect_empty<T: Tcp>(action: &'static str, sockets: &mut Sockets<T>) -> Result<(), Break> {
    match sockets.socket() {
        Ok(fd) if fd.number() == FIRST_FD => Ok(()),
        Ok(fd) => Err(Break::table(action, format!("open number {}", fd.number()))),
        Err(err) => Err(Break::table(
            action,
            format!("still occupied errno {}", err.0),
        )),
    }
}

fn still_idle<T: Tcp>(
    action: &'static str,
    sockets: &mut Sockets<T>,
    fd: SockFd,
) -> Result<(), Break> {
    sockets
        .connect(fd, HOST, PORT)
        .map_err(|err| Break::table(action, format!("left idle errno {}", err.0)))
}

fn staged_connect(phase: Phase) -> Result<(Sockets<ConnectPipe>, SockFd), Break> {
    let mut sockets = Sockets::new(ConnectPipe::new());
    let fd = first_socket("Socket", &mut sockets)?;
    match phase {
        Phase::Idle => Ok((sockets, fd)),
        Phase::Open => {
            sockets
                .connect(fd, HOST, PORT)
                .map_err(|err| Break::errno("Connect", 0, err.0))?;
            Ok((sockets, fd))
        }
        Phase::Closed => {
            sockets
                .connect(fd, HOST, PORT)
                .map_err(|err| Break::errno("Connect", 0, err.0))?;
            sockets
                .shutdown(fd)
                .map_err(|err| Break::errno("Shutdown", 0, err.0))?;
            Ok((sockets, fd))
        }
        Phase::Empty | Phase::Listening => {
            Err(Break::table("Socket", "connect pipe cannot listen"))
        }
    }
}

fn staged_listen() -> Result<(Sockets<ListenPipe>, SockFd, Arc<Mutex<Inner>>), Break> {
    let inner = Arc::new(Mutex::new(Inner::open()));
    let mut sockets = Sockets::new(ListenPipe {
        inner: Arc::clone(&inner),
    });
    let fd = first_socket("ListenOk", &mut sockets)?;
    sockets
        .listen(fd, LISTEN_PORT)
        .map_err(|err| Break::errno("ListenOk", 0, err.0))?;
    Ok((sockets, fd, inner))
}

fn check_files() -> Result<(), Fault> {
    let mut harness = Harness::new().map_err(Fault::Probe)?;
    harness.slot.armed = true;
    let first = match harness.files.open(b"a", Open::CreateWrite) {
        Ok(fd) => fd,
        Err(err) => return Err(Fault::Property(Break::errno("OpenFile", 0, err.0))),
    };
    open_file(first.number())?;
    let nested = harness.slot.nested;
    begin_flight(nested)?;
    busy(nested)?;
    let second = match harness.files.open(b"b", Open::CreateWrite) {
        Ok(fd) => fd,
        Err(err) => return Err(Fault::Property(Break::errno("EndFlight", 0, err.0))),
    };
    end_flight(second.number())?;
    if let Err(err) = harness.files.close(second) {
        return Err(Fault::Property(Break::errno("CloseFile", 0, err.0)));
    }
    if let Err(err) = harness.files.close(first) {
        return Err(Fault::Property(Break::errno("CloseFile", 0, err.0)));
    }
    let third = match harness.files.open(b"c", Open::CreateWrite) {
        Ok(fd) => fd,
        Err(err) => return Err(Fault::Property(Break::errno("CloseFile", 0, err.0))),
    };
    close_file(third.number())
}

fn open_file(number: i32) -> Result<(), Fault> {
    if number == FIRST_FD {
        Ok(())
    } else if number < FIRST_FD {
        Err(Fault::Property(Break::table(
            "OpenFile",
            format!("stored {number}"),
        )))
    } else {
        Err(Fault::Property(Break::table(
            "OpenFile",
            format!("number {number}"),
        )))
    }
}

fn begin_flight(nested: i32) -> Result<(), Fault> {
    if nested < 0 {
        Ok(())
    } else {
        Err(Fault::Property(Break::table(
            "BeginFlight",
            format!("nested {nested}"),
        )))
    }
}

fn busy(nested: i32) -> Result<(), Fault> {
    if nested == -BUSY {
        Ok(())
    } else {
        let got = if nested < 0 { -nested } else { nested };
        Err(Fault::Property(Break::errno("Busy", BUSY, got)))
    }
}

fn end_flight(number: i32) -> Result<(), Fault> {
    if number == FIRST_FD + 1 {
        Ok(())
    } else {
        Err(Fault::Property(Break::table(
            "EndFlight",
            format!("number {number}"),
        )))
    }
}

fn close_file(number: i32) -> Result<(), Fault> {
    if number == FIRST_FD {
        Ok(())
    } else {
        Err(Fault::Property(Break::table(
            "CloseFile",
            format!("reopened {number}"),
        )))
    }
}

fn file_numbers() -> Result<(i32, i32), String> {
    let mut harness = Harness::new()?;
    harness.slot.armed = true;
    let open_fd = match harness.files.open(b"a", Open::CreateWrite) {
        Ok(fd) => fd.number(),
        Err(err) => err.0,
    };
    let busy = if harness.slot.nested < 0 {
        -harness.slot.nested
    } else {
        harness.slot.nested
    };
    Ok((open_fd, busy))
}

fn lock(inner: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    inner.lock().unwrap_or_else(|err| err.into_inner())
}

impl Inner {
    fn open() -> Self {
        Self {
            ready: false,
            polls: 0,
            block: false,
            bound: None,
        }
    }

    fn blocked() -> Self {
        Self {
            block: true,
            ..Self::open()
        }
    }
}

impl ConnectPipe {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::open())),
        }
    }
}

impl BoundPipe {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::open())),
        }
    }
}

fn drive_poll(inner: &Mutex<Inner>) -> bool {
    let mut guard = lock(inner);
    guard.polls += 1;
    if guard.block {
        return false;
    }
    guard.ready = true;
    true
}

fn drive_connect(addr: [u8; 4], port: u16) -> Result<(), Errno> {
    if addr == HOST && port == PORT {
        Ok(())
    } else {
        Err(Errno(Errno::ECONNREFUSED))
    }
}

fn drive_send(data: &[u8]) -> Result<usize, Errno> {
    Ok(data.len())
}

fn drive_recv(_buf: &mut [u8]) -> Result<usize, Errno> {
    Err(Errno(Errno::EAGAIN))
}

// The trait default is the listen that returns errno 95. This pipe does not override it.
impl Tcp for ConnectPipe {
    fn start(&mut self) {
        lock(&self.inner).ready = false;
    }

    fn poll(&mut self) -> bool {
        drive_poll(&self.inner)
    }

    fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Errno> {
        drive_connect(addr, port)
    }

    fn ready(&self) -> bool {
        lock(&self.inner).ready
    }

    fn failed(&self) -> bool {
        false
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, Errno> {
        drive_send(data)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
        drive_recv(buf)
    }
}

impl Tcp for ListenPipe {
    fn start(&mut self) {
        lock(&self.inner).ready = false;
    }

    fn poll(&mut self) -> bool {
        drive_poll(&self.inner)
    }

    fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Errno> {
        drive_connect(addr, port)
    }

    fn listen(&mut self, port: u16) -> Result<(), Errno> {
        lock(&self.inner).bound = Some(port);
        Ok(())
    }

    fn ready(&self) -> bool {
        lock(&self.inner).ready
    }

    fn failed(&self) -> bool {
        false
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, Errno> {
        drive_send(data)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
        drive_recv(buf)
    }
}

impl Tcp for BoundPipe {
    fn start(&mut self) {
        lock(&self.inner).ready = false;
    }

    fn poll(&mut self) -> bool {
        drive_poll(&self.inner)
    }

    fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Errno> {
        drive_connect(addr, port)
    }

    fn listen(&mut self, _port: u16) -> Result<(), Errno> {
        Err(Errno(Errno::EADDRINUSE))
    }

    fn ready(&self) -> bool {
        lock(&self.inner).ready
    }

    fn failed(&self) -> bool {
        false
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, Errno> {
        drive_send(data)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
        drive_recv(buf)
    }
}

impl Harness {
    fn new() -> Result<Self, String> {
        let mut command = empty_queue();
        let mut completion = empty_queue();
        let mut share = vec![0u8; SHARE];
        let command_ptr = std::ptr::from_mut::<fs_queue_t>(command.as_mut());
        let completion_ptr = std::ptr::from_mut::<fs_queue_t>(completion.as_mut());
        let share_ptr = share.as_mut_ptr();
        let slot = Box::new(Slot {
            command: command_ptr,
            completion: completion_ptr,
            files: std::ptr::null_mut(),
            armed: false,
            nested: 0,
        });
        let slot_ptr = (&raw const *slot).cast_mut();
        let files = unsafe {
            Files::bind(
                command_ptr,
                completion_ptr,
                share_ptr,
                SHARE,
                wake_busy,
                slot_ptr.cast(),
            )
        }
        .map_err(|err| format!("bind errno {}", err.0))?;
        let mut files = Box::new(files);
        // SAFETY: `files` is the box allocated above and outlives every wake.
        // The nested open returns from `idle` before it writes a slot.
        unsafe {
            (*slot_ptr).files = &raw mut *files;
        }
        Ok(Self {
            files,
            slot,
            _command: command,
            _completion: completion,
            _share: share,
        })
    }
}

fn wake_busy(ctx: *mut ()) {
    // SAFETY: `ctx` is the `Slot` box installed in `Files::bind`. The nested
    // open returns at `idle` while the outer `exchange` holds the in-flight id.
    let slot = unsafe { &mut *ctx.cast::<Slot>() };
    if slot.armed {
        slot.armed = false;
        slot.nested = if slot.files.is_null() {
            0
        } else {
            let files = unsafe { &mut *slot.files };
            match files.open(b"busy", Open::CreateWrite) {
                Ok(fd) => fd.number(),
                Err(err) => -err.0,
            }
        };
    }
    serve(slot);
}

fn serve(slot: &mut Slot) {
    let mut msg = fs_msg_t {
        cmd: empty_command(),
    };
    if unsafe { fs_message_dequeue(slot.command, &mut msg) } != 0 {
        return;
    }
    let cmd = unsafe { msg.cmd };
    let mut cmpl: fs_cmpl_t = unsafe { core::mem::zeroed() };
    cmpl.id = cmd.id;
    cmpl.status = FS_STATUS_SUCCESS;
    if cmd.r#type == FS_CMD_FILE_OPEN {
        cmpl.data.file_open.fd = 1;
    }
    let _ = unsafe { fs_completion_enqueue(slot.completion, cmpl) };
}

fn empty_command() -> fs_cmd_t {
    fs_cmd_t {
        id: 0,
        r#type: 0,
        params: lerux_sddf::fs_cmd_params_t { min_size: [0; 48] },
    }
}

fn empty_queue() -> Box<fs_queue_t> {
    let queue = Box::<fs_queue_t>::new_zeroed();
    // SAFETY: an all-zero ring is empty. The fields are integers and atomics.
    unsafe { queue.assume_init() }
}

fn read_shape(root: &Path) -> Result<Shape, String> {
    let socket = fs::read_to_string(root.join("userspace/crates/lerux-posix/src/socket.rs"))
        .map_err(|err| format!("socket.rs: {err}"))?;
    let lib = fs::read_to_string(root.join("userspace/crates/lerux-posix/src/lib.rs"))
        .map_err(|err| format!("lib.rs: {err}"))?;
    markers(&socket, &lib)?;
    Ok(Shape {
        ebusy: const_i32(&lib, "EBUSY")?,
        emfile: const_i32(&lib, "EMFILE")?,
        eopnotsupp: const_i32(&lib, "EOPNOTSUPP")?,
        eaddrinuse: const_i32(&lib, "EADDRINUSE")?,
        eisconn: const_i32(&lib, "EISCONN")?,
        enotconn: const_i32(&lib, "ENOTCONN")?,
        stdio_stored: stores_stdio(&socket) || stores_stdio(&lib),
    })
}

fn markers(socket: &str, lib: &str) -> Result<(), String> {
    let new_body = fn_after(socket, "fn new(tcp: T)")?;
    must_contain(&new_body, "fn new", "fd: 3")?;
    let socket_body = fn_after(socket, "fn socket(&mut self)")?;
    must_contain(&socket_body, "fn socket", "Errno::EMFILE")?;
    must_contain(&socket_body, "fn socket", "Phase::Empty")?;
    let connect_body = fn_after(socket, "fn connect(&mut self, fd: SockFd")?;
    must_contain(&connect_body, "fn connect", "Errno::EISCONN")?;
    let default_listen = fn_after(socket, "fn listen(&mut self, _port: u16)")?;
    must_contain(&default_listen, "fn listen default", "Errno::EOPNOTSUPP")?;
    let listen_body = fn_after(socket, "fn listen(&mut self, fd: SockFd, port: u16)")?;
    must_contain(&listen_body, "fn listen", "Phase::Listening")?;
    must_contain(&listen_body, "fn listen", "self.tcp.listen")?;
    let accept_body = fn_after(socket, "fn accept(&mut self, fd: SockFd)")?;
    must_contain(&accept_body, "fn accept", "Phase::Listening")?;
    must_contain(&accept_body, "fn accept", "wait_until_open")?;
    let send_body = fn_after(socket, "fn send(&mut self, fd: SockFd")?;
    must_contain(&send_body, "fn send", "self.open(fd)")?;
    let shutdown_body = fn_after(socket, "fn shutdown(&mut self, fd: SockFd)")?;
    must_contain(&shutdown_body, "fn shutdown", "Phase::Closed")?;
    let close_body = fn_after(socket, "fn close(&mut self, fd: SockFd)")?;
    must_contain(&close_body, "fn close", "Phase::Empty")?;
    must_contain(&close_body, "fn close", "self.tcp.finish")?;
    let open_sock = fn_after(socket, "fn open(&self, fd: SockFd)")?;
    must_contain(&open_sock, "fn open", "Errno::ENOTCONN")?;
    must_contain(&open_sock, "fn open", "Phase::Open")?;
    if !lib.contains("const TABLE_LEN: usize = MAX_FDS - 3;") {
        return Err("TABLE_LEN is not MAX_FDS - 3".to_string());
    }
    let open_file = fn_after(lib, "fn open(&mut self, path: &[u8]")?;
    must_contain(&open_file, "fn open", "index + 3")?;
    let index_body = fn_after(lib, "fn index(fd: Fd)")?;
    must_contain(&index_body, "fn index", "- 3")?;
    let idle_body = fn_after(lib, "fn idle(&self)")?;
    must_contain(&idle_body, "fn idle", "Errno::EBUSY")?;
    let exchange_body = fn_after(lib, "fn exchange(&mut self, mut cmd: fs_cmd_t)")?;
    must_contain(&exchange_body, "fn exchange", "Errno::EBUSY")?;
    Ok(())
}

fn must_contain(body: &str, signature: &str, token: &str) -> Result<(), String> {
    if body.contains(token) {
        Ok(())
    } else {
        Err(format!("{signature} does not mention {token}"))
    }
}

fn const_i32(src: &str, name: &str) -> Result<i32, String> {
    let needle = format!("const {name}: i32 = ");
    let Some(at) = src.find(&needle) else {
        return Err(format!("missing {name}"));
    };
    let rest = &src[at + needle.len()..];
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(format!("bad {name}"));
    }
    digits.parse().map_err(|_| format!("bad {name}"))
}

fn stores_stdio(src: &str) -> bool {
    [
        "Fd(0)",
        "Fd(1)",
        "Fd(2)",
        "SockFd(0)",
        "SockFd(1)",
        "SockFd(2)",
    ]
    .iter()
    .any(|token| src.contains(token))
}

fn fn_after(src: &str, signature: &str) -> Result<String, String> {
    let Some(start) = src.find(signature) else {
        return Err(format!("missing {signature}"));
    };
    let rest = &src[start..];
    let Some(brace) = rest.find('{') else {
        return Err(format!("{signature} has no body"));
    };
    let mut depth = 0usize;
    for (index, ch) in rest[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(rest[brace..=brace + index].to_string());
                }
            }
            _ => {}
        }
    }
    Err(format!("{signature} body does not close"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_parse() {
        let src = "pub const EMFILE: i32 = 24;\npub const EBUSY: i32 = 16;\n";
        assert_eq!(const_i32(src, "EMFILE").unwrap(), 24);
        assert_eq!(const_i32(src, "EBUSY").unwrap(), 16);
    }

    #[test]
    fn a_missing_signature_is_an_error() {
        let err = fn_after("fn other() {}", "fn socket(&mut self)").unwrap_err();
        assert!(err.contains("missing"));
    }

    #[test]
    fn a_socket_without_emfile_is_rejected() {
        let src = "fn socket(&mut self) { self.phase = Phase::Idle; }";
        let body = fn_after(src, "fn socket(&mut self)").unwrap();
        assert!(must_contain(&body, "fn socket", "Errno::EMFILE").is_err());
    }

    #[test]
    fn stdio_tokens_are_stored() {
        assert!(stores_stdio("let fd = Fd(0);"));
        assert!(!stores_stdio("let fd = Fd(index + 3);"));
    }

    #[test]
    fn abstract_socket_graph_is_six_states() {
        let init = World {
            phase: Phase::Empty,
            peer: false,
        };
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();
        seen.insert(init);
        queue.push_back(init);
        while let Some(world) = queue.pop_front() {
            for step in socket_steps(&world) {
                if seen.insert(step.next) {
                    queue.push_back(step.next);
                }
            }
        }
        assert_eq!(seen.len(), 6);
    }

    #[test]
    fn idle_steps_cover_the_documented_errnos() {
        let idle = World {
            phase: Phase::Idle,
            peer: false,
        };
        let errnos: Vec<i32> = socket_steps(&idle).iter().map(|step| step.errno).collect();
        for errno in [
            0,
            SECOND_SOCKET,
            LISTEN_UNSUPPORTED,
            LISTEN_IN_USE,
            SEND_BEFORE,
        ] {
            assert!(errnos.contains(&errno), "{errno}");
        }
    }

    #[test]
    fn protection_domain_sources_still_parse() {
        assert!(read_shape(&repo_root()).is_ok());
    }
}
