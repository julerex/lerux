# ADR-002: Serial virtualiser (sDDF-shaped, Rust)

## Status

Accepted

## Date

2026-07-12

## Context

Today one `serial-driver` PD owns UART MMIO/IRQ **and** multiplexes clients over postcard RPC (`multi-client-2/3`). That couples device access with policy and maps every client channel into the driver address space (trust and blast radius).

sDDF serial splits:

| Role | Responsibility |
|------|----------------|
| UART driver | MMIO + IRQ only; shared queues with virtualisers |
| TX virt | Multiplex client TX → driver TX queue |
| RX virt | Demux driver RX → current client RX queue |
| Clients | SPSC queues + notify (no device maps) |

lerux keeps **typed postcard RPC** for apps (`SerialClient` / `lerux_logging::serial`). Phase 42 steals the **trust boundary**, not the C queue ABI.

## Decision

1. Introduce **`lerux-serial-queue`**: power-of-two SPSC byte queue + separate data region + `producer_signalled` (sDDF-shaped, pure Rust).
2. Split on **workstation** profiles first:
   - **`serial-driver`** (`device-only` feature): UART only; TX/RX queues shared with virt; notify channel to virt.
   - **`serial-virt`**: multi-client postcard RPC (same wire protocol as today’s driver handler); drains TX queue to driver, RX to clients.
3. Leave non-workstation boards on the combined multi-client driver until a later migration.
4. Do **not** require per-client shared queues for v1 — virt keeps the existing RPC boundary so shell/log/supervisor need no source changes beyond channel peer rename in the profile.

## Alternatives considered

### Port sDDF serial C components

Rejected: violates Rust-only userspace; pulls C sDDF dependency tree.

### Shared queues all the way to clients

Deferred: larger client churn; Phase 42 exit only requires driver without client maps and working smokes.

### Keep combined driver forever

Rejected as long-term model; workstation multi-client already shows the pain.

## Consequences

- Driver PD maps: UART MMIO + serial queue regions only (no client PPCs for data).
- Profile channels: clients → `serial_virt`; `serial_driver` ↔ `serial_virt` notify.
- `lerux profile check-channels` treats `serial_virt` as alias of `serial_driver` for `SERIAL_DRIVER` consts.
- Future: split TX/RX virts, switch-char RX demux, per-client queues.

## References

- [sDDF serial.md](https://github.com/au-ts/sddf/blob/main/docs/serial/serial.md)
- [docs/system-generation.md](../system-generation.md)
- Phase 42 in [plan-au-ts.md](../plan-au-ts.md)
