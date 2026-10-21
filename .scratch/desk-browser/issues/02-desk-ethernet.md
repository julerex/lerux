# How would a page leave this motherboard?

Type: research
Label: wayfinder:research
Status: resolved

## Question

For the Gigabyte GA-Z97-D3H as this repo describes it, which network controller would carry a fetched page, what identity and plan does the repo already record, and what can a protection domain actually transmit today?

Cover `pc_z97_d3h` and `pc_z97_d3h_rtl8852be`. Keep the recorded facts separate from the open follow-on named native e1000e (and AHCI) in [docs/plan-arch.md](../../../docs/plan-arch.md) and [docs/boards.md](../../../docs/boards.md). Virtio-pci workstation images do not boot this motherboard. Do not propose a driver design. Do not reboot the machine.

## Answer

No protection domain on this motherboard can transmit a fetched page. The only network controller the repo identifies is the Realtek RTL8852BE (`10ec:b852`) behind root port `00:1c.6`, and `pc_z97_d3h_rtl8852be` only reads its configuration header. `pc_z97_d3h` has no network controller in the image. Native e1000e (and AHCI) stays an open follow-on with no recorded device identity; I217 is not named. Virtio-pci images that can fetch do not boot this board.

Findings: [research/desk-ethernet](https://github.com/julerex/lerux/tree/research/desk-ethernet) — `research-notes/desk-ethernet.md`
