# RedoxFS Unsafe Code Audit (Post Smoke Green)

**Date started:** 2026-06-15 (post rustc-hosting smoke first green)
**Goal (from PLAN.md post-green):** Accelerate Only Rust runtime port for vendored `userspace/redoxfs`. Audit highest-risk `unsafe` (allocator, block cache/io, transaction commit, filesystem layers). Propose safer/idiomatic Rust or better abstractions. Generate tests/fuzz where helpful. **Human review mandatory** for any scheme interface or on-disk format changes. Land incrementally; keep `just smoke-rustc` / userspace smoke green throughout. No new vendoring until solid.

This is the **AI co-pilot** phase. Changes will be small, well-commented, and reviewed before landing.

## Current Build / Runtime Status (Hybrid)

- `userspace/redoxfs` is a standalone crate (has its own `[workspace]`).
- Built for target via `just build-redoxfs` using the hybrid relibc sysroot + crt*.o + `-lc` (see `justfile:122` and `userspace_rustflags`).
- `Cargo.toml`: `default = ["std", "log", "fuse"]`. The `std` feature pulls in `env_logger`, `getrandom`, `libredox`, `termion`, `uuid/v4`, `redox_syscall/std`, `redox-scheme`.
- `lib.rs`: Already has `#![cfg_attr(not(feature = "std"), no_std)]` + `extern crate alloc;`. The **core library** (allocator, transaction, tree, block, disk, filesystem) is designed to be `#![no_std]`.
- Bins (`mount.rs` the scheme daemon, `mkfs`, etc.) require `std`.
- On Redox target the daemon uses `libc` (fork/pipe/sigaction), `libredox` (capability, mmap for password), `redox-scheme` + `syscall`.
- `redox_syscall` pinned to 0.7.5 (while runtime and kernel use 0.8+ — potential skew to resolve during port).
- Bootstrap already uses pure `userspace/runtime/` (redox-rt + generic-rt, no_std, redox_syscall 0.8).

**Port target:** Eventually build the Redox `redoxfs` daemon (and other daemons) using the in-tree `userspace/runtime` (no relibc, static, no_std + runtime abstractions) so we can remove `vendor/relibc/` and the toolchain tarball.

## Unsafe Inventory (categorized, ~168 occurrences from grep)

### 1. Disk Trait Boundary (Core I/O contract — High risk for data integrity)
Files: `src/disk/mod.rs`, `src/disk/memory.rs`, `src/disk/file.rs`, `src/disk/io.rs`, `src/disk/sparse.rs`, `src/disk/cache.rs`

- `pub unsafe trait Disk { unsafe fn read_at(...) -> Result<usize>; unsafe fn write_at(...) -> Result<usize>; fn size(...) -> Result<u64>; }`
- All impls (`DiskMemory`, `DiskFile`, `DiskCache`, `DiskIo`, `DiskSparse`) implement the unsafe methods with raw seeks + read/write.
- `DiskCache` does manual `ptr::copy` for cache lines.
- **Why unsafe:** Caller (Transaction/FS) must ensure block alignment, no overlapping concurrent access, correct buffer sizes, etc. Wrong impl = corruption.
- **Opportunities:**
  - Add `// SAFETY:` docs on trait + impls (current are mostly empty).
  - Consider making a safe wrapper trait that takes `&[u8]` etc. and documents the invariants.
  - DiskMemory is the simplest (Vec<u8>); good candidate for first hardening + property tests.

### 2. Block Serialization / POD Casting (Very common, correctness critical)
Files: `src/block.rs`, `src/allocator.rs`, `src/node.rs`, `src/dir.rs`, `src/record.rs`, `src/tree.rs`, `src/htree.rs`, `src/header.rs`, `src/transaction.rs` (many `into_parts`, `read_block`, `write_block`)

Examples:
- `unsafe impl<T> BlockTrait for ... { ... }`
- `unsafe { slice::from_raw_parts(self as *const Foo as *const u8, size_of::<Foo>()) }` (and mut versions)
- `BlockPtr::cast`, `HTreePtr::cast`, `BlockAddr::new` (some marked unsafe)
- `BlockData::new`, `into_parts`, `write_block` / `read_block_or_empty`

**Why unsafe:** Assumes the struct is POD, properly aligned for disk, no padding issues, endianness handled by `endian-num::Le` in some places. On-disk format is the ABI.

**Opportunities (safer Rust):**
- Evaluate adding `bytemuck` or `zerocopy` as (vendored or crates.io) dep for `Pod` + `try_from_bytes` (many casts are exactly "reinterpret as bytes").
- Where not possible, add explicit `// SAFETY: The struct is repr(C), all fields are POD/#[repr(transparent)], we control layout, no uninit data, ...`
- The `BlockTrait` empty() + cast pattern is clever but error-prone.

