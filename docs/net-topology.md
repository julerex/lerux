# Network topology

Companion to [ADR-003](decisions/003-net-virtualiser.md).

Milestone 17 deleted `userspace/pds/net-server` and the postcard `NetRequest` path. There is no `net-server`. Two client paths remain.

## Device-framework images

The client owns smoltcp. A driver, a transmit virtualiser, a receive virtualiser, and one copier move the frames. The client does not map the device queue.

| Image | Recipe | Client |
|-------|--------|--------|
| `qemu_virt_aarch64_net_sddf` | `just test-net-sddf` | `net_client` |
| `qemu_virt_aarch64_fetch_sddf` | `just test-fetch-sddf` | shell (`lerux-tls` inside the client) |
| `qemu_virt_aarch64_browser_sddf` | `just test-browser-sddf` | shell (same fetch path, then paint) |

The web, agent, and program images reuse that network path. Agent rules are in [AGENTS.md](../AGENTS.md#network-image-qemu_virt_aarch64_net_sddf).

## Remaining virtio boards

`virtio-net-driver` owns the device. `http-server` is the inbound client on the HTTP boards. The virtio hello boards attach the same driver next to `hello`.

| Recipe | Exercises |
|--------|-----------|
| `just test-http` | inbound HTTP on `qemu_virt_aarch64_http` |
| `just test-x86-http` | inbound HTTP on `x86_64_generic_http` |
| `just test-riscv-http` | inbound HTTP on `qemu_virt_riscv64_http` |
| `just test-virtio` | virtio hello with `virtio-net-driver` |
| `just test-x86-virtio` | virtio hello with `virtio-pci-driver` |
| `just test-riscv-virtio` | virtio hello with `virtio-net-driver` |

On QEMU virtio-net, **unified-dma** still applies: Hal plus bounce share the driver DMA region. Apps never map net DMA. See [ADR-003](decisions/003-net-virtualiser.md).

```
virtio_net_driver / virtio_pci_driver  ←IRQ→  NIC
  maps: MMIO/PCI + driver_dma (Hal | bounce) + rings
```

- aarch64 / RISC-V MMIO: bounce is the high half of `virtio_net_driver_dma` (1 MiB + 1 MiB).
- x86 HTTP: same 1+1 MiB split of the driver DMA region.
- x86 virtio hello: bounce sits after the 4 MiB Hal in a 6 MiB driver DMA region.

## L2 path (RPi4 genet)

Still uses a separate client_dma-style region ([Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated)).

## Follow-up

- genet unified-dma stays [lab](plan-arch.md#physical-rpi4-lab-hardware-gated) (Phase 61 closed QEMU arches)
- The desk-browser spec names a `net-server` served by an onboard Ethernet driver. That protection domain is not in this tree.
