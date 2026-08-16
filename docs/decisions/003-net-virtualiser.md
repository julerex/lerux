# ADR-003: Network virtualiser topology (sDDF-shaped, Rust)

## Status

Accepted (Phase 43 complete for aarch64 virtio-net: unified DMA)

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

1. **App-facing boundary:** untrusted apps use only `NetRequest` / `NetResponse`. No app maps virtio rings or net DMA.
2. **`net-server` is the trusted stack + RPC virt:** sole Microkit client of the NIC driver PD; sole smoltcp owner.
3. **Unified DMA (aarch64 virtio-net boards):** drop the separate `virtio_net_client_dma` MR. One `virtio_net_driver_dma` region is split in software:
   - low half → virtio-drivers `HalImpl` (device buffers only)
   - high half → shared bounce for ring payloads (driver `HandlerImpl` + stack `DeviceImpl`)
   The driver therefore has **no distinct client_dma map** in the system description; the bounce is subsystem DMA shared only with the trusted stack PD (sDDF “DMA region”, not untrusted client data).
4. **Do not** insert a no-op extra PD hop without map changes.
5. Stretch: genet / x86 / multi-virt (Rx+Tx PDs) can adopt the same vocabulary later.

## Trust map (aarch64 virtio-net after Phase 43)

| PD | MMIO / IRQ | DMA maps | App RPC | Notes |
|----|------------|----------|---------|--------|
| `virtio-net-driver` | yes | `driver_dma` only (Hal + bounce halves) | no | No `client_dma` MR |
| `net-server` | no | same `driver_dma` (bounce half) + rings | yes | Trusted stack / virt |
| shell / fetch / http-fs / chat | no | **none** | yes | postcard only |

| PD (other platforms) | Notes |
|----------------------|--------|
| virtio-pci (x86 QEMU) | Unified-dma (Phase 61); combo Hal 4 MiB + bounce 2 MiB |
| genet | Still uses separate client_dma until the RPi4 lab port |

## Alternatives considered

### Full sDDF Rx/Tx virt + copy swarm in C

Rejected: C userspace out of scope; large dependency surface.

### Insert passthrough `net-virt` between driver and net-server today

Rejected for v1: rust-sel4 net driver adapter expects the ring client in the same process pattern as today; a blind passthrough either keeps client DMA in the driver or forces a large reimplementation without a clear incremental smoke path.

### Move smoltcp into each app

Rejected: duplicates stacks; weakens isolation story; conflicts with existing `NetRequest` API.

## Consequences

- Aarch64 virtio-net boards (`net`, `fetch`, `http`, workstation, composed variants) use **unified-dma**: no `virtio_net_client_dma` region in the SDF.
- Driver address space no longer includes a separate client_dma mapping; bounce lives in the high half of `virtio_net_driver_dma`.
- Apps still never map net DMA; `NetRequest` remains the API.
- Residual vs full sDDF: no separate Rx/Tx virt PDs or per-client copy PDs; genet not yet on unified-dma. QEMU x86/RISC-V unified-dma is Phase 61.

## References

- [sDDF network.md](https://github.com/au-ts/sddf/blob/main/docs/network/network.md)
- [ADR-002 serial virtualiser](002-serial-virtualiser.md)
- [docs/net-topology.md](../net-topology.md)
- Phase 43 in [plan-au-ts.md](../plan-au-ts.md)
