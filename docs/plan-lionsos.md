# Lerux in Rust, on the LionsOS architecture

Last updated: 2026-10-27

**Status:** Milestone 1 is done and is the shared-structure crate. Milestone 2 is done and is the serial-role image. Milestone 3 is done and is the cothread runtime. Milestone 4 is done and is the block and filesystem image. Milestone 5 is done and is the file-descriptor library. Milestone 6 is done and is the shell on that filesystem image. Milestone 7 is done and is the network-role image. Milestone 8 is done and is the socket and fetch image. Milestone 9 is done and is the timer and static status on the filesystem image. Milestone 10 is done and is the remaining shell commands on the fetch image. Milestone 11 is done and is the web image that serves the seeded hostname file. Milestone 12 is done and is the virtio graphics driver, virtualiser, and browser image. Milestone 13 is done and is the agent image. Milestone 14 is done and is the WebAssembly runtime protection domain. Milestone 15 is done and is package selection from the template. Milestone 16 is done and is the same protection-domain roles on QEMU RISC-V and QEMU x86. Milestone 17 is done and is the removal of the postcard guest model. Milestone 18 is done and is a real paint of the fetched fixture on the browser image. Milestone 19 is done and is the Advanced Host Controller Interface disk. Milestone 20 is done and is the QEMU 82574. Milestones 21 and 22 are not started. They put that disk and that Ethernet device on one q35 image, then bind those drivers on the Z97. Milestones run in order. Implement one milestone per pass, after the previous smoke is green.

**Goal:** Every lerux guest protection domain stays Rust, and the queues, configuration pages, filesystem messages, and component graph are the same ones LionsOS uses.

**Architecture:** Treat the C headers in LionsOS 0.4.0 and the seL4 Device Driver Framework it pins as a specification, not as code to link. Reproduce those structs with `#[repr(C)]` and check their size and offsets against the headers. Split devices into the same single-purpose protection domains LionsOS composes: driver, transmit virtualiser, receive virtualiser, per-client network copier, block virtualiser, and a filesystem server speaking the LionsOS command queue. Applications call a Rust library with the same file-descriptor and socket model as the LionsOS file and socket library. The Internet Protocol stack behind that socket model is smoltcp, one stack per client. Blocking calls use a Rust cothread runtime with the same operations as `libmicrokitco`.

**Tech stack:** Existing Rust protection domains, `#![no_std]`, rust-sel4, Microkit. New guest crates are Rust. The C headers are compiled only by a host layout test. Python sdfgen, musl, MicroPython, lwIP, libvmm, and the WebAssembly Micro Runtime are not linked.

**Decision:** [011-lionsos-structures.md](decisions/011-lionsos-structures.md). Decision records 001 through 004 and 008 still describe the postcard image. They do not govern new device or filesystem code.

## Global constraints

- Specification checkout: LionsOS tag `0.4.0`, commit `ad4c35ea7ae657d2f450e45c821035e746cfa48a`. Its `dep/sddf` submodule is commit `650ace5dba984f5d9c7dd1a005648ff7b40655f7`, which `git describe --tags` names `0.7.0`. The local au-ts mirror is behind that tag. Do not copy struct layouts from the mirror.
- `lerux fetch` clones that LionsOS tag and initialises only `dep/sddf`. It does not initialise the other submodules. Those are C components and language runtimes, and this checkout does not build them. Widen that checkout only when a later milestone needs a header that is not already there. A cached checkout fetches tags with `--no-recurse-submodules`. Fetching every tag otherwise asks the sddf remote for a commit recorded by older LionsOS history, and that remote rejects a commit it no longer has.
- Guest code stays `#![no_std]` Rust. It includes no C. It does not call musl, lwIP, MicroPython, or the WebAssembly Micro Runtime.
- Shared structs use `#[repr(C)]`. A host test fails when `size_of` or `offset_of` disagrees with the header. `fs_cmd_t` and `fs_msg_t` are 64 bytes, which the header asserts.
- Queue indexes use the same acquire and release pairing as the header comments. The producer is the only writer of the tail. The consumer is the only writer of the head. Use that pairing on every new queue, which is the symmetric-multiprocessing branch of the header, including on a single processor.
- A new guest component discovers its queues from a configuration memory region whose magic and fields match the header (`serial_driver_config_t`, `serial_virt_tx_config_t`, `serial_virt_rx_config_t`, `serial_client_config_t`, and the network, block, and filesystem equivalents). A handwritten `Channel::new` is not the protocol. Milestone 17 removed the postcard image. `hello` and `serial-driver` remain.
- Postcard request types are not added. An old protection domain stays on postcard only until the milestone that replaces it deletes it.
- Do not modify `deps/workspace/` by hand. Do not vendor the C trees into git.
- The installed kit is Microkit 2.3.0, seL4 16.0.0, and rust-sel4 `v5.0.0`. Those three pins move together. rust-sel4 5.0.0 is the release that supports this kit. Queue layouts do not depend on the kit version.
- A missing header in the 0.4.0 checkout stops that milestone. The struct is not invented to keep moving.
- Milestones 2–15 are QEMU AArch64. Full terms in new docs and commit messages.

## What "the same" means

