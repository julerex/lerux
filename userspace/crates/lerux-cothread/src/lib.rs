//! Stackful cothreads for one Microkit kernel thread.
//!
//! The operations match `libmicrokitco` at the commit LionsOS 0.4.0 pins
//! (`4bf88ee`): `microkit_cothread_init`, `spawn`, `yield`, `wait_on_channel`,
//! `semaphore_wait`, and `semaphore_signal`. Each cothread has a caller-provided
//! stack. The root cothread is the kernel thread: it runs `init` and
//! `Handler::notified`, and it is the only caller of [`microkit_cothread_recv_ntfn`].
//!
//! New blocking input and output uses this crate. `lerux-service-async` stays
//! for postcard servers that already poll a single future.
//!
//! A contract break panics. Those are the cases where `libmicrokitco` faults the
//! protection domain: a second init, a bad handle, a wait on the root cothread,
//! or `recv_ntfn` from a worker. Spawn returning [`NULL_HANDLE`] is the ordinary
//! full-pool result, not a panic.

#![cfg_attr(not(test), no_std)]

mod switch;

use core::{cell::UnsafeCell, ptr};

/// Process-global cell for a runtime that has one kernel thread.
///
/// `Sync` is sound because a protection domain does not share this cell across
/// kernel threads. Host tests take a lock before they touch it.
pub(crate) struct Exclusive<T>(UnsafeCell<T>);

// SAFETY: a protection domain has one kernel thread. Host tests lock before use.
unsafe impl<T> Sync for Exclusive<T> {}

impl<T> Exclusive<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub(crate) fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// Cothreads in one protection domain, including the root kernel thread.
pub const MAX_COTHREADS: usize = 8;

/// Stacks passed to [`microkit_cothread_init`]. The root kernel thread is not included.
pub const STACK_SLOTS: usize = MAX_COTHREADS - 1;

/// Smallest stack `libmicrokitco` accepts. One page.
pub const MIN_STACK_SIZE: usize = 0x1000;

/// Microkit channel ids are `0..MICROKIT_MAX_CHANNELS`. The kit defines 62.
pub const MICROKIT_MAX_CHANNELS: usize = 62;

/// Spawn result when every worker slot is in use.
pub const NULL_HANDLE: i32 = -1;

/// Handle of the kernel thread. Init leaves this cothread running.
pub const ROOT_HANDLE: i32 = 0;

/// Execution state of one cothread. The numeric values match `co_state_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    /// The handle is in the free pool, or it has not been spawned.
    NotActive = 0,
    /// Waiting on a semaphore or a channel. Not scheduled.
    Blocked = 1,
    /// In the scheduling queue.
    Ready = 2,
    /// The kernel thread is executing this cothread.
    Running = 3,
}

/// User-land semaphore. A signal with an empty queue sticks until the next wait.
#[derive(Clone, Copy, Debug)]
pub struct Semaphore {
    set: bool,
    head: i32,
    tail: i32,
}

impl Default for Semaphore {
    fn default() -> Self {
        Self::new()
    }
}

impl Semaphore {
    /// Empty queue and a clear signal, which is what `semaphore_init` writes.
    pub const fn new() -> Self {
        Self {
            set: false,
            head: NULL_HANDLE,
            tail: NULL_HANDLE,
        }
    }

    /// True when a signal arrived and no cothread has waited for it yet.
    pub fn is_set(&self) -> bool {
        self.set
    }

    /// True when no cothread is blocked on this semaphore.
    pub fn is_queue_empty(&self) -> bool {
        self.head == NULL_HANDLE
    }
}

struct SemLinks {
    set: bool,
    head: i32,
    tail: i32,
}

impl SemLinks {
    fn from(sem: &Semaphore) -> Self {
        Self {
            set: sem.set,
            head: sem.head,
            tail: sem.tail,
        }
    }

    fn apply(self, sem: &mut Semaphore) {
        sem.set = self.set;
        sem.head = self.head;
        sem.tail = self.tail;
    }
}

#[derive(Clone, Copy)]
struct Tcb {
    stack: *mut u8,
    co_handle: *mut u8,
    entry: Option<extern "C" fn()>,
    private_arg: *mut u8,
    state: State,
    next_blocked: i32,
}

impl Tcb {
    const fn empty() -> Self {
        Self {
            stack: ptr::null_mut(),
            co_handle: ptr::null_mut(),
            entry: None,
            private_arg: ptr::null_mut(),
            state: State::NotActive,
            next_blocked: NULL_HANDLE,
        }
    }
}