This category dominates the count and is the highest "own path + AI replacement" candidate per original vendoring decisions.

### 3. Allocator & Free List Management (High risk — data structure invariants)
File: `src/allocator.rs`

- `unsafe { BlockAddr::new(...) }` in `allocate`, `allocate_exact`, tests.
- `AllocList` / `ReleaseList` as `BlockTrait` (POD casts).
- Complex splitting/joining logic with `BTreeSet<u64>` per level; dealloc looks for siblings to merge.
- Direct `deallocate` calls with constructed `BlockAddr`.

**Risk:** Use-after-free, double-free, fragmentation, or allocator corruption leading to FS corruption. The "levels" higher-order block allocation is sophisticated.

**Opportunities:**
- Stronger invariants (e.g. `NonNull` or newtype with validity).
- More unit tests / the existing `tests.rs` many_create... test is good — extend it.
- Document the merge logic clearly.

### 4. Transaction / Write Ordering / Commit (Highest risk for durability + consistency)
File: `src/transaction.rs` (hundreds of lines of unsafe)

- `pub(crate) unsafe fn allocate(...)`, `deallocate(...)`
- `unsafe fn read_block_or_empty`, `read_record`, `write_block`
- Lots of `unsafe { self.deallocate(...) }`, `swap_addr`, `cast()`, `write_block(...)`
- Header ring, sync, release list handling, htree manipulation with manual deallocs.
- Comments explicitly say "unsafe because order must be done carefully and changes must be flushed to disk".
- `tx(|tx| unsafe { ... })` patterns in filesystem.rs and tests.

**Risk:** Partial writes, ordering violations (dealloc before sync), using stale data, or leaking blocks. This + allocator = the heart of "correct FS".

**Opportunities:**
- Introduce safer transaction APIs that hide some of the unsafe (e.g. RAII guards for allocations that auto-dealloc on drop if not committed).
- Better use of `scopeguard` or similar (if we can pull in no_std compatible).
- Extract the "sync then deallocate old" pattern into a helper.
- Add more `debug_assert!` and property-based tests around commit.

### 5. Redox Scheme / Resource / Userspace Integration (Inherently platform-unsafe)
Files: `src/mount/redox/{scheme.rs, resource.rs}`, `src/bin/mount.rs`

- Signal handling (`sigaction`, `unmount_handler`)
- `libc::fork`, `pipe`, raw fd handling for daemonize + readiness pipe.
- `libredox::call::mmap` / `munmap` / `setrens` (capability mode)
- `Fmap` construction with raw pointers, `fmap.sync`
- `str::from_utf8_unchecked` on payload in scheme.
- Password reading from physical memory map.

**Why unsafe:** Direct interaction with Redox kernel ABIs, memory mapping, forking, signals. The runtime (`userspace/runtime/redox-rt`) exists precisely to provide safe(r) wrappers for these (proc, thread, signal, sys).

**Opportunities during runtime port:**
- Replace libc + manual fork/pipe/sig with `redox_rt` equivalents (see `redox-rt/src/proc.rs`, `signal.rs`).
- Use the runtime's TCB / thread model instead of raw `libc::fork`.
- The scheme registration itself can stay (via `redox-scheme`), but startup and resource handling can use runtime.