| LionsOS | Rust in lerux |
| --- | --- |
| `serial_queue_t` (`tail`, `head`, `producer_signalled`) and `serial_queue_handle_t` | Same fields, same enqueue and dequeue rules, in `lerux-sddf` |
| `net_buff_desc_t`, `net_queue_t`, `net_queue_handle_t` | Same fields. `oid` stays a 6-bit field. Clients store `0` there. |
| `blk_req_t`, `blk_resp_t`, `blk_req_queue_t`, `blk_resp_queue_t` | Same fields |
| `fs_cmd_t`, `fs_cmpl_t`, `fs_queue_t`, capacity 511, status codes `FS_STATUS_*`, commands `FS_CMD_*` | Same fields and numeric values |
| `serial_*_config_t` with magic `sDDF\x03` | The system generator writes these bytes into a mapped configuration region |
| Driver, `serial_virt_tx`, `serial_virt_rx`, clients | Four roles. One protection domain does not both drive the serial device and multiplex clients. |
| Network driver, `net_virt_tx`, `net_virt_rx`, one copier per client, one Internet Protocol stack per client | Same roles. The stack is smoltcp. lwIP is not ported. |
| Block driver, `blk_virt`, filesystem server, client | Same roles. The server speaks `fs_queue_t`. The on-disk format is File Allocation Table, implemented in Rust. |
| `libmicrokitco`: `init`, `spawn`, `yield`, `wait_on_channel`, semaphore wait and signal | `lerux-cothread`, stackful, same operations. `lerux-service-async` is not used for new blocking input and output. |
| File and socket calls mapped onto the filesystem queue and the client stack | `lerux-posix`: file-descriptor table, `open`/`read`/`write`/`socket`, filesystem status codes mapped to the same `errno` values as `lib/libc/posix` at the pinned tag |
| sdfgen metaprogram and `serialise_config` | The Rust system generator emits the same protection-domain set and writes the same configuration structs. Python sdfgen is not added. |
| WebAssembly runtime protection domain with filesystem and network access | A Rust WebAssembly runtime in that role. The `LRW1` opcode allowlist is removed. A signature check before instantiate may stay. |
| `gpu_req_t`, `gpu_resp_t`, `gpu_events_t`, and the queue prefixes | Same fields. The pinned graphics headers have no configuration struct and no magic. |
| Kitty Linux guest framebuffer | Not ported. That path is a Linux guest, and the LionsOS header calls it unprincipled. Display uses the driver-framework virtio graphics class, in Rust, on `qemu_virt_aarch64_browser_sddf`. |

LERUXFS2, the combined `net-server`, the postcard `serial-virt`, and `config-server` as its own protocol are the divergences this plan removes.

## Rules for later milestones

1. Copy the struct from the pinned header. Do not rename fields. Do not drop a field because the current lerux code does not use it.
2. One protection domain, one role from the table above.
3. An untrusted client maps its own queue and data region. It does not map device registers or another client's data region.
4. The milestone that makes the new smoke pass also deletes the postcard protection domain it replaced, when that milestone's exit says so. Milestone 2 keeps the postcard serial path because the shell has not moved.
5. A feature with no counterpart becomes a Rust program on `lerux-posix` and the queues. It does not grow a new message crate.

## Milestone 1 — Shared structs that match the headers

Done. No QEMU behavior change. The legacy `just test` smoke still passes.

Landed:

- `userspace/crates/lerux-sddf` holds `serial_queue_t`, `serial_queue_handle_t`, the four `serial_*_config_t` structs, `net_buff_desc_t`, `net_queue_t`, `net_queue_handle_t`, `blk_req_t`, `blk_resp_t`, the block queues, `fs_cmd_t`, `fs_cmpl_t`, `fs_queue_t`, and the `FS_CMD_*` and `FS_STATUS_*` constants. `FS_QUEUE_CAPACITY` is 511. `SDDF_SERIAL_MAGIC` is the five bytes `s`, `D`, `D`, `F`, `0x03`.
- `net_queue_t` and `blk_req_queue_t` use `#[repr(C, align(8))]`. The C types end in a flexible array whose element alignment is 8. The array itself is not part of the Rust prefix.
- `oid` in `net_buff_desc_t` is the low 6 bits of its byte. The layout test compares a byte image from the host C compiler, because `offsetof` on a bit-field is not the check.
- Serial operations: `serial_enqueue`, `serial_dequeue`, `serial_enqueue_local`, `serial_update_shared_tail`, `serial_update_shared_head`, and the consumer-signal helpers. Filesystem operations are the queue half of `fs_command_issue`: enqueue one `fs_msg_t` and dequeue one message. Notification and request-id allocation stay with the caller. Serial colour and batch transfer are not ported.
- `tests/layout.rs` compiles the fetched headers with the host `cc` and compares `sizeof`, `_Alignof`, and `offsetof`. When the headers are absent the test prints that `lerux fetch` is required and passes, so `just check` can run before fetch. Continuous integration runs `just check` before `just fetch`, so that job does not compare layouts.
- Checked on 2026-10-04: `cargo test -p lerux-cli -- parses_lionsos` passed, `lerux fetch` checked out the tag and both header paths, `cargo test -p lerux-sddf` passed, `just check` passed, `just check-pd` passed, and `just test` printed `lerux: Hello from Rust on seL4 Microkit!`.

## Milestone 2 — Serial roles