struct Queue {
    items: usize,
    head: usize,
    tail: usize,
    mem: [i32; MAX_COTHREADS],
}

impl Queue {
    const fn new() -> Self {
        Self {
            items: 0,
            head: 0,
            tail: 0,
            mem: [0; MAX_COTHREADS],
        }
    }

    fn push(&mut self, handle: i32) {
        if self.items == MAX_COTHREADS {
            panic!("lerux-cothread: scheduling queue is full");
        }
        self.mem[self.tail] = handle;
        self.items += 1;
        self.tail = (self.tail + 1) % MAX_COTHREADS;
    }

    fn pop(&mut self) -> Option<i32> {
        if self.items == 0 {
            return None;
        }
        let handle = self.mem[self.head];
        self.head = (self.head + 1) % MAX_COTHREADS;
        self.items -= 1;
        Some(handle)
    }

    fn retain(&mut self, mut keep: impl FnMut(i32) -> bool) {
        let mut fresh = Self::new();
        while let Some(handle) = self.pop() {
            if keep(handle) {
                fresh.push(handle);
            }
        }
        *self = fresh;
    }
}

struct Control {
    initialised: bool,
    stack_size: usize,
    running: i32,
    tcbs: [Tcb; MAX_COTHREADS],
    free: Queue,
    sched: Queue,
    channels: [Semaphore; MICROKIT_MAX_CHANNELS],
}

impl Control {
    const fn empty() -> Self {
        Self {
            initialised: false,
            stack_size: 0,
            running: ROOT_HANDLE,
            tcbs: [Tcb::empty(); MAX_COTHREADS],
            free: Queue::new(),
            sched: Queue::new(),
            channels: [Semaphore::new(); MICROKIT_MAX_CHANNELS],
        }
    }
}

// One runtime per protection domain, matching libmicrokitco's single controller.
// Host tests serialize on a lock because the static is process-global.
static CONTROL: Exclusive<Control> = Exclusive::new(Control::empty());

fn with_control<R>(body: impl FnOnce(&mut Control) -> R) -> R {
    // SAFETY: one kernel thread. Callers drop this borrow before `co_switch`.
    let control = unsafe { &mut *CONTROL.get() };
    if !control.initialised {
        panic!("lerux-cothread: call init before using the runtime");
    }
    body(control)
}

fn idx(handle: i32) -> usize {
    usize::try_from(handle)
        .ok()
        .filter(|handle| *handle < MAX_COTHREADS)
        .unwrap_or_else(|| panic!("lerux-cothread: invalid handle {handle}"))
}

fn channel_index(channel: usize) -> usize {
    if channel >= MICROKIT_MAX_CHANNELS {
        panic!("lerux-cothread: channel {channel} is outside 0..{MICROKIT_MAX_CHANNELS}");
    }
    channel
}

/// Prepare the runtime.
///
/// `stacks` has one pointer per worker slot. Each points at `stack_size` bytes
/// that are 16-byte aligned, at least [`MIN_STACK_SIZE`], and a multiple of 16.
/// The memory stays exclusive to that cothread for the rest of the protection domain.
///
/// # Safety
/// The pointed-to bytes are writable, do not overlap, and outlive every cothread.
pub unsafe fn microkit_cothread_init(stack_size: usize, stacks: &[*mut u8; STACK_SLOTS]) {
    if stack_size < MIN_STACK_SIZE || !stack_size.is_multiple_of(16) {
        panic!(
            "lerux-cothread: stack size {stack_size:#x} is below {MIN_STACK_SIZE:#x} or not a multiple of 16"
        );
    }
    validate_stacks(stack_size, stacks);
    // SAFETY: the static is the only controller, and init runs before other calls.
    let control = unsafe { &mut *CONTROL.get() };
    if control.initialised {
        panic!("lerux-cothread: already initialised");
    }
    *control = Control::empty();
    control.stack_size = stack_size;
    control.running = ROOT_HANDLE;
    control.tcbs[0].state = State::Running;
    control.tcbs[0].co_handle = switch::install_root();
    for (slot, stack) in stacks.iter().copied().enumerate() {
        let handle = slot + 1;
        control.tcbs[handle].stack = stack;
        control.free.push(handle as i32);
    }
    control.initialised = true;
}

