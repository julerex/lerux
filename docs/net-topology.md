# Network topology (Phase 43)

Companion to [ADR-003](decisions/003-net-virtualiser.md).

## App path (all arches)

Untrusted apps never see the NIC. They use postcard RPC:

| Client PD | Typical `NetRequest` ops | net-server channel (workstation) |
|-----------|--------------------------|----------------------------------|
| shell | UDP demo / fetch helper | 3 |
| supervisor | UDP probe | 2 |
| chat-client | UDP | 6 |
| http-file-browser | TCP listen/send/recv | 7 |
| config-server | (optional) | 5 |
| fetch-client / net-client | UDP/TCP | board-specific |

Server entry: `userspace/pds/net-server` (`smoltcp` + multi-client `Handler`).

## L2 path (QEMU virtio-net)

```
virtio_net_driver  ←IRQ→  NIC
       ↕ channel 1 (pp on server)
   net_server
       ↕ shared: client_dma + rx/tx free/used rings
```

Template example: `userspace/systems/templates/net.system.template`.

Driver maps:

- device MMIO (or PCI BARs on x86)
- `virtio_net_driver_dma` (device)
- `virtio_net_client_dma` + ring MRs (**also** mapped into `net_server`)

## L2 path (RPi4 genet)

Same ring symbol names so `net-server` is unchanged; `genet-driver` replaces virtio-net.

## Why there is no `net-virt` PD yet

Serial needed a virt because the UART driver itself multi-cliented apps. Net already multi-clients apps in `net-server`, and the driver already has a **single** Microkit client (`net-server`).

A new PD is only worth it when it **removes client DMA from the NIC driver** (sDDF driver shape). That needs adapter work beyond a channel hop — see ADR-003.

## Smoke coverage

| Board / recipe | Exercises |
|----------------|-----------|
| `just test-net` | net-server + virtio-net UDP IPC |
| `just test-fetch` | TCP/DNS-ish fetch client |
| `just test-http` | inbound HTTP over virtio-net |
| `just test-workstation` | multi-client net-server + http-fs |

## Follow-up (DMA split)

1. Spike: device-only virtio-net PD using only driver DMA + hardware rings.
2. `net-virt` owns client free/used rings and buffer pool.
3. `net-server` attaches smoltcp to virt-facing rings (or frame RPC).
4. Golden: `just test-fetch` + `just test-http` bit-equivalent behaviour.
