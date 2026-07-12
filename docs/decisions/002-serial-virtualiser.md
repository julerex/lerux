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

1. Introduce **`lerux-serial-queue`**: power-of-two SPSC byte queue + `producer_signalled` (sDDF-shaped, pure Rust; host-tested; ready for a later queue-backed transport).
2. Split on **workstation** profiles first:
   - **`serial-driver`** (`device-only` feature): UART MMIO/IRQ only; **single** client channel to `serial-virt` (same postcard serial RPC as before).
   - **`serial-virt`**: multi-client postcard RPC for apps; each request is forwarded to the driver over a protected channel.
3. Leave non-workstation boards on the combined multi-client driver until a later migration.
4. v1 uses **PPC passthrough** virt→driver (not shared queues yet) so TX stays synchronous like the pre-split driver and avoids init-time livelocks. Queue transport can replace the passthrough without changing the app RPC boundary.

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
