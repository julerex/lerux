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

## L2 path (QEMU aarch64 virtio-net, Phase 43 unified-dma)

```
virtio_net_driver  ←IRQ→  NIC
  maps: MMIO + virtio_net_driver_dma (Hal | bounce) + rings
       ↕ channel 1 (pp on server)
   net_server  (smoltcp + app RPC virt)
  maps: virtio_net_driver_dma (bounce half) + rings
```

There is **no** `virtio_net_client_dma` region. Feature `unified-dma` on driver + stack PDs.

Template example: `userspace/systems/templates/net.system.template`.

## L2 path (RPi4 genet / x86)

Still use a separate client_dma-style region until ported to unified-dma.

## Why there is no extra `net-virt` PD

Serial needed a virt because the UART driver multi-cliented apps. Net multi-clients apps in `net-server`; the driver already has a single Microkit client. The sDDF win for Phase 43 is **map ownership** (no distinct client_dma in the driver SDF), not an extra hop.

## Smoke coverage

| Board / recipe | Exercises |
|----------------|-----------|
| `just test-net` | unified-dma + UDP IPC |
| `just test-fetch` | TCP fetch |
| `just test-http` | inbound HTTP |
| `just test-workstation` | multi-client net-server + http-fs |

## Follow-up

- genet / x86 unified-dma
- Optional Rx/Tx virt PD split + copy PDs if a second untrusted L2 client appears