**Exit:** `just test-serial-sddf` boots a QEMU AArch64 image whose protection domains are only `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, and one client. The client writes `lerux shell ready` by `serial_enqueue` into the queue described by `serial_client_config_t`. The driver is the only protection domain that maps the serial device. The configuration regions start with the serial magic. The legacy echo smoke still passes.

**Files:** `userspace/pds/sddf-serial-driver`, `userspace/pds/sddf-serial-virt-tx`, `userspace/pds/sddf-serial-virt-rx`, `userspace/pds/sddf-serial-client`, a system template, and a generator function that fills `serial_driver_config_t`, `serial_virt_tx_config_t`, and `serial_virt_rx_config_t`.

The existing `serial-driver` and `serial-virt` postcard path stay until a later workstation milestone switches the shell and deletes them.

Done.

Landed:

- Protection domains `sddf-serial-driver`, `sddf-serial-virt-tx`, `sddf-serial-virt-rx`, and `sddf-serial-client`. The system names are `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, and `serial_client`. Only `serial_driver` maps the serial device.
- `lerux_sddf::serial_image` fills `serial_driver_config_t`, `serial_virt_tx_config_t`, `serial_virt_rx_config_t`, and `serial_client_config_t`. Each protection domain build script embeds those bytes. The installed Microkit kit cannot prefill a memory region, so the bytes live in the program image and the protection domain copies them at start.
- Board `qemu_virt_aarch64_serial_sddf`, template `serial-sddf.system.template`, recipe `just test-serial-sddf`. The continuous integration job is `serial-sddf`.
- The client writes `lerux shell ready` with `serial_enqueue`.
- Checked on 2026-10-04: `just check` passed, `just check-pd` passed, `just test-serial-sddf` printed `lerux shell ready`, and `just test-echo` passed.

## Milestone 3 — Cothreads

**Exit:** `cargo test -p lerux-cothread` spawns two cothreads, blocks one in `wait_on_channel`, runs the other, signals the channel, and observes the blocked thread resume. A protection-domain smoke on QEMU does the same wait against a Microkit notification. The operations match `microkit_cothread_init`, `spawn`, `yield`, `wait_on_channel`, `semaphore_wait`, and `semaphore_signal`. Stacks are explicit. This crate replaces `lerux-service-async` for new blocking input and output.

Done.

Landed:

- `userspace/crates/lerux-cothread` implements `microkit_cothread_init`, `spawn`, `yield`, `wait_on_channel`, `semaphore_wait`, and `semaphore_signal`. Each worker gets a caller-provided stack. The pool holds 8 cothreads, including the root kernel thread. A contract break panics, which is where `libmicrokitco` faults the protection domain. A full pool returns `NULL_HANDLE`.
- `cargo test -p lerux-cothread` spawns a waiter and a runner. The waiter blocks in `wait_on_channel`. The runner signals that channel. The waiter resumes. A second test covers `semaphore_signal`.
- Board `qemu_virt_aarch64_cothread`, template `cothread.system.template`, protection domains `cothread_client`, `cothread_peer`, and the postcard `serial_driver`. Recipe `just test-cothread`. The continuous integration job is `cothread`.
- The worker logs `cothread waiting`, notifies the peer, and blocks on that channel. The peer notifies back. The client's `notified` calls `microkit_cothread_recv_ntfn`, and the worker logs `cothread resumed`.
- `lerux-service-async` stays for postcard servers that already poll one future. The postcard serial path stays.
- Checked on 2026-10-04: `cargo test -p lerux-cothread` passed, `just check` passed, `just check-pd` passed, and `just test-cothread` printed `cothread waiting` and then `cothread resumed`.

## Milestone 4 — Block and the filesystem queue

**Exit:** `just test-fs-sddf` has `blk_driver`, `blk_virt`, `fatfs`, and one client. The client blocks in a cothread on `FS_CMD_FILE_OPEN`, `FS_CMD_FILE_WRITE`, and `FS_CMD_FILE_READ`. The bytes read back match the bytes written. Messages are `fs_cmd_t` and `fs_cmpl_t`. The on-disk format is File Allocation Table. LERUXFS2 is not mounted by this image.

Done.

Landed:

- Protection domains `sddf-blk-driver`, `sddf-blk-virt`, `sddf-fatfs`, and `sddf-fs-client`. The system names are `blk_driver`, `blk_virt`, `fatfs`, and `fs_client`.
- `lerux_sddf::blk_image` and `lerux_sddf::fs_image` fill the block and filesystem configs. Each protection domain build script embeds those bytes. `tools/lerux-cli/src/fs_sddf.rs` writes the same bytes so a host test can compare them with the rendered system description.
- Board `qemu_virt_aarch64_fs_sddf`, template `fs-sddf.system.template`, recipe `just test-fs-sddf`. The continuous integration job is `fs-sddf`.
- The client blocks in a cothread, opens `SMOKE.TXT`, writes 15 bytes, reads them back, and logs `fs-sddf read ok`. The on-disk format is a 16-bit File Allocation Table in `sddf-fatfs`. One file, one cluster, at most 512 bytes.
- The driver and the filesystem server share the data region. The block virtualiser forwards queue entries and does not copy bytes. `device_region_resource_t.io_addr` stays 0. A user-net device with no driver holds the first virtio-mmio slot so virtio-blk stays at `+0xc00`.
- The postcard `fs-server`, `virtio-blk-driver`, and LERUXFS2 stay.
- Checked on 2026-10-05: `just check` passed, `just check-pd` passed, and the `just test-fs-sddf` serial log printed `fs-sddf read ok`.

