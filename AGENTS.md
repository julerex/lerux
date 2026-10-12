# AGENTS

Instructions for LLM agents writing Rust in this repository.

Read [docs/context.md](docs/context.md) first for domain terms (protection domain, Microkit, board features, rust-sel4).

## Scope

- Applies to all Rust files (`**/*.rs`) in this repo.
- Two contexts with different rules:
  - **Userspace** (`userspace/`) — `#![no_std]` protection domains and shared crates on seL4 Microkit
  - **Host tooling** (`tools/lerux-cli/`) — `std` build and test orchestration on the developer machine
- Favor correctness and matching existing patterns over drive-by refactors.
- Do not modify upstream trees in `deps/workspace/`.

## General idiomatic Rust

Apply to all Rust code unless a context-specific section overrides.

- Prefer borrowing (`&T`, `&str`, `&[T]`) over `.clone()` unless ownership transfer is required.
- Prefer `?` and `let … else { … }` over deep `match` chains for early exit.
- Prefer iterator pipelines for pure transforms; use `for` when `break`, `continue`, or side effects dominate.
- Import order: `core`/`alloc` → external crates → workspace / `lerux-*` → `crate::` / `super::`.
- Prefer `From` / `Into` / `TryFrom` over manual bit-twiddling conversions.
- Use `#[expect(clippy::…)]` with a one-line rationale instead of blanket `#[allow]`.
- **Channel numbers come from the profile manifest**, not freehand magic. Use named `const` `Channel` values that match `support/profiles/*.toml` `[[channel]]` ends (and the composed SDF). Run `lerux profile check-channels` after renumbering; see Phase 41 / ADR-001 / `docs/system-generation.md`. An sDDF image with no profile keeps the channel ids in the system template and copies them into the config struct. The protection domain reads that struct. See [LionsOS queues](#lionsos-queues-lerux-sddf).
- Match existing naming: `HandlerImpl`, `SERIAL_DRIVER`, `*_DRIVER` channel constants.
- Keep comments purposeful (`why`, invariants, safety); remove stale commentary.
- Link TODOs to issues: `// TODO(#NNN): …`.

## Protection domains (`userspace/pds/**`)

Match the style already used in PD crates such as `echo-server` and `boot-init`.

### Crate attributes and entry

```rust
#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const CLIENT: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    // init sinks, drivers, log readiness
    HandlerImpl
}
```

- Every PD `main.rs` uses `#![no_std]` and `#![no_main]`.
- Entry is `#[protection_domain] fn init() -> HandlerImpl` (or equivalent handler type).
- IRQ and notification handling uses `impl Handler for HandlerImpl` with `type Error = Infallible`.

### Panics and `unwrap`

`unwrap()` and `expect()` are acceptable in PD init and top-level handlers when failure is unrecoverable (firmware convention). Prefer `expect("invariant message")` over bare `unwrap()`. Use `unreachable!()` for channels that cannot arrive per the `.system` layout.

Do not drive-by refactor existing `unwrap` sites unless the task requires it.

### IPC

- Use `lerux_ipc` and typed messages from `lerux_interface_types` (postcard + serde).
- On decode failure, return `send_unspecified_error()` rather than panicking.
- Example pattern: `userspace/pds/echo-server/src/main.rs`.
- New queue and filesystem code uses `lerux-sddf` (`serial_queue_t`, `fs_cmd_t`, and the other headers named in ADR-011). Do not add a postcard message for a new device or filesystem connection. Postcard remains for protection domains that ADR-011 has not replaced yet.

### Logging

- Use `lerux_logging::serial` or `lerux_logging::debug` sinks.
- Apply `lerux_logging::default_filter` where noisy `sel4_sys` targets should be suppressed.

### Board features

- Gate platform-specific code with `#[cfg(feature = "board-…")]` in source and matching features in `Cargo.toml`.
- Never hardcode a single platform in logic shared across boards.
- Enable `alloc` only when `sel4-microkit/alloc` (or a PD feature that pulls it in) is required; prefer stack or static buffers in hot paths.

## Shared userspace libraries (`userspace/crates/**`)

Stricter than PDs.

- Stay `#![no_std]` unless there is a strong reason not to.
- Document public items with `//!` / `///`, including IPC contracts and safety assumptions.
- Prefer `Result` in fallible APIs; avoid new `panic!` in library code.
- Re-export upstream rust-sel4 APIs rather than reimplementing them (see `lerux-ipc`).
- Restrict `unsafe` to MMIO/HAL boundaries with documented invariants (see `lerux-virtio-hal`).
- Pin rust-sel4 via workspace git deps at `v4.0.0`; do not vendor copies.

## LionsOS queues (`lerux-sddf`)

Read [docs/plan-lionsos.md](docs/plan-lionsos.md) and [ADR-011](docs/decisions/011-lionsos-structures.md). One milestone per pass. Leave a postcard protection domain in place until that milestone's exit says to delete it. The serial example is `qemu_virt_aarch64_serial_sddf`: `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, and `serial_client`. A missing header in the pinned LionsOS 0.4.0 checkout stops that milestone. Do not invent the struct.

### Config bytes

- Fill a `#[repr(C)]` config by zeroing it, then assigning fields. A struct literal leaves padding uninitialized, so two fills of `serial_connection_resource_t` do not compare equal: the `u8` `id` is padded out to the next pointer. `serial_config_from_bytes` is `unsafe` because a `bool` other than 0 or 1 is undefined behavior. The generator writes 0 or 1.
- The installed Microkit kit is 2.2.0 and cannot prefill a memory region. Do not objcopy a zero `#[link_section]` static. rustc places that static in `.bss`. Each protection domain `build.rs` writes `OUT_DIR/config.bin` from the `lerux-sddf::serial_image` constructors. The domain `include_bytes!` those bytes and copies them into an aligned value. `include_bytes!` has alignment 1. The host and the guest are both 64-bit little-endian, so the pointer-sized virtual addresses survive the copy.
- The protection domain compiles under `just check-pd` without a board build. It must not read files that `lerux-cli` writes during `build()`. `tools/lerux-cli/src/serial_sddf.rs` writes the same bytes so a host test can compare them with the rendered system description.
- Block and filesystem configs use that same path. The constructors are `lerux-sddf::blk_image` and `lerux-sddf::fs_image`. `tools/lerux-cli/src/fs_sddf.rs` writes the bytes the host test compares. `fatfs` writes `blk_client_config_t` and then `fs_server_config_t` into one `config.bin`. The shell writes `serial_client_config_t` and then `fs_image::shell_client_config()` into one `config.bin`. On `qemu_virt_aarch64_fs_sddf` that file continues with the timer client page and then the schedule page. On `qemu_virt_aarch64_fetch_sddf`, `qemu_virt_aarch64_web_sddf`, and `qemu_virt_aarch64_browser_sddf` that file continues with `net_image::shell_client_config()`. Each serial role on that image embeds its own serial config. `sddf-fs-client` still embeds `client_config()` and stays off the image. On `qemu_virt_aarch64_agent_sddf` the serial client is `sddf-grok` and embeds the serial client config only. The agent embeds `fs_image::shell_client_config()` and then `net_image::shell_client_config()`. It has no serial page.
- Template virtual addresses use the same digit grouping as the Rust constants (`0x2_000_000`). `0x2_000_000` and `0x2000000` are the same number and different spellings.
- Copy a large config once during init. `serial_virt_tx_config_t` is several kilobytes, and the stack is `0x10_000`. Keep queue handles and channel ids in the handler.

### Channels, queues, and the serial device

- The device interrupt is not a field of `serial_driver_config_t`. The template `<irq id>` uses `DRIVER_IRQ_CHANNEL`.
- Implement `Handler::notified` on every protection domain that can be notified. Implement `Handler::protected` before setting `pp="true"`. Both defaults panic.
- Microkit zeroes shared pages, so `producer_signalled` starts at 0 and the consumer signals the producer. The client still implements `notified` when that signal only returns transmit space. After an enqueue, the producer notifies the consumer. The consumer notifies the producer only when the queue asks for that signal, then cancels it.
- Queue indexes use an acquire load for the index the other side writes and a release store for the index this side owns.
- The QEMU PL011 path reuses `sel4-pl011-driver`. Leave the baud alone. `Write::write` spins while the transmit FIFO is full, so the transmit drain does not arm a transmit interrupt. The C driver in `deps/workspace/lionsos/dep/sddf/drivers/serial/arm/uart.c` ORs `PL011_LCR_PARTY_EN` where a disable was intended. That OR enables parity. The Rust driver does not copy it.
- Crate and program names use hyphens (`sddf-serial-driver.elf`). Microkit protection-domain names use underscores (`serial_driver`). A search for `serial-driver.elf` also matches `sddf-serial-driver.elf`.
- `just test-serial-sddf` expects `lerux shell ready` from `serial_enqueue`. The client does not link `lerux-logging`. Keep that substring out of `expect` and `assert` messages.

## Cothreads (`lerux-cothread`)

Read Milestone 3 in [docs/plan-lionsos.md](docs/plan-lionsos.md). New blocking input and output uses `lerux-cothread`. Leave `lerux-service-async` in place for postcard servers.

- The root cothread is the Microkit kernel thread. Only the root calls `microkit_cothread_recv_ntfn`, and it calls that from `Handler::notified`. A worker blocks in `microkit_cothread_wait_on_channel` or `semaphore_wait`. `semaphore_signal` and `recv_ntfn` switch to the waiter before they return.
- Pass explicit stacks. The minimum is `MIN_STACK_SIZE` (`0x1000`). The smoke stacks are `0x4000`, 16-byte aligned, and live in static storage. There is no guard page.
- A contract break panics: a second init, a bad handle, a wait on the root, `recv_ntfn` from a worker, or destroying a blocked cothread. Spawn of a full pool returns `NULL_HANDLE`.
- Do not panic inside a cothread. Unwind across the context switch is unsafe. Put assertions on the root.
- Host tests take a process lock before they use the runtime. The runtime itself does not lock. A lock held across the switch deadlocks.
- The x86 context switch is Intel syntax. This rustc rejects `.intel_syntax` under `-D warnings`. RISC-V is not implemented.
- `just test-cothread` expects `cothread waiting` and then `cothread resumed`. Keep `lerux shell ready` out of that protection domain.

## Filesystem image (`qemu_virt_aarch64_fs_sddf`)

Read Milestones 4, 6, and 9 in [docs/plan-lionsos.md](docs/plan-lionsos.md). The postcard `fs-server`, `virtio-blk-driver`, and LERUXFS2 stay. The workstation shell in `userspace/pds/shell` stays.

- Protection domains are `blk_driver`, `serial_driver`, `blk_virt`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `timer_driver`, and `shell`. Program names use hyphens (`sddf-blk-driver.elf`, `sddf-timer-driver.elf`, `sddf-shell.elf`). `sddf-fs-client` remains a workspace member. The image omits it because the shell already uses the one filesystem client queue.
- Block and filesystem virtual addresses start at `0x4_000_000`. Serial queues stay at `0x3_000_000` through `0x3_007_000`. The UART stays at `0x2_000_000`. Virtio memory-mapped input/output stays at `0x6_000_000_000` and the virtio direct-memory-access region stays at `0x8_000_000_000`.
- `io_or_offset` is a byte offset into the shared data region. `device_region_resource_t.io_addr` stays 0. Microkit 2.2.0 assigns RAM physical addresses at load time, and this driver does not program that field.
- The QEMU user-net device has no driver. It occupies the first virtio-mmio slot so virtio-blk stays at `+0xc00` in the page at `0xa003000`, interrupt 78.
- `blk_driver` and `fatfs` map the data region. `blk_virt` forwards one client's queue entries and does not copy bytes. The partition is 0. The shell maps the four serial client regions and the three filesystem regions. It leaves the UART, the virtio memory-mapped input/output, and the block data region unmapped.
- Copy `blk_virt_config_t` once into a static. It is several kilobytes. Do not put it on the stack.
- The File Allocation Table code lives in `sddf-fatfs`. Do not depend on `lerux-fat`. That crate depends on `lerux-interface-types`. A path is one 8.3 component in the root, or an optional leading slash and exactly two 8.3 components opened read-only. `.`, an empty path, and `/` name the root. A nested create or write is rejected. After format, and on a volume that is already formatted, `fatfs` creates root directory `CONFIG` and file `HOSTNAME` with the bytes `lerux` when that file is missing. One cluster per file. A write starts at offset 0 and is at most 512 bytes.
- The shell and `fatfs` block in a cothread. Assertions stay on the root. Do not add `lerux-logging` or `lerux-interface-types` to `sddf-shell`. `just test-fs-sddf` expects `lerux shell ready` from `serial_enqueue`. Keep that substring out of `expect` and `assert` messages. Golden strings live in `support/smoke-expects.toml`. The QEMU smoke writes each script step to the guest serial after the boot line.
- The one-shot init lines (`serial_driver: pl011`, `blk_driver: ready`, the virtualiser ready lines, and `fatfs: serving`) print before `lerux shell ready`. Leave them.
- Block queue `head` and `tail` stay plain `u32`. The consumer acquire-loads the index the other side writes. The producer release-stores the index it owns. The flexible array starts at `size_of` of the fixed prefix. Enqueue and dequeue do not read `plugged`. The field stays because the header has it. Capacity is 16, and the queue region is one page (`0x1000`). Enqueue returns -1 when the ring is full.
- `fs_completion_enqueue` zeros the `fs_msg_t` slot before it writes `fs_cmpl_t`. The completion is smaller than the command. Writing the completion alone leaves the rest of the slot uninitialized.
- `VirtIOBlk::capacity` counts 512-byte sectors. Transfer block N is virtio sector `N * 8`. Flush and barrier return success with `success_count` 0. They do not submit a virtio flush.
- Priorities and budgets are `blk_driver` 4 and 4000, `serial_driver` 4 and 4000, `blk_virt` 3 and 3000, `serial_virt_tx` 3 and 3000, `serial_virt_rx` 3 and 3000, `fatfs` 2 and 2000, `timer_driver` 5 and 5000, and `shell` 1 and 1000. The period equals the budget. `timer_driver` is passive. The two block and serial drivers share priority 4. Their interrupts differ: serial interrupt 33 and block interrupt 78, each bound to channel id 0 on its own driver. Notifying a higher-priority protection domain runs it before the caller reaches `wait_on_channel`. The channel semaphore sticks, so that signal is still there at the wait.
- `lerux disk-img` writes a 4 MiB raw `support/disk.img` with `55 AA` at byte 510. The bytes-per-sector field is 0. The guest formats the volume when the signature is missing or the sector size is not 512. An already formatted image keeps old files, so a local `ls` can show names a fresh continuous integration disk does not have. The smoke expect is a substring. Do not treat that host image as a File Allocation Table.
- `stage_zero` flushes the current transfer block, then zeros the next one. The second table header is written at byte 512 of transfer block 4, which is sector 33. The volume is 512-byte sectors, one sector per cluster, 8192 sectors, 1 reserved sector, 2 tables of 32 sectors, and 512 root entries. Sector 1 is the first table, 33 the second, 65 the root, and 97 the first data sector (cluster 2). Sector S sits in transfer block `S/8` at byte `(S % 8) * 512`. The 4096-byte transfer buffer is a static cell.
- The server handles open, close, write, read, stat, rename, remove, mkdir, directory open, directory read, and directory close. Open accepts create plus write-only, or read-only. A second create of the same name returns `FS_STATUS_ALREADY_EXISTS`. Creating a file marks its cluster in both tables. One file is open, and its server descriptor is 0. One directory is open, and its server descriptor is 1. A second open of that kind returns `FS_STATUS_TOO_MANY_OPEN_FILES`. `FILE_OPEN` of a directory returns `FS_STATUS_ERROR`. A directory read that finishes returns `FS_STATUS_END_OF_DIRECTORY`. Leave that status off the debug log. Logging it writes the UART while the serial driver owns the device. A name that starts with `0x00` ends the directory. Skip `0xE5`, `.`, and `..`. `rm` of a directory that still holds another name returns `FS_STATUS_NOT_EMPTY`. Otherwise it marks the slot `0xE5` and frees the cluster. `rename` rewrites the 11-byte root name. The destination must be absent. Root `stat` is inode 1, size 0, and a directory. A file or directory inode is its root index plus 2. Directory mode is `0o040000` and regular-file mode is `0o100000`, each combined with `0o777`.
- Shell channels are transmit 0, receive 1, filesystem server 2 (`FS_SHELL_SERVER_CHANNEL`), and timer 3 (`SHELL_TIMER_CHANNEL`, `pp="true"`). `fatfs` notifies its channel 1. `client_config()` stays channel 0 for the unbooted `sddf-fs-client`. The shell closes each descriptor before the next command. A line is at most 80 bytes. Success lines are `mkdir ok`, `write ok`, `mv ok`, `rm ok`, `ls ok`, `stat size N`, and `stat dir`. `cat` prints the file bytes and a newline. `ls` prints each display name on its own line, sorted by bytes, and then `ls ok`. An empty directory prints `ls ok`. More than 32 names, or a name longer than 12 bytes, prints `ls err`. A failed command prints its name plus `err`. An unknown or empty command prints `err`.
- The board feature `board-qemu_virt_aarch64_fs_sddf` appends the timer client page and then `fs_image_schedule()` to the shell config. `config get hostname` prints the file bytes. Any other key prints `config err`. `date` is a protected call with label `SDDF_TIMER_GET_TIME`. A reply of 0 nanoseconds prints `date err`. `ps` and `qos` print each role as `name priority budget`. `dmesg` prints the 2048-byte shell log and does not record the dump. `reboot` prints `reboot`. The host then sends QMP `system_reset` because the board sets `qemu.qmp`. The worker stack stays `0x4000`. Do not add `lerux-logging`.

## Network image (`qemu_virt_aarch64_net_sddf`)

Read Milestone 7 in [docs/plan-lionsos.md](docs/plan-lionsos.md). The postcard `net-server` stays on the other boards. This image has no `net-server`.

- Protection domains are `net_driver`, `serial_driver`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `net_copy`, and `net_client`. Program names use hyphens (`sddf-net-driver.elf`, `sddf-net-virt-tx.elf`, `sddf-net-virt-rx.elf`, `sddf-net-copy.elf`, `sddf-net-client.elf`).
- Network queues start at `0x5_000_000`. Serial queues stay at `0x3_000_000` through `0x3_007_000`. The UART stays at `0x2_000_000`. Virtio memory-mapped input/output stays at `0x6_000_000_000` and the virtio direct-memory-access region stays at `0x8_000_000_000`.
- Virtio-net is the only virtio device, so it occupies `+0xe00` in the page at `0xa003000`, interrupt 79. That interrupt is channel 0 on `net_driver`. `net_driver_config_t` has no interrupt field.
- The client hardware address is `52:54:00:12:34:56` (`CLIENT_MAC`). That is QEMU's default for the first virtio-net device. The driver asserts the device address equals it. Leave the LionsOS C driver's `52:54:01:00:00:07` unused. `0x52` is unicast.
- `device_region_resource_t.io_addr` is `usize` and stays 0. `net_buff_desc_t.io_or_offset` is `u64`. Convert with `u64::try_from(...).expect("offset fits")` before adding the two. Descriptors carry byte offsets into the shared data region.
- The client maps the four serial client regions, the client receive and transmit queues, the client receive data region, and the transmit data region. It leaves the driver queues, the copier queues, the driver receive data region, the metadata region, the virtio memory-mapped input/output, and the direct-memory-access region unmapped.
- Copy `net_virt_tx_config_t` and `net_virt_rx_config_t` once into a static. Both are larger than the `0x10_000` stack. Do not return those two types by value on the protection-domain stack. `net_driver_config_t`, `net_copy_config_t`, and `net_client_config_t` can be returned by value. The client config file is the serial client config followed by `net_image::client_config`. The serial name field stays `serial_client`.
- `net_virt_rx` calls `net_buffers_init` on the driver free queue with the data region's `io_addr`. The copier calls `net_buffers_init` on the client receive free queue with base 0 and does not notify. The client calls `net_buffers_init` once on its transmit free queue with base 0. The client must leave the receive free queue alone. Do not initialise a queue twice.
- The client writes a transmit frame, dequeues one free descriptor, copies the bytes, sets the length and object index 0, enqueues the active queue, and notifies `net_virt_tx` when the queue asks. The client does not enqueue its transmit free queue. `net_virt_tx` is the side that returns transmit buffers.
- `net_virt_tx` returns an invalid client buffer to the client free queue and notifies the client. A valid buffer's offset gains the region `io_addr` and is enqueued on the driver active queue.
- The driver copies a received packet into the driver receive data region and enqueues it active. When the virtio transmit ring cannot accept a frame, `send_one` returns and the descriptor still goes back to the driver free queue, so that frame is dropped. `send` runs only when `can_send` is true.
- The receive virtualiser keeps a frame for the one client address or the broadcast address. Every other destination, including Internet Protocol version 6 multicast, goes back to the driver free queue. This image has one client, so a broadcast frame is enqueued once.
- The copier copies from the driver receive data region into the client receive data region. An empty client free queue returns the virtualiser buffer and drops the packet. A bad client offset asserts `offset fits`. The virtualiser object index on this image is 0.
- Skip ARM cache maintenance. The copies are cached loads and stores on one core. The direct-memory-access region is only for the virtio device.
- Priorities are `net_driver` 4, `serial_driver` 4, `net_virt_tx` 3, `net_virt_rx` 3, `serial_virt_tx` 3, `serial_virt_rx` 3, `net_copy` 2, and `net_client` 1. Client channels are serial transmit 0, serial receive 1, network receive 2, and network transmit 3.
- The client does not depend on `lerux-logging` or `lerux-interface-types`. It does not enable the smoltcp `alloc` feature. Socket storage is leaked so the socket set can borrow it for the life of the protection domain. `just test-net-sddf` expects `ip 10.0.2.15` and then `ping ok`. Keep those strings, and `lerux shell ready`, out of `expect` and `assert` messages.
- The one-shot lines (`serial_driver: pl011`, `net_driver: ready`, the virtualiser ready lines, and `net_copy: ready`) print before the address. Leave them.
- Dynamic Host Configuration Protocol ignores the first `Deconfigured` event. QEMU user-net configures `10.0.2.15` with prefix 24. The gateway is `10.0.2.2` when the offer names no router. Virtual time advances one millisecond after `30_000` `spin_loop` iterations, then the interface polls once. The address and the ping each stop after `30_000` virtual milliseconds and print `ip err` or `ping err`. Virtual time keeps increasing from the address into the ping.
- Queue capacity is 16. Each buffer is 2048 bytes. Clients store object index 0.

## Fetch image (`qemu_virt_aarch64_fetch_sddf`)

Read Milestones 8 and 10 in [docs/plan-lionsos.md](docs/plan-lionsos.md). This image is the filesystem image plus the network roles. The shell is the one serial client, the filesystem client, and the network client. Postcard `net-server`, `fs-server`, and `tls-proxy` stay on the other boards.

- Protection domains are `blk_driver`, `net_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `net_copy`, and `shell`. The shell program is `sddf-shell.elf`.
- Virtio-net is the first virtio device, at `+0xe00` in the page at `0xa003000`, interrupt 79. Virtio-blk stays at `+0xc00`, interrupt 78. Both drivers map that page at `0x6_000_000_000`. Each driver has its own direct-memory-access region, and both regions use virtual address `0x8_000_000_000`.
- Shell channels are serial transmit 0, serial receive 1, filesystem server 2, network receive 3 (`SHELL_NET_RX_CHANNEL`), and network transmit 4 (`SHELL_NET_TX_CHANNEL`). The ping image keeps network receive 2 and network transmit 3.
- The shell config is the serial client page, the filesystem shell page, then the network shell page. `net_buffers_init` runs once on the transmit free queue with base 0, during init on the root, before the worker starts. Leave the receive free queue alone. Do not initialise a queue twice.
- The board feature `board-qemu_virt_aarch64_fetch_sddf` links smoltcp and `lerux-tls` into the shell. The filesystem-image build of `sddf-shell` does not. Do not add `lerux-logging` or `lerux-interface-types`. There is no `TlsRequest` or `HttpRequest`.
- The worker stack is `0x1_0000` on this image. The Transport Layer Security buffer is 16640 bytes. The heap is 1 MiB. The protection-domain stack attribute stays `0x10_000`. Assertions stay on the root. Do not panic inside the cothread.
- `fetch NAME` brings up Dynamic Host Configuration Protocol, connects to `10.0.2.2` port 8443, and writes the body of `GET /fixture.html`. The success line is `fetch ok`. `just test-fetch-sddf` then cats the file and expects `lerux-http-fixture`. Keep `lerux shell ready`, `fetch ok`, and `lerux-http-fixture` out of `expect` and `assert` messages.
- One socket. A second `socket` while that socket is open returns errno 24. `Sockets::close` aborts the connection and returns the table to empty, so `fetch` and `chat` can each connect in one boot. `drive` can report connected while ciphertext is still in the session. Send that ciphertext before waiting for the next record.
- `calc` prints `calc N`. `backup SRC DST` reads `SRC` and creates `DST` through `lerux-posix`. `source NAME` runs each line of that file. A nested `source` prints `source err`. `cert` loads the baked-in smoke certificate authority and prints `cert ok`. `edit NAME TEXT` creates that file through `lerux-posix`. Create of an existing name is rejected. `chat` connects to `10.0.2.2` port 18080, sends `chat`, and prints `chat ok` when the host echo returns those bytes. The board sets `qemu.tcp_echo`. Keep `calc 9`, `backup ok`, `cert ok`, `edit ok`, and `chat ok` out of `expect` and `assert` messages.
- Priorities are the drivers at 4, the virtualisers at 3, `fatfs` and `net_copy` at 2, and `shell` at 1.
- The one-shot ready lines print before `lerux shell ready`. Leave them.

## Web image (`qemu_virt_aarch64_web_sddf`)

Read Milestone 11 in [docs/plan-lionsos.md](docs/plan-lionsos.md). This image reuses `fetch-sddf.system.template` and the same protection domains as the fetch image. The shell serves `/config/hostname`. Postcard `http-file-browser` and `http-server` stay.

- The board feature `board-qemu_virt_aarch64_web_sddf` enables `net-client` and does not link `lerux-tls`, `lerux-logging`, or `lerux-interface-types`. Do not enable it with `board-qemu_virt_aarch64_fetch_sddf`, `board-qemu_virt_aarch64_browser_sddf`, or `board-qemu_virt_aarch64_fs_sddf`. Those features do not share one build. Channel 3 is the network receive channel here and the timer channel on the filesystem image.
- There is no timer. `qemu.tcp_echo` and `qemu.https_one` stay off. `qemu.net` is `hostfwd`, which maps host port 18080 to guest port 8080. Virtio-net stays at `+0xe00` and interrupt 79. Virtio-blk stays at `+0xc00` and interrupt 78.
- The worker reads `/config/hostname` with `Open::Read` before it listens, then closes the file. The body is those bytes. `fatfs` seeds them as `lerux` with no newline. A failed open prints `web err`.
- `begin_listen` brings up Dynamic Host Configuration Protocol, then `listen` on guest port 8080. The worker prints `web listening` after `listen` returns and before `accept`. The host curls `http://127.0.0.1:18080/` after that line. The response is `HTTP/1.1 200 OK`, `Content-Length`, and `Connection: close`, then the file bytes. `shutdown` calls `tcp.close()` so the finish follows the queued bytes. A failed serve calls `close`, which still aborts. A successful serve leaves the table closed.
- After that response the worker prints `lerux shell ready` and enters the same command loop. The smoke does not send a command. `just test-web-sddf` expects `web listening`. `curl_expect` is `lerux`. Keep `web listening`, `lerux`, and `web err` out of `expect` and `assert` messages.
- The worker stack is `0x1_0000` and the heap is 1 MiB because the feature enables `net-client`. Assertions stay on the root. Priorities match the fetch image.

## Browser image (`qemu_virt_aarch64_browser_sddf`)

Read Milestone 12 in [docs/plan-lionsos.md](docs/plan-lionsos.md). This image is the fetch image plus `gpu_driver` and `gpu_virt`. The shell fetches the fixture and paints through the graphics client queues. Postcard `display-server`, `browser-ui`, `web-content`, and `DisplayRequest` stay.

- Protection domains are `blk_driver`, `net_driver`, `gpu_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `gpu_virt`, `fatfs`, `net_copy`, and `shell`. Programs use hyphens (`sddf-gpu-driver.elf`, `sddf-gpu-virt.elf`, `sddf-shell.elf`).
- The pinned graphics headers have no configuration struct and no magic. Do not invent `gpu_driver_config_t`. The shell config stays the serial client page, the filesystem shell page, and the network shell page.
- Graphics regions start at `0x7_000_000`. Driver events, request, response, and data are `0x7_000_000`, `0x7_001_000`, `0x7_002_000`, and `0x7_003_000`. Client events, request, response, and data are `0x7_004_000`, `0x7_005_000`, `0x7_006_000`, and `0x7_007_000`. Each region is one page. Queue capacity is 16.
- Virtio-gpu is the third virtio-mmio device, at `+0xa00` in the page at `0xa003000`, interrupt 77. Virtio-net stays at `+0xe00` and interrupt 79. Virtio-blk stays at `+0xc00` and interrupt 78. Append `-device virtio-gpu-device` only when `qemu.gpu` is true, after virtio-net and virtio-blk. Do not set `xres` or `yres`.
- The driver maps the virtio memory-mapped input/output at `0x6_000_000_000`, its own 2 MiB direct-memory-access region at `0x8_000_000_000`, the four driver regions, and the client data region. `gpu_virt` maps all eight graphics regions. The shell maps only the four client regions and must not map virtio memory-mapped input/output.
- The device interrupt is channel 0 on `gpu_driver`. Channel 1 reaches `gpu_virt`. The virtualiser's channel 0 reaches the driver and its channel 1 reaches the shell. Shell channel 5 is `SHELL_GPU_CHANNEL`.
- Resource id 0 disables a scanout. The painted resource id is 1. The format is `GPU_FORMAT_B8G8R8A8_UNORM` (value 0). QEMU rejects a scanout whose width or height is below 16, so the pattern is 16 by 16. The pixel bytes are `4c 52 58 ff`.
- `change_resolution` returns the framebuffer borrow. Copy the client pixels, drop the borrow, then flush. Blob commands return `GPU_RESP_ERR_INVALID_PARAMETER`. One resource record. Skip the cursor and the example timer state machine.
- The virtualiser forwards one client and keeps the client's resource ids. It copies a display-info response from the driver data region into the client data region. It rejects blob requests locally. Queue indexes use an acquire load for the other side and a release store for this side.
- The board feature `board-qemu_virt_aarch64_browser_sddf` links smoltcp and `lerux-tls`. Do not enable it with `board-qemu_virt_aarch64_fetch_sddf`, `board-qemu_virt_aarch64_web_sddf`, or `board-qemu_virt_aarch64_fs_sddf`. Do not add `lerux-logging` or `lerux-interface-types`.
- The worker fetches the fixture, prints `title lerux`, then paints, then prints `pixel 4c5258ff`, then `lerux shell ready`. A failed title prints `title err`. A failed flush prints `pixel err`. Write the pixels after display info returns. Both use client data offset 0. The worker stack is `0x1_0000` and the heap is 1 MiB. Assertions stay on the root. Do not panic inside the cothread.
- The root installs the client queues and calls `microkit_cothread_recv_ntfn` for channel 5. The worker notifies that channel and waits. Priorities match the fetch image, with `gpu_driver` at 4 and `gpu_virt` at 3.
- `just test-browser-sddf` expects `title lerux` and then `pixel 4c5258ff`. The smoke does not send a command. Keep `lerux shell ready`, `title lerux`, `pixel 4c5258ff`, `title err`, and `pixel err` out of `expect` and `assert` messages.
- The one-shot ready lines print before the title. Leave them.

## Agent image (`qemu_virt_aarch64_agent_sddf`)

Read Milestone 13 in [docs/plan-lionsos.md](docs/plan-lionsos.md). `sddf-shell` stays on the other images. The postcard `agent` protection domain stays. LionsOS 0.4.0 has no agent header. The tool page is `agent_tool_t` in `lerux-sddf`.

- Protection domains are `blk_driver`, `net_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `net_copy`, `shell`, and `agent`. The shell program is `sddf-grok.elf`. The agent program is `sddf-agent.elf`. Neither crate has a board feature.
- The shell maps the four serial client regions and the tool page. The agent maps the three filesystem client regions, the network client queues and data regions, and the tool page. The tool page is `0xa_000_000`, size `0x1_000`, and both map it read-write. The shell embeds the serial client config only. The agent embeds `fs_image::shell_client_config()` and then `net_image::shell_client_config()`.
- Kinds are Read 1, Write 2, Edit 3, ListDir 4, Search 5, WebFetch 6, and Browse 7. The name cap is 64, the argument cap is 128, and the result cap is 256. States are idle 0, browse 1, browse done 2, ok 3, and err 4.
- Shell channels are serial transmit 0, serial receive 1, tool completion 2 (`SHELL_TOOL_CHANNEL`), and browse 3 (`SHELL_BROWSE_CHANNEL`). Agent channels are tool request 0 (`AGENT_TOOL_CHANNEL`), browse 1 (`AGENT_BROWSE_CHANNEL`), filesystem server 2 (`AGENT_FS_CHANNEL`), network receive 3, and network transmit 4. A notify signals the other end.
- The shell worker zeros the page, fills the kind, name, and argument, notifies channel 2, and waits on that channel. The agent worker waits on channel 0, runs the tool, sets ok or err, and notifies channel 0. An error sets `result_len` to 0. The agent has priority 2 and the shell has priority 1, so the agent runs before the shell reaches the wait. The channel semaphore keeps that signal.
- Browse sets state browse and notifies agent channel 1. The shell root answers that notification before it calls `microkit_cothread_recv_ntfn` on the tool channel. It copies the name into the result, or sets `result_len` to 0 when the name length is out of range, sets browse done, and notifies shell channel 3. The agent then sets ok only when the state is browse done and `result_len` is greater than 0. Do not call `microkit_cothread_recv_ntfn` on the browse channel in the shell. The agent calls it on the tool, browse, filesystem, and network channels.
- Write unlinks the name and ignores errno 2, then opens with `Open::CreateWrite`. `fatfs` logs `FS_STATUS_NO_FILE` for that missing name. Read fails when the file is longer than 256 bytes. Edit reads at most 512 bytes and refuses a result longer than 256. List and search keep at most 32 names of 12 bytes and a result of at most 256 bytes. `.`, an empty path, and `/` name the root, so a child path is the bare name.
- WebFetch brings up Dynamic Host Configuration Protocol, connects to `10.0.2.2` port 8443, and reads the body of a `GET` for the given path. The path starts with `/` and contains no space, carriage return, or line feed. The body must fit in 256 bytes. `net_buffers_init` runs once on the transmit free queue with base 0, on the root, before the worker starts. Leave the receive free queue alone.
- The agent heap is 1 MiB. The agent worker stack is `0x1_0000`. The shell worker stack is `0x4000`. The protection-domain stack attribute stays `0x10_000`. Assertions stay on the root. Do not panic inside a cothread. Do not hold a `Files` borrow across a second `files()` call. Do not add `lerux-logging` or `lerux-interface-types`.
- `grok write NAME TEXT` prints `agent: tool write ok`. The other successful tools print the result bytes. A failed tool prints `agent: tool err`. A line is at most 80 bytes.
- Priorities are the drivers at 4, the virtualisers at 3, `fatfs`, `net_copy`, and `agent` at 2, and the shell at 1.
- `just test-agent-sddf` expects `agent: tool write ok` and then `hello-agent`. Keep those strings, `lerux shell ready`, and `agent: tool err` out of `expect` and `assert` messages. The one-shot ready lines print before `lerux shell ready`. Leave them.

## File-descriptor library (`lerux-posix`)

Read Milestones 5, 6, 8, 11, and 13 in [docs/plan-lionsos.md](docs/plan-lionsos.md). The library speaks the filesystem queue and, on the fetch, web, browser, and agent images, the socket calls. The shell on `qemu_virt_aarch64_fs_sddf` is the guest file caller. The shell on `qemu_virt_aarch64_web_sddf` is the guest listener. The shell on `qemu_virt_aarch64_browser_sddf` reuses the fetch socket calls. The agent on `qemu_virt_aarch64_agent_sddf` is the guest file and socket caller. The shell on that image is only the serial client.

- Paths are copied as given. Do not add a leading slash.
- Descriptors 0, 1, and 2 are not stored. The first `open` returns 3. Sockets are a separate table. The first `socket` also returns 3. A second `socket` while one is open returns errno 24. `listen` on an idle socket binds one port and keeps that same socket. `accept` waits until the peer is connected. A default `Tcp::listen` returns errno 95. A listen the stack rejects as already bound returns errno 98. `shutdown` finishes an open socket and leaves the table closed. `close` aborts and returns that table to empty. `send` before `connect` or `accept` returns errno 107. A second `connect` returns errno 106.
- A second call while a command is in flight returns errno 16.
- `FS_STATUS_NO_FILE` maps to errno 2. `FILE_ERR` and `EPERM` are both 1.
- `open_dir`, `read_dir`, and `close_dir` use the directory commands. `read_dir` returns `Ok(None)` for `FS_STATUS_END_OF_DIRECTORY` before that status is mapped. The status map would turn it into errno 1.
- `cargo test -p lerux-posix` is the host check. Keep `lerux shell ready` out of that crate.

## Host tooling (`tools/lerux-cli/**`)

- Use `anyhow::Result` at the CLI boundary; add context with `.context("…")?`.
- Use `clap` derive for subcommands.
- No `unwrap()` or `expect()` in production paths — use `?` or `bail!`.
- `std` only; do not depend on seL4 userspace crates. `lerux-sddf` is allowed: it has no seL4 dependency and is the shared layout for the config bytes the command-line interface checks. Do not depend on a protection-domain crate.

## Quality gates

Run before finishing Rust changes:

```bash
just check
```

This runs `cargo fmt --all --check`, clippy and tests for every crate named in the justfile `check` recipe (including `lerux-sddf`), and `lerux profile check-qos`. After protection-domain or shared userspace crate changes, also run:

```bash
just check-pd
```

`check-pd` runs cross-target clippy on all PD and shared userspace crates (one pass per arch). It requires a built SDK (`just build-sdk`) for `SEL4_INCLUDE_DIRS`. `just check-all` runs both.

PD changes may need a full board build (`just build` or `just build-pd <crate>`) because targets are seL4 cross-compile profiles.

Workspace `[lints]` in the root `Cargo.toml` sets clippy defaults; each crate inherits them via `[lints] workspace = true`.

CI runs `just check` before the SDK pipeline and `just check-pd` after the SDK artifact is ready (in parallel with smoke).

A few results look like success and are not:

- `lerux-cli` is a binary crate. Test it with `cargo test -p lerux-cli -- <filter>`. There is no `--lib` target. `--exact` needs the full path `module::tests::<name>`. A filter matches test function names, not the integration-test file name. Run a file with `cargo test -p <crate> --test <file stem>`. A filter that matches nothing still exits 0; `running 0 tests` is the result.
- Do not run `just check`, `just check-pd`, and `just test-*` together. `just check` uses `build/host`. `just check-pd` and a smoke share `build/target`, and both invoke `cargo run -p lerux-cli` in the default target directory.
- Update the milestone status, the board row, and the smoke counts in `docs/ci.md` and the README only after `just check`, `just check-pd`, and that milestone's smokes have passed. The workflow `include` list and those hand-written counts move together. Leave the historical job counts inside `docs/plan.md` as they were.
- Lint protection domains with `just check-pd`. That command sets up libclang. `cargo clippy` on a PD target without it panics inside bindgen.
- `ci = true` in `support/boards.toml` is the set `just test-all` runs. GitHub smoke is the `include` list in `.github/workflows/rust.yml`. The job count in `docs/ci.md` and the README is written by hand. A new board name is passed as `--features board-<name>` only when that key exists in the PD `Cargo.toml`.
- Take a QEMU `-device` property from that binary's `-device <name>,help`. The host kernel's interrupt mode is a different machine. `just test-x86-usb-kbd` covers qemu-xhci and QEMU's `usb-kbd`. The desk keyboard and the Intel xHCI path are the metal boot in `docs/boards.md`.
- The exit status of a shell string is its last command. A redirected recipe that ends in `echo` reports that echo. Read the log for the recipe's own status line.

## What not to do

- Do not introduce `std` into PD crates.
- Do not modify seL4 or Microkit sources under `deps/workspace/`.
- Do not add vendored rust-sel4 trees — use workspace dependencies.
- Do not "clean up" unrelated code, normalize experiments, or rewrite git history unless asked.

## Cursor Cloud specific instructions

The VM snapshot already has the full dev environment: system packages (cmake, ninja, dtc, xmllint, `qemu-system-{arm,x86,misc}`, libclang, glib/pixman for SP804 QEMU), the pinned Rust nightly, `just`, the prebuilt Microkit SDK, and the cross toolchains. Do not reinstall these; the startup update script only runs `cargo fetch --locked`.

- **SDK**: obtained via `just fetch-sdk` (prebuilt SDK 2.2.0 at `deps/microkit-sdk/`, path in `deps/.sdk-path`), not `just build-sdk`. It covers the `qemu_virt_aarch64`, `qemu_virt_riscv64`, `x86_64_generic`, and `rpi4b_4gb` boards used by the smoke matrix. All of `deps/workspace/`, `deps/microkit-sdk/`, `deps/toolchains/`, `deps/.sdk-path` are gitignored but persist in the snapshot. If the SDK is ever missing, re-run `just fetch && just fetch-sdk`.
- **Cross toolchains auto-install**: `just build`/`just test` auto-download the ARM (`aarch64-none-elf`) and RISC-V toolchains into `deps/toolchains/` on the first PD build if absent — no manual ARM toolchain step needed.
- **Commands** are documented in the `README.md`/`justfile`: `just check` (host lint, no SDK), `just check-pd` (cross clippy, needs SDK), `just test` (default aarch64 hello smoke in QEMU). Other boards via `BOARD=... just test` or the `just test-*` recipes.
- **Caveat**: `init`/`composed`/`*-composed` boards need patched SP804 QEMU — run `cargo run -p lerux-cli -- install sp804-qemu` first (build deps are already installed). Not required for the default hello/echo/virtio/http flows.

## Further reading

**General idiomatic Rust**

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — official naming, error, and API design
- [Apollo Rust Best Practices](https://github.com/apollographql/rust-best-practices) — practical idioms and lint discipline
- [Rust Design Patterns — Idioms](https://rust-unofficial.github.io/patterns/idioms/)
- [cheats.rs](https://cheats.rs/) — concise ownership, string, and error tips
- [Clippy](https://doc.rust-lang.org/clippy/) — machine-enforced idioms
- [idiomatic-rust index](https://corrode.dev/idiomatic-rust/) — curated article list

**lerux / embedded context**

- [Rust on seL4](https://docs.sel4.systems/projects/rust/)
- [rust-sel4 API docs](https://sel4.github.io/rust-sel4/)
- [Embedded Rust Book — no_std](https://docs.rust-embedded.org/book/intro/no-std.html)
- [High Assurance Rust](https://highassurance.rs/) — firmware-oriented patterns