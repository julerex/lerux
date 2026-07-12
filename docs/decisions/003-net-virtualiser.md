# ADR-003: Network virtualiser topology (sDDF-shaped, Rust)

## Status

Accepted (Phase 43 design; DMA ownership split deferred)

## Date

2026-07-12

## Context

Today’s net path:

```
apps (shell, fetch, http-fs, …)
        │  NetRequest / NetResponse (postcard RPC)
        ▼
   net-server  (smoltcp stack + multi-client RPC mux)
        │  shared rings + client DMA + notify
        ▼
   virtio-net-driver | genet-driver | virtio-pci-driver
        │
        ▼
      NIC
```

sDDF network splits further:

| Role | Responsibility |
|------|----------------|
| Ethernet driver | NIC regs/IRQs/hardware rings only — **no client DMA** |
| Tx virt | Multiplex client TX buffers to driver |
| Rx virt | Classify RX, hand off to copiers / trusted clients |
| Copy PD (per client) | Copy RX DMA → private client data (privacy + availability) |
| Client | Own IP stack; Tx/Rx queues only |

lerux already puts **untrusted apps behind typed RPC** (`NetRequest` / `NetResponse`), which matches sDDF’s “prefer untrusted clients not to own L2 DMA”. The remaining gap is **driver address-space trust**: `virtio-net-driver` maps both driver DMA **and** client DMA / rings (required by rust-sel4 `sel4-microkit-driver-adapters` net `HandlerImpl` today).

Serial Phase 42 taught us to prefer a working trust boundary (device-only driver + virt) over a full sDDF queue port on day one.

## Decision

1. **Document and freeze the app-facing boundary:** all untrusted apps use `NetRequest` / `NetResponse` via `net-server` (or a future rename). No app maps virtio rings.
2. **Treat `net-server` as the trusted “stack + RPC virt”** for Phase 43: it is the sole Microkit client of the NIC driver PD and the sole smoltcp owner.
3. **Defer “driver without client DMA”** until we either:
   - extend/replace the rust-sel4 virtio-net driver adapter so the driver PD only maps device + driver DMA, with a separate virt PD owning client rings; or
   - reimplement a thin device-only net driver + queue handoff in lerux (using lessons from `lerux-serial-queue`).
4. **Do not** insert a no-op `net-virt` PD that only renames channels without changing maps — that adds hop cost without a trust win (serial-virt was different: it removed multi-client from the UART driver).
5. Stretch (later): genet path reuses the same role map; multi-stack clients still go through RPC unless explicitly trusted.

## Trust map (current, Phase 43)

| PD | MMIO / IRQ | Client DMA / rings | App RPC | Notes |
|----|------------|--------------------|---------|--------|
| `virtio-net-driver` | yes | yes (shared w/ server) | no | Sole L2 client = `net-server` |
| `genet-driver` | yes | yes (virtio-shaped rings) | no | RPi4 |
| `virtio-pci-driver` | PCI | yes | no | x86 combined |
| `net-server` | no | yes | yes | smoltcp + multi-client mux |
| shell / fetch / http-fs / chat | no | no | yes | postcard only |

### Target (post DMA-split)

| PD | MMIO / IRQ | Client DMA / rings | App RPC |
|----|------------|--------------------|---------|
| `*-net-driver` | yes | **no** | no |
| `net-virt` (Tx/Rx or combined) | no | yes (driver-facing) | optional L2 |
| `net-server` | no | yes (or via virt only) | yes |

## Alternatives considered

### Full sDDF Rx/Tx virt + copy swarm in C

Rejected: C userspace out of scope; large dependency surface.

### Insert passthrough `net-virt` between driver and net-server today

Rejected for v1: rust-sel4 net driver adapter expects the ring client in the same process pattern as today; a blind passthrough either keeps client DMA in the driver or forces a large reimplementation without a clear incremental smoke path.

### Move smoltcp into each app

Rejected: duplicates stacks; weakens isolation story; conflicts with existing `NetRequest` API.

## Consequences

- Phase 43 exit for **documentation + app trust** is met: apps never map NIC DMA; `NetRequest` remains the API.
- Phase 43 exit for **driver without client DMA** remains open (tracked as follow-up under plan-au-ts Phase 43 stretch / next PR).
- Serial-virt (ADR-002) remains the template for device-only drivers when the adapter stack allows it.
- Fetch/HTTP/net smokes stay the regression gate for any future DMA split.

## References

- [sDDF network.md](https://github.com/au-ts/sddf/blob/main/docs/network/network.md)
- [ADR-002 serial virtualiser](002-serial-virtualiser.md)
- [docs/net-topology.md](../net-topology.md)
- Phase 43 in [plan-au-ts.md](../plan-au-ts.md)