fn validate_stacks(stack_size: usize, stacks: &[*mut u8; STACK_SLOTS]) {
    for stack in stacks {
        if stack.is_null() || !(*stack as usize).is_multiple_of(16) {
            panic!("lerux-cothread: stack is null or not 16-byte aligned");
        }
        // A store faults when the caller named memory the protection domain cannot write.
        // SAFETY: `stack_size` is non-zero and the pointer is the caller's stack base.
        unsafe {
            ptr::write_volatile(*stack, 0);
            ptr::write_volatile(stack.add(stack_size - 1), 0);
        }
    }
    for (index, stack) in stacks.iter().copied().enumerate() {
        let start = stack as usize;
        let end = start
            .checked_add(stack_size)
            .unwrap_or_else(|| panic!("lerux-cothread: stack range overflows"));
        for other in stacks.iter().copied().skip(index + 1) {
            let other_start = other as usize;
            let other_end = other_start
                .checked_add(stack_size)
                .unwrap_or_else(|| panic!("lerux-cothread: stack range overflows"));
            if start < other_end && other_start < end {
                panic!("lerux-cothread: stacks overlap");
            }
        }
    }
}

/// Create a cothread and put it at the back of the scheduling queue.
///
/// The first spawn returns 1. The entry runs only after [`microkit_cothread_yield`]
/// or a semaphore signal switches to it. When `entry` returns, the handle goes
/// back to the free pool. `private_arg` is what [`microkit_cothread_my_arg`] reads.
pub fn microkit_cothread_spawn(entry: extern "C" fn(), private_arg: *mut u8) -> i32 {
    with_control(|control| {
        let Some(handle) = control.free.pop() else {
            return NULL_HANDLE;
        };
        let index = idx(handle);
        let stack = control.tcbs[index].stack;
        let stack_size = control.stack_size;
        // SAFETY: init checked this stack, and the handle is not running.
        unsafe { ptr::write_bytes(stack, 0, stack_size) };
        // SAFETY: the same stack, still exclusive, with the size init checked.
        let co_handle = unsafe { switch::derive(stack, stack_size) };
        control.tcbs[index].co_handle = co_handle;
        control.tcbs[index].entry = Some(entry);
        control.tcbs[index].private_arg = private_arg;
        control.tcbs[index].state = State::Ready;
        control.tcbs[index].next_blocked = NULL_HANDLE;
        control.sched.push(handle);
        handle
    })
}

/// Handle of the cothread the kernel thread is executing.
pub fn microkit_cothread_my_handle() -> i32 {
    with_control(|control| control.running)
}

/// Argument passed to [`microkit_cothread_spawn`] for the running worker.
pub fn microkit_cothread_my_arg() -> *mut u8 {
    with_control(|control| {
        if control.running == ROOT_HANDLE {
            panic!("lerux-cothread: my_arg called from the root cothread");
        }
        control.tcbs[idx(control.running)].private_arg
    })
}

/// State of `handle`. A handle outside `0..MAX_COTHREADS` panics.
pub fn microkit_cothread_query_state(handle: i32) -> State {
    with_control(|control| control.tcbs[idx(handle)].state)
}

/// Put the caller at the back of the scheduling queue and run the next ready cothread.
///
/// The caller keeps running when it is the only ready cothread.
pub fn microkit_cothread_yield() {
    let switched = with_control(|control| {
        let running = control.running;
        control.sched.push(running);
        control.tcbs[idx(running)].state = State::Ready;
        take_next(control)
    });
    // SAFETY: `take_next` returned a handle planted by init or spawn.
    unsafe { switch::co_switch(switched) };
}

/// Release `handle`. Destroying a blocked cothread panics. Destroying the running
/// cothread switches to whoever is ready, or back to the root.
pub fn microkit_cothread_destroy(handle: i32) {
    let switched = with_control(|control| release(control, handle));
    if let Some(next) = switched {
        // SAFETY: `release` returned the next planted handle.
        unsafe { switch::co_switch(next) };
        panic!("lerux-cothread: destroyed cothread resumed");
    }
}

fn release(control: &mut Control, handle: i32) -> Option<*mut u8> {
    let index = idx(handle);
    if handle == ROOT_HANDLE {
        panic!("lerux-cothread: cannot destroy the root cothread");
    }
    match control.tcbs[index].state {
        State::NotActive => panic!("lerux-cothread: destroy of an inactive handle"),
        State::Blocked => panic!("lerux-cothread: cannot destroy a blocked cothread"),
        State::Running if handle != control.running => {
            panic!("lerux-cothread: destroy of a running handle that is not current")
        }
        State::Ready | State::Running => {}
    }
    control.sched.retain(|queued| queued != handle);
    control.free.push(handle);
    control.tcbs[index].state = State::NotActive;
    control.tcbs[index].entry = None;
    if handle == control.running {
        Some(take_next(control))
    } else {
        None
    }
}