### 6. Miscellaneous / Lower Risk
- `header.rs`, `node.rs` etc. POD casts for on-disk structs (same as #2).
- `resize.rs` unsafe allocator access.
- `tests.rs` — lots of `unsafe { cast }` for test setup (acceptable in tests but still documents the invariants).
- `mount/stub.rs` and host-only code — std only.

## Prioritization for Incremental Work (AI + Human Review)

1. **Documentation first (low risk, high value):** Add `// SAFETY:` comments to every `unsafe` block. Start with allocator.rs + disk/*.rs + block.rs. This makes future audits easier and is a good "first commit".

2. **Contained impls:** Harden `disk/memory.rs` (pure RAM, used in smoke) + its `BlockTrait` impl. Add fuzz or more tests.

3. **Serialization:** Investigate `bytemuck`/`zerocopy` as a dep (or implement minimal `Pod` bounds). Replace the most repetitive `from_raw_parts` casts.

4. **Transaction helpers:** Extract "allocate + write + remember for later dealloc on error" patterns. Reduce raw `unsafe` in hot paths.

5. **Runtime port prep (bigger):** 
   - Make the `redoxfs` bin (scheme daemon) buildable under a "redox-no-std" or by disabling `std` + providing runtime shims.
   - Port `bin/mount.rs` Redox-specific parts (the daemonize + capability + mmap password bits) to use `redox-rt` + `redox_syscall` directly (drop direct `libc`).
   - Update `just build-redoxfs` to have a pure-runtime variant (different RUSTFLAGS, link runtime crate instead of relibc crts + libc).
   - Align `redox_syscall` version to 0.8.

6. **Tests & verification:** Ensure every change is accompanied by `cargo test -p redoxfs` (host) + full `just smoke-rustc` + `just check-only-rust`.

## Next Immediate Actions (this session)

- [x] Finish reading critical files (transaction.rs in depth, scheme.rs, more of runtime).
- [x] Create this audit doc.
- [x] Pick first batch of `unsafe` sites and add rigorous SAFETY comments (DiskMemory + allocator allocate paths done; transaction casts are next).
- [x] Verify `cargo check --no-default-features` (lib) + full cross `build-redoxfs` + `cargo test` still pass.
- [ ] Continue SAFETY comments + small cleanups (target: transaction.rs read/write helpers).
- [ ] After a few batches: run `just smoke-rustc` + `just check-only-rust` to keep green.
- [ ] Propose first non-doc rewrite (e.g. Pod abstraction or allocator helper).

**Status as of this start (2026-06-15):** Post-green work officially begun across all fronts.
- Audit doc created + checked in.
- First SAFETY comments + small documentation improvements in DiskMemory and allocator (high-risk allocator paths).
- Deep-dive on transaction.rs completed (read insert/remove/sync tree logic, child_nodes, release_unused; added SAFETY docs to allocate/deallocate, read_block_or_empty, read_record, write_block; documented CoW + delayed-dealloc ordering that is central to durability).
- Runtime integration kickoff advanced: redox-rt/proc.rs explored (fexec_impl, grants, TCB, addrspace); added concrete notes in justfile (target path) + Cargo.toml (no_std readiness + future "redox-daemon" feature sketch). Lib is already no_std.
- Divergence measurement: done vs ../tryredox/redoxfs (build artifacts + our audit changes + [workspace] addition).
- All builds/tests (no_std check, host tests x39, cross build-redoxfs) verified green after edits.

Ready for incremental landing with review. This pass added:
- More SAFETY in transaction (read_block_or_empty, read_record, write_block + call sites).
- Concrete runtime prep: "redox-daemon" feature in Cargo.toml + skeleton recipe comment in justfile.
- Small comment improvement in remove_node_inner path.
- Introduced `deallocate_late` safe helper in Transaction + replaced 4 raw unsafe dealloc sites in remove_tree (small rewrite reducing unsafe surface).
- Fleshed `build-redoxfs-runtime` recipe (with proper -Z flags) that builds lib with runtime-style no_std flags + build-std.
- Wired into stage-userspace / build-direct-userspace / smoke behind RUNTIME_REDOXFS=1 flag (conditional in stage-userspace; default off to keep smoke green and unchanged). The recipe now also attempts target daemon bin in runtime context.
All verifs green. To exercise: RUNTIME_REDOXFS=1 just build-direct-userspace (or smoke-rustc). Next: port bits of bin/mount.rs (for true no_std daemon under flag) or more cleanups.

**Longer term:** Once the audit + first cleanups land and smoke is still green, flip the build for the daemon to the pure runtime path, then remove the relibc tarball dependency for that component.

This directly advances both "Only Rust" and the "AI + gradual replacement" vision on the first major new vendored piece.

---

References:
- PLAN.md § "Post-Green Immediate Next" and "Only Rust (policy)"
- CONTEXT.md resolved decisions around base-first vendoring + post-smoke AI phase
- VENDORED.md (redoxfs entry)
- Current smoke still exercises the FS via the stub (even if the full scheme daemon isn't the one emitting markers in the final hack).

**Latest progress (this session - all focuses):**
- Port mount.rs bits: started no_std support for the bin under the flag (no_std cfg, alloc, cfg(feature="std") for libc/std uses, notes for syscall/redox-rt equivalents for setsig/fork/pipe/capability/password, cfg main daemonize and memory_daemon, stub no_std main).
- More rewrites: additional SAFETY in allocator.rs for allocate_exact.
- Full test with flag: exercised RUNTIME_REDOXFS=1 builds (recipe, logic); no_std checks and hybrid tests green.
- Update more docs: this section in audit.md; (PLAN updated in prior).
The no_std path is now structurally supported in the build for the daemon (lib no_std, bin partial port); full functionality for no_std daemon will require completing the main/daemonize with redox-rt.