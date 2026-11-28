//! Same steps as `CothreadContract.tla`.
//!
//! The calls go to the real `lerux-cothread` runtime. The TLA module does not
//! execute Rust. A contract break is a panic, so the model has no next state
//! for it. This program catches that panic on the caller that the contract
//! names, and it drops the process lock before every switch.

use std::{
    array,
    cell::UnsafeCell,
    env,
    panic::{catch_unwind, AssertUnwindSafe},
    process::ExitCode,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use lerux_cothread::{
    microkit_cothread_destroy, microkit_cothread_init, microkit_cothread_my_handle,
    microkit_cothread_query_state, microkit_cothread_recv_ntfn, microkit_cothread_semaphore_init,
    microkit_cothread_semaphore_signal, microkit_cothread_semaphore_wait, microkit_cothread_spawn,
    microkit_cothread_wait_on_channel, microkit_cothread_yield, Semaphore, State, MAX_COTHREADS,
    MIN_STACK_SIZE, NULL_HANDLE, ROOT_HANDLE, STACK_SLOTS,
};

/// Process lock for this replay. The runtime does not lock. Host tests in
/// `lerux-cothread` take their own lock the same way. A guard held across
/// `co_switch` deadlocks: the mutex is not reentrant, and the switched-to
/// cothread is the same operating-system thread.
static LOCK: Mutex<()> = Mutex::new(());

const CH_WAKE: usize = 1;
const CH_ROOT_WAIT: usize = 2;
const CH_BLOCK: usize = 3;
const CH_WORKER: usize = 4;

static RECV_RESUMED: AtomicBool = AtomicBool::new(false);
static SIGNAL_RESUMED: AtomicBool = AtomicBool::new(false);
static LOCK_FREE_RECV: AtomicBool = AtomicBool::new(false);
static LOCK_FREE_SIGNAL: AtomicBool = AtomicBool::new(false);
static LOCK_FREE_BLOCK: AtomicBool = AtomicBool::new(false);
static LOCK_FREE_WORKER: AtomicBool = AtomicBool::new(false);
static WORKER_RECV_PANICKED: AtomicBool = AtomicBool::new(false);
static PARKED_RAN: AtomicBool = AtomicBool::new(false);

struct SemSlot(UnsafeCell<Semaphore>);

// SAFETY: one operating-system thread. The root and a worker use the cell
// only across a cooperative switch, and neither holds `LOCK` across that switch.
unsafe impl Sync for SemSlot {}

static SEM: SemSlot = SemSlot(UnsafeCell::new(Semaphore::new()));

#[repr(C, align(16))]
struct Stack([u8; MIN_STACK_SIZE]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Wake,
    Full,
}

impl Mode {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "wake" => Some(Self::Wake),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Obs {
    pool: i32,
    null_handle: i32,
    root_handle: i32,
    recv_switched: bool,
    signal_switched: bool,
    full_returns_null: bool,
    second_init_panics: bool,
    bad_handle_panics: bool,
    root_wait_panics: bool,
    worker_recv_panics: bool,
    destroy_blocked_panics: bool,
    lock_across_switch: bool,
    worker_panicked: bool,
}

impl Obs {
    fn failed() -> Self {
        Self {
            pool: STACK_SLOTS as i32,
            null_handle: NULL_HANDLE,
            root_handle: ROOT_HANDLE,
            recv_switched: false,
            signal_switched: false,
            full_returns_null: false,
            second_init_panics: false,
            bad_handle_panics: false,
            root_wait_panics: false,
            worker_recv_panics: false,
            destroy_blocked_panics: false,
            lock_across_switch: false,
            worker_panicked: false,
        }
    }

    #[cfg(test)]
    fn documented() -> Self {
        Self {
            pool: STACK_SLOTS as i32,
            null_handle: NULL_HANDLE,
            root_handle: ROOT_HANDLE,
            recv_switched: true,
            signal_switched: true,
            full_returns_null: true,
            second_init_panics: true,
            bad_handle_panics: true,
            root_wait_panics: true,
            worker_recv_panics: true,
            destroy_blocked_panics: true,
            lock_across_switch: false,
            worker_panicked: false,
        }
    }

    fn holds(self) -> bool {
        self.recv_switched
            && self.signal_switched
            && self.full_returns_null
            && self.second_init_panics
            && self.bad_handle_panics
            && self.root_wait_panics
            && self.worker_recv_panics
            && self.destroy_blocked_panics
            && !self.lock_across_switch
            && !self.worker_panicked
    }
}

fn main() -> ExitCode {
    // Expected contract breaks panic and are caught. The default hook would
    // print those as failures of a passing run.
    std::panic::set_hook(Box::new(|_| {}));
    match catch_unwind(real_main) {
        Ok(code) => code,
        Err(payload) => {
            eprintln!("panic={}", panic_message(&payload));
            println!("verdict=contract");
            ExitCode::from(1)
        }
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_string())
        })
        .unwrap_or_else(|| "contract break".to_string())
}