fn take_next(control: &mut Control) -> *mut u8 {
    let next = loop {
        let Some(handle) = control.sched.pop() else {
            break ROOT_HANDLE;
        };
        if handle == ROOT_HANDLE || control.tcbs[idx(handle)].state == State::Ready {
            break handle;
        }
    };
    control.tcbs[idx(next)].state = State::Running;
    control.running = next;
    control.tcbs[idx(next)].co_handle
}

/// Block the running worker on `sem`, or consume a sticky signal and return.
///
/// The root cothread cannot wait. It receives notifications and signals.
pub fn microkit_cothread_semaphore_wait(sem: &mut Semaphore) {
    let mut links = SemLinks::from(sem);
    let blocked = block_running(&mut links);
    links.apply(sem);
    if blocked {
        // SAFETY: the waiter was switched off the ready queue by `block_running`.
        // The next handle belongs to a planted context.
        unsafe { switch::co_switch(scheduled_handle()) };
    }
}

/// Wake one waiter and switch to it. A signal with no waiter sticks.
pub fn microkit_cothread_semaphore_signal(sem: &mut Semaphore) {
    let mut links = SemLinks::from(sem);
    let wake = with_control(|control| signal_one(control, &mut links));
    links.apply(sem);
    if let Some(handle) = wake {
        // SAFETY: `signal_one` selected a blocked cothread that spawn planted.
        unsafe { switch::co_switch(handle) };
    }
}

/// Same as [`Semaphore::new`].
pub fn microkit_cothread_semaphore_init(sem: &mut Semaphore) {
    *sem = Semaphore::new();
}

/// Block the running worker until [`microkit_cothread_recv_ntfn`] signals `channel`.
pub fn microkit_cothread_wait_on_channel(channel: usize) {
    let channel = channel_index(channel);
    let mut links = with_control(|control| SemLinks::from(&control.channels[channel]));
    let blocked = block_running(&mut links);
    with_control(|control| links.apply(&mut control.channels[channel]));
    if blocked {
        // SAFETY: same as `semaphore_wait`. The channel semaphore lives in the controller.
        unsafe { switch::co_switch(scheduled_handle()) };
    }
}

/// Signal the cothread blocked in [`microkit_cothread_wait_on_channel`].
///
/// Only the root cothread may call this. Call it from `Handler::notified`.
/// A waiting worker is switched to immediately, and this function returns when
/// that worker blocks again or returns.
pub fn microkit_cothread_recv_ntfn(channel: usize) {
    let channel = channel_index(channel);
    let wake = with_control(|control| {
        if control.running != ROOT_HANDLE {
            panic!("lerux-cothread: recv_ntfn called from a worker cothread");
        }
        let mut links = SemLinks::from(&control.channels[channel]);
        let wake = signal_one(control, &mut links);
        links.apply(&mut control.channels[channel]);
        wake
    });
    if let Some(handle) = wake {
        // SAFETY: the handle is the planted context of the woken worker.
        unsafe { switch::co_switch(handle) };
    }
}

fn block_running(links: &mut SemLinks) -> bool {
    if links.set {
        links.set = false;
        return false;
    }
    with_control(|control| {
        if control.running == ROOT_HANDLE {
            panic!("lerux-cothread: the root cothread cannot wait");
        }
        let running = control.running;
        let index = idx(running);
        control.tcbs[index].state = State::Blocked;
        control.tcbs[index].next_blocked = NULL_HANDLE;
        if links.head == NULL_HANDLE {
            links.head = running;
            links.tail = running;
        } else {
            let tail = idx(links.tail);
            control.tcbs[tail].next_blocked = running;
            links.tail = running;
        }
    });
    true
}

fn signal_one(control: &mut Control, links: &mut SemLinks) -> Option<*mut u8> {
    if links.set {
        return None;
    }
    if links.head == NULL_HANDLE {
        links.set = true;
        return None;
    }
    let head = links.head;
    let next = control.tcbs[idx(head)].next_blocked;
    control.tcbs[idx(head)].next_blocked = NULL_HANDLE;
    let running = control.running;
    control.sched.push(running);
    control.tcbs[idx(running)].state = State::Ready;
    links.head = next;
    if next == NULL_HANDLE {
        links.tail = NULL_HANDLE;
        links.set = false;
    }
    control.running = head;
    control.tcbs[idx(head)].state = State::Running;
    Some(control.tcbs[idx(head)].co_handle)
}

fn scheduled_handle() -> *mut u8 {
    with_control(take_next)
}

