# lerux microbench results (Phase 49)

Generated (unix): 1783873277

## Host

- **uname:** `Linux desktop 6.8.0-124-generic #124~22.04.1-Ubuntu SMP PREEMPT_DYNAMIC Tue May 26 21:05:19 UTC  x86_64 x86_64 x86_64 GNU/Linux`
- **QEMU:** `QEMU emulator version 6.2.0`
- **Note:** Host wall-clock between guest start/done markers on QEMU TCG; relative only.

## Results

| Board | Metric | Value | Unit | n | total_ns |
|-------|--------|------:|------|--:|--------:|
| `qemu_virt_aarch64_bench_echo` | echo_rtt | 28526 | ns | 1000 | 28526039 |
| `qemu_virt_aarch64_bench_blk` | blk_read | 2451 | iops | 500 | 203994191 |
| `qemu_virt_aarch64_bench_net` | udp_tx | 2252 | pps | 200 | 88803700 |

## Markers

- start: `INFO  [echo_client] lerux-bench: echo start n=1000`
- done: `INFO  [echo_client] lerux-bench: echo done n=1000`
- start: `INFO  [blk_client] lerux-bench: blk_read start n=500`
- done: `INFO  [blk_client] lerux-bench: blk_read done n=500`
- start: `INFO  [net_client] lerux-bench: udp_tx start n=200`
- done: `INFO  [net_client] lerux-bench: udp_tx done n=200`

## Repro

```bash
just bench
# or: cargo run -q -p lerux-cli -- bench
```

Host wall-clock between guest markers on QEMU TCG — compare relatively on the same machine.
