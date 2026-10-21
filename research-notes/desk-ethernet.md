# How would a page leave this motherboard?

No protection domain on the Gigabyte GA-Z97-D3H can transmit a fetched page. The only network controller this repo identifies on that board is the Realtek RTL8852BE (`10ec:b852`), and `pc_z97_d3h_rtl8852be` only reads its Peripheral Component Interconnect configuration header. `pc_z97_d3h` records no network controller in the image. Native `e1000e` is an open follow-on and has no recorded device identity. The string I217 does not appear in the repo.

## Recorded facts

### The controller the tree can name

`pc_z97_d3h_rtl8852be` is the hardware-only image that talks to a network device on this motherboard. Its protection domains are `rtl8852be-probe` and `serial-driver`. There is no QEMU profile and no continuous-integration smoke, because the repo has no QEMU model of the card.

The probe reads root port `00:1c.6` (function 6 is fixed; the secondary bus is read from the bridge and is not compiled in). It then reads the endpoint at device 0, function 0 on that secondary bus, and logs the vendor, device, revision, subsystem, and assigned base addresses. The system description gives that protection domain one I/O port, `0xcf8` for 8 bytes, used as the `0xcf8` / `0xcfc` configuration pair. It does not map a base-address register, and it has no card interrupt.

`lerux-rtw8852be` parses that header. It does not touch the card. Matching ids are `10ec:b852` (the id the crate records for this Z97) and `10ec:b85b` (the second id in the Linux `rtw89_8852be` table). A match still requires BAR 0 to be I/O and BAR 2 to be a 64-bit non-prefetchable memory window, which is the window Linux `rtw89_pci` iomaps. This tree does not iomap it.

The captured header used by the host test (`fixtures/z97-05-00.0.config`, behind `fixtures/z97-00-1c.6.config`) is:

| Field | Value |
|-------|--------|
| Card | `10ec:b852` rev 0, subsystem `1a3b:5470` |
| I/O BAR | `0xd000`, length `0x100` |
| Memory BAR | `0xf7c0_0000`, length `0x10_0000` (1 MiB; the 64-byte header does not carry the length; the crate takes it from this desk's sysfs resource) |
| Bridge | Intel `8086:8c9c` at `00:1c.6`, secondary bus 5, memory `0xf7c0_0000`–`0xf7cf_ffff`, I/O `0xd000`–`0xdfff` |

`8086:8c9c` is the root port in front of the Realtek card. It is not an Ethernet device id. The probe logs whatever the live header contains. The bus number and the base addresses above are the captured assignment, not constants in the protection domain.

### What a protection domain can transmit

On `pc_z97_d3h_rtl8852be` the probe can send log lines to the serial driver (COM1 at `0x3f8`). After the log it returns. `notified` does nothing. There is no socket, no frame enqueue, and no fetch command.

`pc_z97_d3h` is not a key in `support/boards.toml`. `docs/boards.md` says so, and says the on-screen shell was a postcard protection domain. Milestone 17 of `docs/plan-lionsos.md` records that this board and the postcard workstation boards are gone. Phase 82 and the install path still describe that shell: local commands are `echo`, `help`, `pwd`, `clear`, `history`, `calc`, and `qos`; `ls`, `fetch`, and `edit` print `unavailable` because the image has no disk or network driver. ADR-010 says `pc_z97_d3h` has no network interface controller, so the signed-module fetch stays on QEMU. There is no protection domain selected by that board name that can send a page. A metal boot of the Video Graphics Array image has not been recorded.

Images that do fetch use virtio. `x86_64_generic_fetch_sddf` and `x86_64_generic_net_sddf` drive `VirtIONet` over a Peripheral Component Interconnect transport under QEMU (`qemu.net = "user"`). `x86_64_generic_http` uses `virtio-pci-driver` the same way. `docs/boards.md` says virtio-pci workstation images will not boot this motherboard. `docs/platforms.md` lists virtio-pci for the x86_64 q35 column and leaves the Z97 metal cell as later work.

`render_board("pc_z97_d3h")` returns `None` because that helper only renders names with an sddf suffix. That test does not, by itself, mean the shell board was deleted. The board table and Milestone 17 do.

A comment immediately above `[x86_64_generic_virtio]` in `support/boards.toml` still says "Same console image as pc_z97_d3h". The section under that comment is the virtio hello board (`hello`, `serial-driver`, `virtio-pci-driver`), not a console image. `console-pc.system.template` is not in `userspace/systems/templates/`, and `support/profiles/` has no `pc-console` profile.

## Open follow-on

This is a plan item, not a controller the image uses.

`docs/plan-arch.md` (Physical RPi4 lab, follow-on after the Pi gate) and `docs/plan.md` (Phase 82, and the same item under Physical RPi4 lab) leave one box unchecked: native Advanced Host Controller Interface and native `e1000e`, so that `ls` and `fetch` work on `pc_z97_d3h`. The same sentence says the Video Graphics Array shell, the keyboards, COM1, and the Limine ISO are already in the image. `docs/boards.md` says those workstation drivers are not in this board, and that QEMU `workstation-x86` is virtio-pci only. `docs/platforms.md` marks the Z97 metal network cell `e1000e later` and the block cell `AHCI later`. The file header of `docs/plan-arch.md` still says Advanced Host Controller Interface / `e1000e` are open (last updated 2026-09-26).

Milestone 16 of `docs/plan-lionsos.md` says a native desk Ethernet driver is a later class driver and is not part of that milestone, and that `pc_z97_d3h` was unchanged there. That sentence does not name `e1000e` and does not give a device id.

No file records a Peripheral Component Interconnect id, bus address, or base address for the onboard Ethernet controller that `e1000e` would drive. I217 is not named.

## Evidence

- `support/boards.toml` — `[pc_z97_d3h_rtl8852be]`; no `[pc_z97_d3h]`
- `userspace/systems/templates/rtl8852be-probe.system.template` — config I/O port only
- `userspace/pds/rtl8852be-probe/src/main.rs` — `probe`, `ROOT_DEV` `0x1c`, `ROOT_FUNC` `6`
- `userspace/crates/lerux-rtw8852be/src/lib.rs` — `RTL8852BE`, `identify`, `z97_header_is_8852be_at_the_assigned_bars`, `z97_root_port_window_covers_the_card`
- `userspace/crates/lerux-rtw8852be/fixtures/z97-05-00.0.config`, `fixtures/z97-00-1c.6.config`
- `docs/boards.md` — hardware-board bullet, Gigabyte Z97-D3H install path, "not in this board"
- `docs/platforms.md` — HW x86 row and the Z97 metal network cell
- `docs/plan.md` — Phase 81 ("no NIC"), Phase 82, Physical RPi4 lab follow-on
- `docs/plan-arch.md` — same follow-on
- `docs/plan-lionsos.md` — Milestones 16 and 17
- `docs/decisions/010-program-runtime.md` — fetch stays on QEMU
- `userspace/pds/sddf-net-driver/src/pci.rs` — `VirtIONet` on x86
- `tools/lerux-cli/src/board.rs` — `rtl8852be_probe_board_is_hardware_only`

## Unknowns

- The onboard part behind the name `e1000e` has no recorded id. Nothing in the tree says it is an I217, or says which bus it sits on.
- The fixture's bus 5 and base addresses are one captured assignment. The probe will log a different assignment if firmware moves the windows.
- Phase 82 prose still describes a shell image that the board table and Milestone 17 say is gone. Those two accounts are both in the tree. Neither account includes a network transmit on this motherboard.
- No metal run of the probe's log line is recorded in these docs. The identity above is the checked-in header and the host test, not a boot log from this session.