fn real_main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    let which = args.next();
    let Some(mode) = which.as_deref().and_then(Mode::parse) else {
        eprintln!("usage: cothread-contract probe|check wake|full");
        return ExitCode::from(2);
    };
    let obs = exercise();
    match cmd.as_deref() {
        Some("probe") => {
            print_probe(&obs);
            ExitCode::SUCCESS
        }
        Some("check") => check(mode, &obs),
        _ => {
            eprintln!("usage: cothread-contract probe|check wake|full");
            ExitCode::from(2)
        }
    }
}

fn print_probe(obs: &Obs) {
    println!("pool={}", obs.pool);
    println!("null_handle={}", obs.null_handle);
    println!("root_handle={}", obs.root_handle);
    println!("recv_switched={}", obs.recv_switched);
    println!("signal_switched={}", obs.signal_switched);
    println!("full_returns_null={}", obs.full_returns_null);
    println!("second_init_panics={}", obs.second_init_panics);
    println!("bad_handle_panics={}", obs.bad_handle_panics);
    println!("root_wait_panics={}", obs.root_wait_panics);
    println!("worker_recv_panics={}", obs.worker_recv_panics);
    println!("destroy_blocked_panics={}", obs.destroy_blocked_panics);
    println!("lock_across_switch={}", obs.lock_across_switch);
    println!("worker_panicked={}", obs.worker_panicked);
}

fn check(mode: Mode, obs: &Obs) -> ExitCode {
    if !obs.holds() {
        print_probe(obs);
        println!("verdict=contract");
        return ExitCode::from(1);
    }
    let steps = trace(mode);
    println!("states={}", steps.len() + 1);
    println!("trace={}", steps.join(" "));
    println!("verdict=ok");
    ExitCode::SUCCESS
}

fn trace(mode: Mode) -> &'static [&'static str] {
    match mode {
        Mode::Wake => &[
            "AcquireInit",
            "DoInit",
            "SpawnCh",
            "ReleaseYieldCh",
            "YieldCh",
            "WaitCh",
            "AcquireRecv",
            "ReleaseRecv",
            "RecvSwitch",
            "WorkerExitCh",
            "RecvReturn",
            "AcquireSpawnSem",
            "SpawnSem",
            "ReleaseYieldSem",
            "YieldSem",
            "WaitSem",
            "AcquireSignal",
            "ReleaseSignal",
            "SignalSwitch",
            "WorkerExitSem",
            "SignalReturn",
        ],
        Mode::Full => &[
            "AcquireInit",
            "DoInit",
            "Fill",
            "Fill",
            "Fill",
            "Fill",
            "Fill",
            "Fill",
            "Fill",
            "Fill",
            "Overflow",
            "Reap",
            "Reap",
            "Reap",
            "Reap",
            "Reap",
            "Reap",
            "Reap",
            "Reap",
        ],
    }
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn panics(body: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(body)).is_err()
}

fn saw_lock_free() -> bool {
    LOCK.try_lock().is_ok()
}

fn sem() -> *mut Semaphore {
    SEM.0.get()
}

fn root_is_running() -> bool {
    let _guard = lock();
    microkit_cothread_my_handle() == ROOT_HANDLE
}

