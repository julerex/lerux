//! Same steps as `SerialTx.tla`, calling the real serial queue functions.
//!
//! The control flow is the client `emit` / `say` loop, `serial_virt_tx`, and
//! `drain_tx`. A change to those branches has to be applied here and in the
//! TLA module together. A change inside `serial_enqueue`, `serial_dequeue`,
//! or the signal functions is executed by this program as it stands.

use std::{
    collections::{HashSet, VecDeque},
    process::ExitCode,
    sync::atomic::{AtomicU32, Ordering},
};

use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_enqueue, serial_queue_free,
    serial_queue_handle_t, serial_queue_init, serial_queue_length_consumer, serial_queue_t,
    serial_request_consumer_signal, serial_require_consumer_signal,
};

const CAPACITY: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Phase {
    Enqueue,
    Request,
    Wait,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Virt {
    None,
    Check,
    Request,
    Recheck,
    Move,
    Drain,
    Wake,
    Resume,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Ring {
    data: [u8; CAPACITY as usize],
    head: u32,
    tail: u32,
    signal: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct World {
    client: Ring,
    driver: Ring,
    uart: Vec<u8>,
    accepted: usize,
    phase: Phase,
    blocked: bool,
    sem: bool,
    pending: bool,
    virt: Virt,
    burst: u32,
    note: bool,
}

struct Rings {
    client_q: Box<serial_queue_t>,
    client_data: Vec<u8>,
    client: serial_queue_handle_t,
    driver_q: Box<serial_queue_t>,
    driver_data: Vec<u8>,
    driver: serial_queue_handle_t,
}

fn empty_ring() -> Ring {
    Ring {
        data: [0; CAPACITY as usize],
        head: 0,
        tail: 0,
        signal: 0,
    }
}

fn init() -> World {
    World {
        client: empty_ring(),
        driver: empty_ring(),
        uart: Vec::new(),
        accepted: 0,
        phase: Phase::Enqueue,
        blocked: false,
        sem: false,
        pending: false,
        virt: Virt::None,
        burst: 0,
        note: false,
    }
}

fn contents(ring: &Ring) -> Vec<u8> {
    let count = ring.tail.wrapping_sub(ring.head);
    (0..count)
        .map(|offset| {
            let index = ring.head.wrapping_add(offset) % CAPACITY;
            ring.data[index as usize]
        })
        .collect()
}

fn mount(ring: &Ring) -> (Box<serial_queue_t>, Vec<u8>, serial_queue_handle_t) {
    let mut queue = Box::new(serial_queue_t {
        tail: AtomicU32::new(ring.tail),
        head: AtomicU32::new(ring.head),
        producer_signalled: AtomicU32::new(ring.signal),
    });
    let mut data = ring.data.to_vec();
    let mut handle = serial_queue_handle_t {
        queue: std::ptr::null_mut(),
        capacity: 0,
        data_region: std::ptr::null_mut(),
    };
    let queue_ptr = &raw mut *queue;
    let data_ptr = data.as_mut_ptr();
    // SAFETY: `queue` and `data` stay in this function's boxes until `snap`
    // has copied the bytes back. `CAPACITY` is non-zero.
    unsafe { serial_queue_init(&mut handle, queue_ptr, CAPACITY, data_ptr) };
    (queue, data, handle)
}

fn snap(queue: &serial_queue_t, data: &[u8]) -> Ring {
    Ring {
        data: data.try_into().expect("data region is the model capacity"),
        head: queue.head.load(Ordering::Relaxed),
        tail: queue.tail.load(Ordering::Relaxed),
        signal: queue.producer_signalled.load(Ordering::Relaxed),
    }
}

fn with_rings<T>(world: &World, body: impl FnOnce(&mut Rings) -> T) -> (T, Ring, Ring) {
    let (client_q, client_data, client) = mount(&world.client);
    let (driver_q, driver_data, driver) = mount(&world.driver);
    let mut rings = Rings {
        client_q,
        client_data,
        client,
        driver_q,
        driver_data,
        driver,
    };
    let value = body(&mut rings);
    let client_ring = snap(&rings.client_q, &rings.client_data);
    let driver_ring = snap(&rings.driver_q, &rings.driver_data);
    (value, client_ring, driver_ring)
}

fn client_turn(world: &World) -> bool {
    world.virt == Virt::None && !world.blocked
}

fn finish_virt(world: &mut World) {
    if world.note {
        world.virt = Virt::Resume;
        world.note = false;
    } else {
        world.virt = Virt::None;
    }
}

fn shell_enqueue(world: &World, bytes: &[u8]) -> Option<World> {
    if !client_turn(world) || world.phase != Phase::Enqueue || world.accepted >= bytes.len() {
        return None;
    }
    let byte = bytes[world.accepted];
    let (status, client, _) = with_rings(world, |rings| {
        // SAFETY: `rings.client` points at the mounted client ring.
        unsafe { serial_enqueue(&rings.client, byte) }
    });
    let mut next = world.clone();
    next.client = client;
    if status == 0 {
        next.accepted += 1;
    } else {
        next.phase = Phase::Request;
    }
    Some(next)
}

fn shell_request(world: &World) -> Option<World> {
    if !client_turn(world) || world.phase != Phase::Request {
        return None;
    }
    let (_, client, _) = with_rings(world, |rings| {
        // SAFETY: `rings.client` points at the mounted client ring.
        unsafe { serial_request_consumer_signal(&rings.client) }
    });
    let mut next = world.clone();
    next.client = client;
    next.phase = Phase::Wait;
    Some(next)
}

fn shell_wait(world: &World) -> Option<World> {
    if !client_turn(world) || world.phase != Phase::Wait {
        return None;
    }
    let mut next = world.clone();
    if next.sem {
        next.sem = false;
        next.phase = Phase::Enqueue;
    } else {
        next.blocked = true;
    }
    Some(next)
}

fn shell_notify(world: &World, bytes: &[u8]) -> Option<World> {
    if !client_turn(world) || world.phase != Phase::Enqueue || world.accepted != bytes.len() {
        return None;
    }
    let mut next = world.clone();
    next.phase = Phase::Idle;
    next.virt = Virt::Check;
    Some(next)
}

fn virt_check(world: &World) -> Option<World> {
    if world.virt != Virt::Check {
        return None;
    }
    let (length, _, _) = with_rings(world, |rings| {
        // SAFETY: `rings.client` points at the mounted client ring.
        unsafe { serial_queue_length_consumer(&rings.client) }
    });
    let mut next = world.clone();
    if length == 0 {
        next.burst = 0;
        finish_virt(&mut next);
    } else {
        next.burst = length;
        next.virt = Virt::Request;
    }
    Some(next)
}

fn virt_request(world: &World) -> Option<World> {
    if world.virt != Virt::Request {
        return None;
    }
    let (_, _, driver) = with_rings(world, |rings| {
        // SAFETY: both handles point at the mounted rings.
        let free = unsafe { serial_queue_free(&rings.driver) };
        if world.burst > free {
            unsafe { serial_request_consumer_signal(&rings.driver) };
        }
    });
    let mut next = world.clone();
    next.driver = driver;
    next.virt = Virt::Recheck;
    Some(next)
}

fn virt_recheck(world: &World) -> Option<World> {
    if world.virt != Virt::Recheck {
        return None;
    }
    let (free, _, _) = with_rings(world, |rings| {
        // SAFETY: `rings.driver` points at the mounted driver ring.
        unsafe { serial_queue_free(&rings.driver) }
    });
    let mut next = world.clone();
    if world.burst > free {
        next.pending = true;
        finish_virt(&mut next);
    } else {
        next.virt = Virt::Move;
    }
    Some(next)
}

fn virt_move(world: &World) -> Option<World> {
    if world.virt != Virt::Move {
        return None;
    }
    let ((), client, driver) = with_rings(world, |rings| {
        for _ in 0..world.burst {
            let mut byte = 0;
            // SAFETY: both handles point at the mounted rings. `burst` is the
            // length `move_client` captured, and the recheck found driver space.
            let got = unsafe { serial_dequeue(&rings.client, &mut byte) };
            assert_eq!(got, 0, "counted client byte is present");
            let put = unsafe { serial_enqueue(&rings.driver, byte) };
            assert_eq!(put, 0, "counted driver space is present");
        }
    });
    let mut next = world.clone();
    next.client = client;
    next.driver = driver;
    next.virt = Virt::Drain;
    Some(next)
}

fn drain_bytes(world: &World) -> (Ring, Vec<u8>, bool) {
    let mut written = Vec::new();
    let (transferred, _, driver) = with_rings(world, |rings| {
        let mut transferred = false;
        loop {
            let mut byte = 0;
            // SAFETY: `rings.driver` points at the mounted driver ring.
            if unsafe { serial_dequeue(&rings.driver, &mut byte) } != 0 {
                break;
            }
            written.push(byte);
            transferred = true;
        }
        if transferred {
            // SAFETY: `rings.driver` points at the mounted driver ring.
            let ask = unsafe { serial_require_consumer_signal(&rings.driver) };
            if ask {
                unsafe { serial_cancel_consumer_signal(&rings.driver) };
            }
            ask
        } else {
            false
        }
    });
    (driver, written, transferred)
}

fn virt_drain(world: &World) -> Option<World> {
    if world.virt != Virt::Drain {
        return None;
    }
    let (driver, written, asked) = drain_bytes(world);
    let mut next = world.clone();
    next.driver = driver;
    next.uart.extend(written);
    if asked {
        next.note = true;
    }
    next.virt = Virt::Wake;
    Some(next)
}

fn virt_wake(world: &World) -> Option<World> {
    if world.virt != Virt::Wake {
        return None;
    }
    let (asked, client, _) = with_rings(world, |rings| {
        // SAFETY: `rings.client` points at the mounted client ring.
        let ask = unsafe { serial_require_consumer_signal(&rings.client) };
        if ask {
            unsafe { serial_cancel_consumer_signal(&rings.client) };
        }
        ask
    });
    let mut next = world.clone();
    next.client = client;
    if asked {
        if next.blocked {
            next.blocked = false;
            next.phase = Phase::Enqueue;
        } else {
            next.sem = true;
        }
    }
    finish_virt(&mut next);
    Some(next)
}

fn virt_resume(world: &World) -> Option<World> {
    if world.virt != Virt::Resume {
        return None;
    }
    let mut next = world.clone();
    if next.pending {
        next.pending = false;
        next.virt = Virt::Check;
    } else {
        next.virt = Virt::None;
    }
    Some(next)
}

fn driver_irq(world: &World) -> Option<World> {
    if matches!(world.virt, Virt::Drain) || contents(&world.driver).is_empty() {
        return None;
    }
    let (driver, written, asked) = drain_bytes(world);
    let mut next = world.clone();
    next.driver = driver;
    next.uart.extend(written);
    if asked {
        if next.virt == Virt::None {
            next.virt = Virt::Resume;
        } else {
            next.note = true;
        }
    }
    Some(next)
}

fn successors(world: &World, bytes: &[u8]) -> Vec<World> {
    [
        shell_enqueue(world, bytes),
        shell_request(world),
        shell_wait(world),
        shell_notify(world, bytes),
        virt_check(world),
        virt_request(world),
        virt_recheck(world),
        virt_move(world),
        virt_drain(world),
        virt_wake(world),
        virt_resume(world),
        driver_irq(world),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn order_ok(world: &World, bytes: &[u8]) -> bool {
    let mut got = world.uart.clone();
    got.extend(contents(&world.driver));
    got.extend(contents(&world.client));
    got == bytes[..world.accepted]
}

fn stuck(world: &World) -> bool {
    world.blocked
        && world.virt == Virt::None
        && !world.pending
        && contents(&world.driver).is_empty()
}

fn terminated(world: &World, bytes: &[u8]) -> bool {
    world.phase == Phase::Idle
        && world.virt == Virt::None
        && !world.blocked
        && !world.pending
        && contents(&world.client).is_empty()
        && contents(&world.driver).is_empty()
        && world.accepted == bytes.len()
        && world.uart == bytes
}

#[derive(Debug)]
enum Outcome {
    Holds,
    Stuck,
    Order,
}

fn explore(count: usize) -> Outcome {
    let bytes: Vec<u8> = (1..=count)
        .map(|n| u8::try_from(n).expect("count fits in a byte"))
        .collect();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    let start = init();
    seen.insert(start.clone());
    queue.push_back(start);
    let mut saw_stuck = false;
    let mut saw_done = false;
    while let Some(world) = queue.pop_front() {
        if !order_ok(&world, &bytes) {
            return Outcome::Order;
        }
        if stuck(&world) {
            saw_stuck = true;
        }
        if terminated(&world, &bytes) {
            saw_done = true;
        }
        for next in successors(&world, &bytes) {
            if seen.insert(next.clone()) {
                queue.push_back(next);
            }
        }
    }
    if saw_stuck {
        Outcome::Stuck
    } else if saw_done {
        Outcome::Holds
    } else {
        Outcome::Order
    }
}

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).unwrap_or_default();
    match mode.as_str() {
        "fits" => match explore(CAPACITY as usize) {
            Outcome::Holds => ExitCode::SUCCESS,
            other => {
                eprintln!("fits: {other:?}");
                ExitCode::from(2)
            }
        },
        "overflow" => match explore(CAPACITY as usize + 1) {
            Outcome::Stuck => ExitCode::from(1),
            other => {
                eprintln!("overflow: {other:?}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("usage: serial-tx-backpressure fits|overflow");
            ExitCode::from(2)
        }
    }
}