When this milestone landed, the server accepted open, write, and read. Any other command returned `FS_STATUS_INVALID_COMMAND`. Milestone 6 widened that command set. These image rules still hold. Current agent rules are in [AGENTS.md](../AGENTS.md#filesystem-image-qemu_virt_aarch64_fs_sddf).

- `lerux disk-img` writes a 4 MiB raw image. The guest formats it because the bytes-per-sector field is 0.
- Flush and barrier return success and do not submit a virtio flush.

## Milestone 5 — File-descriptor library

**Exit:** `lerux-posix` on the host, with a fake filesystem queue, implements `open`, `read`, `write`, `mkdir`, `unlink`, `rename`, and `stat`. Status codes map to the `errno` values in the pinned `lib/libc/posix`. `MAX_FDS` defaults to 128. Descriptors 0, 1, and 2 are reserved. `cargo test -p lerux-posix` covers a successful read and `FS_STATUS_NO_FILE`.

Done. No QEMU behavior change.

Landed:

- `userspace/crates/lerux-posix` implements `open`, `read`, `write`, `mkdir`, `unlink`, `rename`, `stat`, and `close` against a fake filesystem queue. The library is `no_std` and depends only on `lerux-sddf`.
- `MAX_FDS` is 128. Descriptors 0, 1, and 2 are not in the table. The first `open` returns 3.
- Paths are copied into the share as given. A length above 4095 returns errno 36 and sends no command.
- Status codes use the mapping in the pinned `lib/libc/posix`. `FS_STATUS_NO_FILE` is errno 2. `FS_STATUS_ERROR` and `FS_STATUS_SERVER_WAS_DENIED` are both errno 1.
- The host test drives real `fs_queue_t` rings. The Milestone 4 protection domains are unchanged.
- Checked on 2026-10-05: `cargo test -p lerux-posix` passed, `just check` passed, and `just check-pd` passed.

## Milestone 6 — Shell file commands

**Exit:** The scripted serial session on the Milestone 4 image runs `mkdir`, `write`, `cat`, `ls`, `mv`, `rm`, and `stat` through `lerux-posix`. Golden strings live in `support/smoke-expects.toml`. The shell protection domain does not depend on `lerux-interface-types`.

Shell commands this plan keeps: `ls`, `cat`, `write`, `mkdir`, `rm`, `mv`, `cd`, `pwd`, `stat`, `df`, `echo`, `help`, `history`, `clear`, `date`, `uptime`, `ip`, `ping`, `fetch`, `config`, `hostname`, `cert`, `backup`, `calc`, `source`, `run`, `reboot`, `dmesg`, `ps`, `status`, `qos`.

Done.

Landed:

- The filesystem image boots `blk_driver`, `serial_driver`, `blk_virt`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, and `shell`. The program name is `sddf-shell.elf`. The shell is the filesystem client. `sddf-fs-client` remains a workspace member. The image omits it because the shell already uses the one filesystem client queue.
- Block and filesystem virtual addresses start at `0x4_000_000`. The serial queues stay at `0x3_000_000`.
- The shell calls `lerux-posix`. Its dependencies are `lerux-cothread`, `lerux-posix`, `lerux-sddf`, and `sel4-microkit`. It writes `lerux shell ready` with `serial_enqueue`. Transmit is channel 0, receive is channel 1, and the filesystem server is channel 2.
- `fatfs` handles open, close, write, read, stat, rename, remove, mkdir, directory open, directory read, and directory close. A path is one 8.3 component in the root. `.` names the root. One file and one directory can be open. A write starts at offset 0, uses one cluster, and is at most 512 bytes.
- `support/smoke-expects.toml` scripts `mkdir`, `write`, `cat`, `ls`, `mv`, `rm`, and `stat`. The QEMU smoke writes each step to the guest serial after the boot line. The continuous integration job stays `fs-sddf`. The smoke count stays 47.
- A finished directory read returns `FS_STATUS_END_OF_DIRECTORY`. `fatfs` leaves that status off the debug log. Logging it writes the debug UART while the serial driver owns the device.
- The postcard `fs-server`, `virtio-blk-driver`, LERUXFS2, and the workstation shell stay.
- Checked on 2026-10-05: `just check` passed, `just check-pd` passed, and `just test-fs-sddf` printed `lerux shell ready` and the nine scripted steps.

## Milestone 7 — Network roles

**Exit:** The image adds `net_driver`, `net_virt_tx`, `net_virt_rx`, one copier, and a client that owns smoltcp. The client does not map the device queue. After Dynamic Host Configuration Protocol on QEMU user-net, `ip` prints an address and `ping` of the gateway gets a reply. There is no `net-server` in this image.

Done.

Landed:

- The network image boots `net_driver`, `serial_driver`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `net_copy`, and `net_client`. Program names use hyphens (`sddf-net-driver.elf`, `sddf-net-client.elf`). The client owns smoltcp. This image has no `net-server`. The postcard `net-server` stays on the other boards.
- Network queues start at `0x5_000_000`. The client maps its own queues and the transmit data region. It leaves the driver queues, the copier queues, the driver receive data region, and the device unmapped.
- QEMU user-net answers Dynamic Host Configuration Protocol with `10.0.2.15`. The client prints that address and then `ping ok` after the gateway replies.
- `support/smoke-expects.toml` expects those two strings. The continuous integration job is `net-sddf`. The smoke count is 48.
- Checked on 2026-10-09: `just check` passed, `just check-pd` passed, and `just test-net-sddf` printed `ip 10.0.2.15` and `ping ok`.

## Milestone 8 — Sockets and fetch

**Exit:** `lerux-posix` grows `socket`, `connect`, `send`, and `recv` on that client's smoltcp. `fetch` of the existing host `lerux https-one` fixture writes a file, and `cat` prints it. Transport Layer Security is a Rust library inside the client, or one protection domain in front of it, using those socket calls. There is no `TlsRequest` or `HttpRequest` type.

Done.

Landed:

- `lerux-posix` adds `Sockets` and `Tcp`. The shell on `qemu_virt_aarch64_fetch_sddf` owns smoltcp and `lerux-tls`. `fetch FIX.TXT` writes the body of `GET /fixture.html`. `cat` prints that body. Transport Layer Security uses `socket`, `connect`, `send`, and `recv`. There is no `TlsRequest` or `HttpRequest`.
- The image boots the filesystem roles and the network roles. The shell is the one client of each. Its network channels are 3 and 4. The ping image stays on channels 2 and 3.
- `support/smoke-expects.toml` expects `fetch ok` and then `lerux-http-fixture`. The continuous integration job is `fetch-sddf`. The smoke count is 49.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, and `just test-fetch-sddf` printed `fetch ok` and `lerux-http-fixture`.

## Milestone 9 — Config, time, reboot, static status

**Exit:** `config get hostname` reads `/config/hostname` with `open`. `date` reads the driver-framework timer client. `reboot` resets QEMU and the next log contains `lerux shell ready`. `ps` and `qos` print the protection domains, priorities, and budgets written into the generated configuration. `dmesg` prints serial bytes the shell stored. No `config-server`, `supervisor`, or `log-server` is in the image. The supervisor-only `secret.*` rule is not reimplemented.

Done.

Landed:

- The filesystem image adds `timer_driver`. The program name is `sddf-timer-driver.elf`. It is passive, priority 5, budget 5000, and period 5000. The shell calls it on channel 3 with `pp="true"`. `date` sends `SDDF_TIMER_GET_TIME` and prints `YYYY-MM-DD`. A reply of 0 nanoseconds prints `date err`.
- Every role on this image has a budget, and the period equals that budget. `ps` and `qos` print `name priority budget` from the schedule page the shell embeds.
- `fatfs` creates `/config/hostname` with the bytes `lerux` when that file is missing, including on a volume that was already formatted. `config get hostname` opens that path. Any other key prints `config err`. A nested path is a read-only open of exactly two 8.3 components. A nested create or write is rejected.
- `dmesg` prints the serial bytes the shell stored. The log holds 2048 bytes and stops when it is full, so the boot line stays at the front. The dump itself is not stored.
- `reboot` prints `reboot`. The host then sends QMP `system_reset`. The next boot prints `lerux shell ready`. The board sets `qemu.qmp`.
- The image has no `config-server`, no `supervisor`, and no `log-server`.
- `support/smoke-expects.toml` scripts those six commands after the file commands. The continuous integration job stays `fs-sddf`. The smoke count stays 49.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, and `just test-fs-sddf` printed `lerux`, `1970-01-01`, `shell 1 1000`, `fatfs 2 2000`, `lerux shell ready`, and a second `lerux shell ready` after the reset.

## Milestone 10 — Remaining shell programs

**Exit:** Scripted smokes for `calc`, `backup`, `source`, `cert`, `edit`, and `chat`. `edit` and `backup` use `lerux-posix`. `chat` uses the socket calls. Each smoke has one expected serial string.

Done.

Landed:

- The shell on `qemu_virt_aarch64_fetch_sddf` adds `calc`, `backup`, `source`, `cert`, `edit`, and `chat`. `backup` reads one file and writes another through `lerux-posix`. `edit` creates a file through `lerux-posix`. `source` reads a file and runs each line. `calc` prints `calc N`. `cert` loads the baked-in smoke certificate authority and prints `cert ok`.
- `chat` calls `socket`, `connect`, `send`, and `recv`. It sends `chat` to `10.0.2.2` port 18080. The host echo writes those bytes back. `Sockets::close` returns the one socket to empty so `fetch` and `chat` can each connect in the same boot. A second `socket` while one is still open returns errno 24.
- The board starts the host echo because `qemu.tcp_echo` is true. `support/smoke-expects.toml` expects `calc 9`, `backup ok`, `calc 4`, `cert ok`, `edit ok`, and `chat ok`. The continuous integration job stays `fetch-sddf`. The smoke count stays 49.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, and `just test-fetch-sddf` printed those six strings after `fetch ok`.

## Milestone 11 — File browser

**Exit:** A Rust program on the socket and file libraries serves one seeded file over Hypertext Transfer Protocol. `just test-web-sddf` expects that body in the host response. `http-file-browser` and `http-server` are not ported as postcard clients.

Done.

Landed:

- `lerux-posix` adds `listen`, `accept`, and `shutdown` on the one socket. `listen` binds that socket. `accept` waits for the peer. `shutdown` finishes the connection and leaves the table closed. `close` still aborts and returns the table to empty. A pipe that only connects returns errno 95 from `listen`. A listen the stack rejects as already bound returns errno 98.
- The shell on `qemu_virt_aarch64_web_sddf` reads `/config/hostname` and serves those bytes on guest port 8080. The image reuses `fetch-sddf.system.template` and the fetch protection domains. It does not link `lerux-tls`. Postcard `http-file-browser` and `http-server` stay.
- The worker prints `web listening` after `listen` and before `accept`. The host curls `http://127.0.0.1:18080/` and expects the body `lerux`. `support/smoke-expects.toml` expects `web listening`. The continuous integration job is `web-sddf`. The smoke count is 50.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, and `just test-web-sddf` printed `web listening` and the host curl matched `lerux`.

## Milestone 12 — Display and browser

**Exit:** A Rust virtio graphics driver and virtualiser use the queue and configuration structs from the pinned driver-framework graphics headers. The browser program writes pixels through that client interface and fetches the page through the socket library. `just test-browser-sddf` expects a known pixel pattern and the page title on the serial log. `DisplayRequest` is not ported.

Done.

Landed:

- `lerux-sddf` copies `gpu_req_t`, `gpu_resp_t`, `gpu_events_t`, and the queue prefixes from the pinned graphics headers. Those headers have no configuration struct and no magic. Queue capacity is 16. Graphics regions start at `0x7_000_000`.
- `qemu_virt_aarch64_browser_sddf` adds `gpu_driver` and `gpu_virt` to the fetch protection domains. The programs are `sddf-gpu-driver.elf` and `sddf-gpu-virt.elf`. Virtio-gpu is the third virtio-mmio device, at `+0xa00` and interrupt 77. The driver maps that page and a 2 MiB direct-memory-access region. The shell maps only the four client graphics regions. The shell config stays the serial page, the filesystem page, and the network page.
- The shell fetches `GET /fixture.html` and prints `title lerux`. It then submits display info, a 16 by 16 resource, and a flush. QEMU rejects a scanout whose width or height is below 16. The pixel bytes are `4c 52 58 ff`. The success line is `pixel 4c5258ff`. `DisplayRequest` is not ported. Postcard `display-server`, `browser-ui`, and `web-content` stay.
- Shell channel 5 is the graphics virtualiser. Drivers stay at priority 4 and virtualisers at priority 3. The board feature does not share a build with the fetch, web, or filesystem shell features.
- `support/smoke-expects.toml` expects `title lerux` and then `pixel 4c5258ff`. The continuous integration job is `browser-sddf`. The smoke count is 51.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, and `just test-browser-sddf` printed `title lerux` and `pixel 4c5258ff`.

## Milestone 13 — Agent

**Exit:** `grok` notifies the agent protection domain. The agent uses `lerux-posix` for Read, Edit, Write, ListDir, and Search, the socket library for WebFetch, and one shared memory region plus a notification for Browse. The session expects `agent: tool write ok` and the written file bytes. Tool arguments are a `#[repr(C)]` struct in the agent header, not postcard.

Done.

Landed:

- LionsOS 0.4.0 has no agent header. `lerux-sddf` defines `agent_tool_t` for this milestone. The shared page is `0xa_000_000`, one page, mapped read-write by `sddf-grok` and `sddf-agent`. Kinds are Read, Write, Edit, ListDir, Search, WebFetch, and Browse. States are idle, browse, browse done, ok, and err.
- `qemu_virt_aarch64_agent_sddf` reuses the fetch roles. The shell program is `sddf-grok.elf` and is the serial client. The agent program is `sddf-agent.elf` and is the filesystem client and the network client. `sddf-shell` stays on the other images. The postcard `agent` protection domain stays.
- The shell notifies the agent on the tool channel. The agent writes the file through `lerux-posix` and notifies the shell. Browse uses the same page and a second notification. The shell answers Browse on the root before it resumes the worker. WebFetch connects to `10.0.2.2` port 8443 through the socket library. The smoke does not call it.
- `support/smoke-expects.toml` expects `lerux shell ready`, then `agent: tool write ok`, then `hello-agent`. The continuous integration job is `agent-sddf`. The smoke count is 52.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, and `just test-agent-sddf` printed `lerux shell ready`, `agent: tool write ok`, and `hello-agent`.

## Milestone 14 — WebAssembly role

**Exit:** One Rust runtime protection domain has filesystem and socket access through `lerux-posix`, which is the role of the LionsOS WebAssembly component. `just test-program-sddf` runs a signed ordinary WebAssembly module and expects its print. A bad signature does not instantiate the module. The `LRW1` opcode list and `lerux-prog` interpreter are deleted in this milestone.

Done.

Landed:

- `lerux-wasm` runs ordinary WebAssembly with `wasmi` 2.0.0. The features are `validate`, `prefer-btree-collections`, and `portable-dispatch`. `run_signed` checks the ed25519 signature before instantiate. [ADR-012](decisions/012-ordinary-webassembly.md) records why `wasmi` is accepted here and why `wasmtime` and the WebAssembly Micro Runtime stay out. [ADR-010](decisions/010-program-runtime.md) still records the envelope. Its interpreter decision is superseded.
- `qemu_virt_aarch64_program_sddf` is the fetch image with `program` (`sddf-program.elf`) in place of the shell. The protection domain is the serial client, the filesystem client, and the network client. The module reads `/config/hostname` and allocates a socket through `lerux-posix`. The pipe does not connect. The network queues are mapped. `net_buffers_init` runs once on the transmit free queue.
- A bad signature prints `program: rejected` and does not instantiate the module. The signed module then prints `lerux-wasm: ran`. `support/smoke-expects.toml` expects those two lines in that order. The continuous integration job is `program-sddf`. The smoke count is 53.
- `lerux-prog` is deleted. The postcard `program-runtime` uses `lerux-wasm` and still logs `lerux-prog: ran`. `just test-program` stays. That protection domain has no network client.
- Checked on 2026-10-12: `just check` passed, `just check-pd` passed, `just test-program-sddf` printed `program: rejected` and `lerux-wasm: ran`, and `just test-program` printed `lerux-prog: ran`.

## Milestone 15 — Packages

**Exit:** `lerux package install edit` adds the edit program to the Rust system description for the workstation. The generator fails when the component name is unknown. Installing still rebuilds the image.

Done.

Landed:

- The template is the component table. Package fragments name the optional protection domains. `lerux package install edit` merges `edit` into `dev-workstation`, writes a system description with one `edit` protection domain, and rebuilds `loader.img`. A second run does not add a second `edit` element. The command refuses a profile that extends another profile, so `support/profiles/workstation.toml` stays as it is.
- The generator deletes an optional protection domain the profile does not select and leaves every other template protection domain in place. A profile name or a channel end that is not in the template fails with `unknown component "nosuch"` and does not write the system description.
- The pre-wired `workstation` profile still selects `edit`. Board generation for the workstation images still contains that protection domain. `dev-workstation` does not, until install.
- Checked on 2026-10-12. `cargo test -p lerux-cli` passed, including the install test and `unknown component "nosuch"`. `just check` passed. `lerux package install edit` wrote one `edit` protection domain into `build/qemu_virt_aarch64_workstation/system.system` and rebuilt `loader.img`.

## Milestone 16 — Other boards

**Exit:** The same protection-domain roles boot on QEMU RISC-V and QEMU x86, using the devices the pinned driver framework already classifies (virtio, `ns16550`). `pc_z97_d3h` is unchanged in this milestone. A native Advanced Host Controller Interface, Video Graphics Array, or desk Ethernet driver is a later driver-framework class driver in Rust, with a queue struct taken from the matching header, and it is not part of this milestone.

Done.

Landed:

- `lerux-machine` holds the physical addresses, ports, and interrupts. Protection domains compile the device offsets and the direct-memory-access addresses. The system description compiles the physical addresses when the `system-description` feature is on.
- QEMU RISC-V and QEMU x86 boot the serial, cothread, network, filesystem, fetch, web, agent, and program roles with virtio and the 16550. RISC-V graphics is the virtio-gpu page at `0x10_003_000`. x86 graphics stays unsupported. `pc_z97_d3h` is unchanged.
- On x86, an image that has both virtio-blk and virtio-net routes the block interrupt to pin 11 and the network interrupt to pin 10. The block driver uses the memory-mapped configuration window. The network driver stays at priority 2. The standalone network image keeps pin 11. The standalone block image keeps pin 10.
- An x86 filesystem smoke that prints `reboot` restarts QEMU on the host. QEMU 6.2 does not reload a multiboot kernel on `system_reset`.
- The new boards set `ci = false`. They are not in the workflow include list. The smoke count stays 53. Each one joins that list only after `just check`, `just check-pd`, and that board's smoke have passed, in the same change as the include list and the handwritten counts. The waiting list is [Outside the workflow](ci.md#waiting-on-a-smoke). Done for this milestone does not mean those boards are in continuous integration.
- Checked on 2026-10-21. `just check` passed. `just check-pd` passed for aarch64 (234 crate combinations), riscv64 (74), and x86_64 (79).

## Milestone 17 — Remove the old model

**Exit:** Guest code has no `lerux_interface_types` dependency, no `LERUXFS`, and no postcard `SerialClient` or `NetRequest`. `docs/ci.md` lists the new recipes. `just test` is the Rust serial-and-filesystem workstation smoke. `just check` still lints the host tool.

Done.

Landed:

- Guest code under `userspace/` has no `lerux-interface-types` dependency, no `LERUXFS`, and no `NetRequest`. The postcard protection domains and the crates `lerux-interface-types`, `lerux-ipc`, `lerux-fs`, `lerux-fat`, and `lerux-service-async` are deleted. Device-framework images, `hello`, and the drivers that did not use those crates stay. The logging serial sink stays for `hello` and the cothread client.
- `just test` with no `BOARD` boots `qemu_virt_aarch64_fs_sddf`. `just build` still defaults to `qemu_virt_aarch64`. The continuous integration hello job is `BOARD=qemu_virt_aarch64 just test`. The smoke include list is 19 jobs. `docs/ci.md` and the README use that count.
- `pc_z97_d3h` and the postcard workstation boards are gone. Their shells were postcard protection domains.
- A board `lerux_image::render_board` accepts, including every name ending in `_sddf` and `x86_64_generic_fs_ahci`, takes its system description from that renderer. The `template` string on those rows is not a file in `userspace/systems/templates`, and `system_vars` on those rows are not substituted. Postcard boards still read the template file.
- Checked on 2026-10-21. `just check` passed. `just check-pd` passed (51 aarch64 crate combinations and 37 x86_64 crate combinations). On a fresh `support/disk.img`, `just test` printed `lerux shell ready`, the scripted shell lines, and `reboot`.

## Milestone 18 — Paint the fetched fixture

**Exit:** The browser shell parses the fetched fixture with `lerux-html`, lays it out and paints it with `lerux-web`, and presents that frame through the existing graphics queues. `just test-browser-sddf` expects `title lerux` and then `pixel 000000ff`. The pixel line is the first non-white pixel of the frame.

Done.

Landed:

- `PaintedPage` owns a 320 by 64 frame, 81920 bytes. The user-agent sheet places the paragraph at y=24, so a 16-pixel-tall frame has no text. The client data region grows to `0x1_4000`. The other seven graphics regions stay one page. The driver attach and transfer checks use the client data size. Queues stay on `GPU_REGION_SIZE`.
- The shell calls `show_page` after the title scan. Display info still uses offset 0, and the frame is copied after that reply. The fourth byte is set to `0xff` in one place. The serial line is sampled from the frame. Resource id 1 and the scanout floor of 16 stay.
- `support/smoke-expects.toml` expects `pixel 000000ff` on both browser rows. The continuous integration job stays `browser-sddf`. The smoke count stays 53.
- Checked on 2026-10-21. `just check` passed. `just check-pd` passed. `just test-browser-sddf` printed `title lerux`, `pixel 000000ff`, and `lerux shell ready`.

Part names for these milestones are in [GLOSSARY.md](../GLOSSARY.md). The driver tables are in [platforms.md](platforms.md#x86-device-map).

The x86 guest already has the NS16550, the VGA text console, the keyboard, and the clock. The virtio x86 boards stay. Milestones 19–21 give QEMU q35 a disk and a network interface that the Z97 also has. The new protection domains keep the Microkit names `blk_driver` and `net_driver`. The block virtualiser, the filesystem server, the network virtualisers, the copier, and the shell stay. A 4096-byte transfer is eight 512-byte sectors. A flush reports success and submits no device command. The Ethernet device uses the client address `52:54:00:12:34:56`. Interrupts stay I/O APIC pins. Take every QEMU device property from that binary's `-device <name>,help`.

## Milestone 19 — Advanced Host Controller Interface disk

**Exit:** `just test-x86-ahci` boots a filesystem image whose only disk is QEMU `ich9-ahci` and prints `lerux shell ready` plus the scripted shell lines.

Done. `just test-x86-ahci` printed `ahci: port 0`, then `lerux shell ready`, then the scripted shell lines. The controller interrupt is a message-signaled interrupt on vector 51.

- New protection domain `sddf-ahci-driver`. It maps the controller register block, keeps the command list and the FIS in its own direct-memory-access region, and runs one command at a time on port 0. It speaks the existing block queues.
- It accepts a controller by class `0x010601`. On this QEMU that controller is `8086:2922` at `00:1f.2`. The Z97 Serial ATA identifier is recorded in Milestone 22, before the driver accepts it.
- Board `x86_64_generic_fs_ahci` reuses the filesystem template and roles. The QEMU command attaches `support/disk.img` as an `ide-hd` on that controller and does not attach virtio block.
- The ready line is `ahci: port 0`, printed once before the shell line. `ci` stays false.

## Milestone 20 — QEMU Ethernet

**Exit:** `just test-x86-e1000e` boots a network image whose only network device is `-device e1000e` and prints `ip 10.0.2.15`, then `ping ok`.

Done. `just test-x86-e1000e` printed `e1000e: ready`, then `ip 10.0.2.15`, then `ping ok`. The system description binds the device interrupt to I/O APIC pin 11, vector 50, level, active-low.

- New protection domain `sddf-e1000e-driver`. It programs the 82574 (`8086:10d3`): reset, link, one transmit ring, one receive ring, and the interrupt pin. Frames are copied between the shared data regions and the driver's own buffers. It speaks the existing network queues.
- Board `x86_64_generic_net_e1000e` reuses the network template and roles. User networking stays, so the address is still `10.0.2.15`. Virtio network is not attached.
- The ready line is `e1000e: ready`, printed once before the address. `ci` stays false.

## Milestone 21 — One realistic q35 image

**Exit:** `just test-x86-fetch-native` boots the fetch roles with `ich9-ahci` and `e1000e` together and prints `fetch ok`. The host fixture check still expects `lerux-http-fixture`.

Not started.

- Board `x86_64_generic_fetch_native` reuses the fetch template. The block interrupt keeps its pin. The network device uses the other pin, matching the combined virtio x86 image.
- Serial stays the NS16550. No virtio device is attached.
- After `just check`, `just check-pd`, and the three smokes pass, the workflow include list and the hand-written smoke counts move together. Until then the new boards stay out of continuous integration.

## Milestone 22 — Desk

**Exit:** The same two programs accept the recorded desk devices. This milestone cannot close on QEMU.

Not started.

- The Ethernet driver also accepts `8086:153b` at `00:19.0`, revision `0`, subsystem `1458:e000`. That is the I217-V recorded in [context.md](context.md). Memory is `0xf7e00000` (128 KiB) and `0xf7e3c000` (4 KiB). I/O is `0xf080`. The QEMU 82574 smoke does not prove that physical-layer path.
- Read the Z97 Serial ATA configuration space on the desk, then add that identifier beside `8086:2922`.
- The RTL8852BE probe stays a configuration-space read. There is no Intel display-engine driver and no Radeon driver. The desk image composes this Ethernet driver in the network-driver role.

## Milestone 23 — RISC-V high guest virtio map

**Exit:** A RISC-V virtio memory-mapped window can use a high guest virtual address, and the identity map is no longer required for boot.

Not started.

- On QEMU RISC-V the virtio memory-mapped guest virtual address equals the physical page (`0x10_001_000`, `0x10_002_000`, `0x10_003_000`). AArch64 uses the high window `0x6_000_000_000` for the same role. Copying that window onto RISC-V stops the boot. Leave the identity map in place until Microkit can place a high guest map on this architecture. `riscv_virtio_guest_address_matches_the_physical_page` locks the rule.