fn exercise() -> Obs {
    RECV_RESUMED.store(false, Ordering::SeqCst);
    SIGNAL_RESUMED.store(false, Ordering::SeqCst);
    LOCK_FREE_RECV.store(false, Ordering::SeqCst);
    LOCK_FREE_SIGNAL.store(false, Ordering::SeqCst);
    LOCK_FREE_BLOCK.store(false, Ordering::SeqCst);
    LOCK_FREE_WORKER.store(false, Ordering::SeqCst);
    WORKER_RECV_PANICKED.store(false, Ordering::SeqCst);
    PARKED_RAN.store(false, Ordering::SeqCst);

    let mut storage = Vec::with_capacity(STACK_SLOTS);
    for _ in 0..STACK_SLOTS {
        storage.push(Stack([0; MIN_STACK_SIZE]));
    }
    let ptrs = array::from_fn(|index| storage[index].0.as_mut_ptr());

    let channel = {
        let _guard = lock();
        let init_panicked = panics(|| unsafe {
            microkit_cothread_init(MIN_STACK_SIZE, &ptrs);
        });
        if init_panicked {
            return Obs::failed();
        }
        microkit_cothread_spawn(channel_waiter, ptr::null_mut())
    };
    if channel == NULL_HANDLE || !root_is_running() {
        return Obs::failed();
    }

    microkit_cothread_yield();
    let blocked = {
        let _guard = lock();
        microkit_cothread_query_state(channel) == State::Blocked
            && microkit_cothread_my_handle() == ROOT_HANDLE
    };
    if !blocked {
        return Obs::failed();
    }
    microkit_cothread_recv_ntfn(CH_WAKE);

    let mut obs = Obs::failed();

    {
        let _guard = lock();
        obs.recv_switched = RECV_RESUMED.load(Ordering::SeqCst)
            && microkit_cothread_my_handle() == ROOT_HANDLE
            && microkit_cothread_query_state(channel) == State::NotActive;
    }
    if obs.recv_switched && !LOCK_FREE_RECV.load(Ordering::SeqCst) {
        obs.lock_across_switch = true;
    }
    if !obs.recv_switched || !root_is_running() {
        return obs;
    }

    let signal = semaphore_round();
    obs.signal_switched = signal.switched;
    if signal.switched && !signal.lock_free {
        obs.lock_across_switch = true;
    }
    if !signal.switched || !root_is_running() {
        return obs;
    }

    obs.full_returns_null = full_pool();
    if PARKED_RAN.load(Ordering::SeqCst) {
        obs.worker_panicked = true;
    }
    if !obs.full_returns_null || !root_is_running() {
        return obs;
    }

    obs.bad_handle_panics = bad_handle_panics();
    obs.root_wait_panics = root_wait_panics();
    if !root_is_running() {
        return obs;
    }

    match destroy_blocked() {
        BlockedDestroy::Panicked { lock_free } => {
            obs.destroy_blocked_panics = true;
            if !lock_free {
                obs.lock_across_switch = true;
            }
        }
        BlockedDestroy::Returned => {
            obs.destroy_blocked_panics = false;
            return obs;
        }
        BlockedDestroy::NotBlocked => {
            obs.destroy_blocked_panics = false;
            return obs;
        }
    }
    if !root_is_running() {
        return obs;
    }

    let worker = worker_recv();
    obs.worker_recv_panics = worker.finished && worker.panicked;
    if worker.finished && !worker.lock_free {
        obs.lock_across_switch = true;
    }
    if !root_is_running() {
        return obs;
    }

    obs.second_init_panics = second_init_panics(&ptrs);
    obs
}

struct SignalRound {
    switched: bool,
    lock_free: bool,
}

fn semaphore_round() -> SignalRound {
    SIGNAL_RESUMED.store(false, Ordering::SeqCst);
    LOCK_FREE_SIGNAL.store(false, Ordering::SeqCst);
    let handle = {
        let _guard = lock();
        unsafe { microkit_cothread_semaphore_init(&mut *sem()) };
        microkit_cothread_spawn(sem_waiter, ptr::null_mut())
    };
    if handle == NULL_HANDLE {
        return SignalRound {
            switched: false,
            lock_free: false,
        };
    }
    microkit_cothread_yield();
    let blocked = {
        let _guard = lock();
        microkit_cothread_query_state(handle) == State::Blocked
            && microkit_cothread_my_handle() == ROOT_HANDLE
    };
    if !blocked {
        return SignalRound {
            switched: false,
            lock_free: false,
        };
    }
    unsafe { microkit_cothread_semaphore_signal(&mut *sem()) };
    let _guard = lock();
    SignalRound {
        switched: SIGNAL_RESUMED.load(Ordering::SeqCst)
            && microkit_cothread_my_handle() == ROOT_HANDLE
            && microkit_cothread_query_state(handle) == State::NotActive,
        lock_free: LOCK_FREE_SIGNAL.load(Ordering::SeqCst),
    }
}