/// First instruction of a new cothread. The assembly entry calls this.
#[unsafe(no_mangle)]
extern "C" fn lerux_cothread_started() -> ! {
    let entry = with_control(|control| {
        control.tcbs[idx(control.running)].entry.unwrap_or_else(|| {
            panic!("lerux-cothread: started a cothread with no entry");
        })
    });
    entry();
    microkit_cothread_destroy(microkit_cothread_my_handle());
    panic!("lerux-cothread: cothread returned after destroy");
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

    use super::*;

    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static RAN: AtomicBool = AtomicBool::new(false);
    static RESUMED: AtomicBool = AtomicBool::new(false);
    static STEP: AtomicU8 = AtomicU8::new(0);
    static SEM: Exclusive<Semaphore> = Exclusive::new(Semaphore::new());

    const CHANNEL: usize = 3;

    #[repr(C, align(16))]
    struct Stack([u8; MIN_STACK_SIZE]);

    fn reset() {
        // SAFETY: the test lock is held, so no cothread is inside the runtime.
        unsafe { (*CONTROL.get()).initialised = false };
        RAN.store(false, Ordering::SeqCst);
        RESUMED.store(false, Ordering::SeqCst);
        STEP.store(0, Ordering::SeqCst);
        // SAFETY: the semaphore is only used by the semaphore test, under the lock.
        unsafe { *SEM.get() = Semaphore::new() };
    }

    fn init_stacks() -> std::vec::Vec<Stack> {
        let mut storage = std::vec::Vec::with_capacity(STACK_SLOTS);
        for _ in 0..STACK_SLOTS {
            storage.push(Stack([0; MIN_STACK_SIZE]));
        }
        let ptrs = core::array::from_fn(|index| storage[index].0.as_mut_ptr());
        // SAFETY: `storage` outlives the switches below. The test returns to this
        // frame before dropping it, and reset drops any leftover ready cothread.
        unsafe { microkit_cothread_init(MIN_STACK_SIZE, &ptrs) };
        storage
    }

    extern "C" fn waiter() {
        microkit_cothread_wait_on_channel(CHANNEL);
        RESUMED.store(true, Ordering::SeqCst);
    }

    extern "C" fn runner() {
        STEP.store(1, Ordering::SeqCst);
        RAN.store(true, Ordering::SeqCst);
    }

    extern "C" fn sem_waiter() {
        // SAFETY: `SEM` is a test semaphore outside the controller. The lock is held.
        unsafe { microkit_cothread_semaphore_wait(&mut *SEM.get()) };
        RESUMED.store(true, Ordering::SeqCst);
    }

    extern "C" fn sem_signaler() {
        RAN.store(true, Ordering::SeqCst);
        // SAFETY: same semaphore as `sem_waiter`.
        unsafe { microkit_cothread_semaphore_signal(&mut *SEM.get()) };
    }

    #[test]
    fn blocked_cothread_resumes_when_its_channel_is_signalled() {
        let _guard = LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset();
        let _stacks = init_stacks();
        let preserved = 0xA5A5_5A5A_u64;
        let waiter_handle = microkit_cothread_spawn(waiter, ptr::null_mut());
        let runner_handle = microkit_cothread_spawn(runner, ptr::null_mut());
        assert_eq!(waiter_handle, 1);
        assert_eq!(runner_handle, 2);

        microkit_cothread_yield();

        assert_eq!(preserved, 0xA5A5_5A5A_u64);
        assert!(RAN.load(Ordering::SeqCst));
        assert!(!RESUMED.load(Ordering::SeqCst));
        assert_eq!(STEP.load(Ordering::SeqCst), 1);
        assert_eq!(microkit_cothread_query_state(waiter_handle), State::Blocked);
        assert_eq!(
            microkit_cothread_query_state(runner_handle),
            State::NotActive
        );

        microkit_cothread_recv_ntfn(CHANNEL);

        assert!(RESUMED.load(Ordering::SeqCst));
        assert_eq!(preserved, 0xA5A5_5A5A_u64);
        assert_eq!(microkit_cothread_my_handle(), ROOT_HANDLE);
    }

    #[test]
    fn semaphore_signal_resumes_the_waiter() {
        let _guard = LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset();
        let _stacks = init_stacks();
        assert_ne!(
            microkit_cothread_spawn(sem_waiter, ptr::null_mut()),
            NULL_HANDLE
        );
        assert_ne!(
            microkit_cothread_spawn(sem_signaler, ptr::null_mut()),
            NULL_HANDLE
        );

        microkit_cothread_yield();

        assert!(RAN.load(Ordering::SeqCst));
        assert!(RESUMED.load(Ordering::SeqCst));
        assert_eq!(microkit_cothread_my_handle(), ROOT_HANDLE);
    }
}
