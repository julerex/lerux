# Performance baselines (Phase 49)

Reproducible **host-timed microbenches** on QEMU aarch64 for postcard + shared-ring paths:

| Metric | Board | Guest work |
|--------|-------|------------|
| Echo RTT | `qemu_virt_aarch64_bench_echo` | 1000× `EchoRequest::Ping` PPC |
| Block read IOPS | `qemu_virt_aarch64_bench_blk` | 500× sector read via `blk-server` |
| UDP TX PPS | `qemu_virt_aarch64_bench_net` | 200× `UdpTx` + `Poll` complete |

Guests print `lerux-bench: <name> start n=N` / `done n=N`. The **host** measures wall-clock between those lines (stock seL4 EL0 cannot read `CNTVCT_EL0` without traps).

QEMU TCG numbers are for **relative** comparison on the same host — not bare-metal sDDF.

## Run

```bash
just bench
# or: cargo run -q -p lerux-cli -- bench

# Phase 57: same run + fail if support/bench-thresholds.toml is missed
just bench-check
```

Outputs:

- `build/bench/bench-results.json`
- `build/bench/bench-results.md`
- `build/bench/<board>.serial.log` (raw capture for `lerux diagnose`)
- `docs/bench-results.latest.md` (snapshot; commit if you want a recorded baseline)

Thresholds live in [`support/bench-thresholds.toml`](../support/bench-thresholds.toml) (loose QEMU TCG floors/ceilings).

## Latest snapshot

See [bench-results.latest.md](bench-results.latest.md) after `just bench`.

## Optional external comparison

Comparing to sDDF/LionsOS is **out of tree**. Do not vendor sDDF into this repo.

## Caveats

- Host load affects wall-clock; take medians of several runs for papers.
- UDP “PPS” is completed UdpTx RPCs, not wire packet rate.
- Echo RTT is PPC round-trip, not network RTT.