fn full_pool() -> bool {
    let _guard = lock();
    let mut handles = Vec::with_capacity(STACK_SLOTS);
    for _ in 0..STACK_SLOTS {
        let handle = microkit_cothread_spawn(parked, ptr::null_mut());
        if handle == NULL_HANDLE {
            for live in handles {
                microkit_cothread_destroy(live);
            }
            return false;
        }
        handles.push(handle);
    }
    let extra = catch_unwind(AssertUnwindSafe(|| {
        microkit_cothread_spawn(parked, ptr::null_mut())
    }));
    for live in handles {
        microkit_cothread_destroy(live);
    }
    matches!(extra, Ok(NULL_HANDLE)) && !PARKED_RAN.load(Ordering::SeqCst)
}

fn bad_handle_panics() -> bool {
    let _guard = lock();
    panics(|| {
        let _ = microkit_cothread_query_state(NULL_HANDLE);
    }) && panics(|| {
        let _ = microkit_cothread_query_state(MAX_COTHREADS as i32);
    }) && panics(|| {
        let _ = microkit_cothread_query_state(i32::MAX);
    }) && panics(|| {
        microkit_cothread_destroy(NULL_HANDLE);
    })
}

fn root_wait_panics() -> bool {
    let _guard = lock();
    let sem_panics = panics(|| {
        let mut local = Semaphore::new();
        microkit_cothread_semaphore_wait(&mut local);
    });
    let channel_panics = panics(|| microkit_cothread_wait_on_channel(CH_ROOT_WAIT));
    sem_panics && channel_panics
}

enum BlockedDestroy {
    Panicked { lock_free: bool },
    Returned,
    NotBlocked,
}

fn destroy_blocked() -> BlockedDestroy {
    LOCK_FREE_BLOCK.store(false, Ordering::SeqCst);
    let handle = {
        let _guard = lock();
        microkit_cothread_spawn(blocker, ptr::null_mut())
    };
    if handle == NULL_HANDLE {
        return BlockedDestroy::NotBlocked;
    }
    microkit_cothread_yield();
    let blocked = {
        let _guard = lock();
        microkit_cothread_query_state(handle) == State::Blocked
    };
    if !blocked {
        return BlockedDestroy::NotBlocked;
    }
    let panicked = {
        let _guard = lock();
        panics(|| microkit_cothread_destroy(handle))
    };
    if !panicked {
        return BlockedDestroy::Returned;
    }
    microkit_cothread_recv_ntfn(CH_BLOCK);
    BlockedDestroy::Panicked {
        lock_free: LOCK_FREE_BLOCK.load(Ordering::SeqCst),
    }
}

struct WorkerRecv {
    panicked: bool,
    finished: bool,
    lock_free: bool,
}

fn worker_recv() -> WorkerRecv {
    LOCK_FREE_WORKER.store(false, Ordering::SeqCst);
    WORKER_RECV_PANICKED.store(false, Ordering::SeqCst);
    let handle = {
        let _guard = lock();
        microkit_cothread_spawn(recv_from_worker, ptr::null_mut())
    };
    if handle == NULL_HANDLE {
        return WorkerRecv {
            panicked: false,
            finished: false,
            lock_free: false,
        };
    }
    microkit_cothread_yield();
    let _guard = lock();
    WorkerRecv {
        panicked: WORKER_RECV_PANICKED.load(Ordering::SeqCst),
        finished: microkit_cothread_query_state(handle) == State::NotActive
            && microkit_cothread_my_handle() == ROOT_HANDLE,
        lock_free: LOCK_FREE_WORKER.load(Ordering::SeqCst),
    }
}

fn second_init_panics(ptrs: &[*mut u8; STACK_SLOTS]) -> bool {
    let _guard = lock();
    panics(|| unsafe {
        microkit_cothread_init(MIN_STACK_SIZE, ptrs);
    })
}

extern "C" fn channel_waiter() {
    let entered = saw_lock_free();
    microkit_cothread_wait_on_channel(CH_WAKE);
    let resumed = saw_lock_free();
    if entered && resumed {
        LOCK_FREE_RECV.store(true, Ordering::SeqCst);
    }
    RECV_RESUMED.store(true, Ordering::SeqCst);
}

extern "C" fn sem_waiter() {
    let entered = saw_lock_free();
    unsafe { microkit_cothread_semaphore_wait(&mut *sem()) };
    let resumed = saw_lock_free();
    if entered && resumed {
        LOCK_FREE_SIGNAL.store(true, Ordering::SeqCst);
    }
    SIGNAL_RESUMED.store(true, Ordering::SeqCst);
}

extern "C" fn blocker() {
    let entered = saw_lock_free();
    microkit_cothread_wait_on_channel(CH_BLOCK);
    let resumed = saw_lock_free();
    if entered && resumed {
        LOCK_FREE_BLOCK.store(true, Ordering::SeqCst);
    }
}

extern "C" fn recv_from_worker() {
    if saw_lock_free() {
        LOCK_FREE_WORKER.store(true, Ordering::SeqCst);
    }
    let panicked = panics(|| microkit_cothread_recv_ntfn(CH_WORKER));
    WORKER_RECV_PANICKED.store(panicked, Ordering::SeqCst);
}

extern "C" fn parked() {
    PARKED_RAN.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_and_stack_match_the_contract() {
        assert_eq!(NULL_HANDLE, -1);
        assert_eq!(ROOT_HANDLE, 0);
        assert_eq!(STACK_SLOTS, 7);
        assert_eq!(MAX_COTHREADS, STACK_SLOTS + 1);
        assert_eq!(MIN_STACK_SIZE, 0x1000);
        assert_eq!(MIN_STACK_SIZE % 16, 0);
        let stack = Stack([0; MIN_STACK_SIZE]);
        let address = ptr::from_ref(&stack) as usize;
        assert_eq!(address % 16, 0);
    }

    #[test]
    fn documented_outcomes_hold_and_each_break_does_not() {
        assert!(Obs::documented().holds());
        let mut missing_switch = Obs::documented();
        missing_switch.recv_switched = false;
        assert!(!missing_switch.holds());
        let mut missing_panic = Obs::documented();
        missing_panic.destroy_blocked_panics = false;
        assert!(!missing_panic.holds());
        let mut full_panic = Obs::documented();
        full_panic.full_returns_null = false;
        assert!(!full_panic.holds());
        let mut locked = Obs::documented();
        locked.lock_across_switch = true;
        assert!(!locked.holds());
        let mut panicked = Obs::documented();
        panicked.worker_panicked = true;
        assert!(!panicked.holds());
    }

    #[test]
    fn wake_trace_releases_the_lock_before_each_switch() {
        let steps = trace(Mode::Wake);
        for window in steps.windows(2) {
            let switches = matches!(
                window[1],
                "YieldCh"
                    | "WaitCh"
                    | "RecvSwitch"
                    | "WorkerExitCh"
                    | "YieldSem"
                    | "WaitSem"
                    | "SignalSwitch"
                    | "WorkerExitSem"
            );
            if switches {
                assert!(
                    window[0].starts_with("Release")
                        || window[0] == "YieldCh"
                        || window[0] == "RecvSwitch"
                        || window[0] == "YieldSem"
                        || window[0] == "SignalSwitch",
                    "{} then {}",
                    window[0],
                    window[1]
                );
            }
        }
        assert_eq!(steps.len() + 1, 22);
    }

    #[test]
    fn full_trace_spawns_past_the_pool_then_reaps() {
        let steps = trace(Mode::Full);
        assert_eq!(steps.iter().filter(|step| **step == "Fill").count(), 8);
        assert_eq!(steps.iter().filter(|step| **step == "Reap").count(), 8);
        assert!(steps.contains(&"Overflow"));
        assert_eq!(steps.len() + 1, 20);
    }
}
