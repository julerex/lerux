# Browser on this desk

Label: wayfinder:map

## Destination

A written spec for booting lerux on this desk and using the full-screen Ladybird-shaped browser: type an address, and see that page on the monitor. The map ends when that spec is written.

## Notes

This desk is the Gigabyte Z97-D3H (`pc_z97_d3h`). The monitor is on the motherboard HDMI port. The desk-browser spec scans that port out through the Intel HD Graphics 4600, with firmware Initial Display Output set to `IGFX`. The RX 560 stays unused. After the reboot on 2026-10-22, firmware reports the Intel device as the boot VGA device. The browser is one full-screen surface: `browser-ui` owns chrome, `web-content` owns `lerux-html` and `lerux-web`, and `request-server` fetches. `browser-ui` paints the address line and presents the screen, as recorded in [What chrome is on the screen?](issues/04-chrome.md). The address is typed on that screen from the Phase 82 keyboard (PS/2, or a USB HID boot keyboard). Serial `open` is not the product path.

The spec amends ADR-009's QEMU-only framebuffer ceiling for this desk, as recorded in [Which scanout path does the spec commit to?](issues/05-scanout-path.md). Ladybird's topology stays. `lerux-html` and `lerux-web` stay the engines and grow to the public page recorded in [What counts as seeing a page?](issues/03-page-class.md).

Every session reads [docs/context.md](../../docs/context.md), [ADR-009](../../docs/decisions/009-interactive-surface.md), the Z97 section of [docs/boards.md](../../docs/boards.md), and [docs/plan-interactive.md](../../docs/plan-interactive.md). Grilling tickets use the grilling and domain-modeling skills. Research tickets use the research skill.

Tracker: local markdown in this directory. This repo has no `docs/agents/issue-tracker.md`.

## Decisions so far

- [How would a page leave this motherboard?](issues/02-desk-ethernet.md): No protection domain can transmit a page; the RTL8852BE probe only reads config space (`10ec:b852`), and native e1000e has no recorded device identity.
- [What can legacy boot scan out on this RX 560?](issues/01-rx560-scanout.md): Legacy Limine leaves VGA text at `0xb8000`; a linear framebuffer needs a VBE mode set on that same CSM entry, and the address is unknown until a boot reads it.
- [What counts as seeing a page?](issues/03-page-class.md): The typed `https` public name, first load in a fresh profile, matching the shared behavior of the stable Firefox and Chrome named when the spec is written. `lerux-html` and `lerux-web` grow to that.
- [What chrome is on the screen?](issues/04-chrome.md): One address line above the page. `browser-ui` paints it, presents the screen, and takes the keyboard. A failure is one of five fixed lines in the page area.
- [Which scanout path does the spec commit to?](issues/05-scanout-path.md): The VESA linear framebuffer on the legacy Limine entry. `browser-ui` maps it and copies a 1024 by 752 page under a 16-row address line.
- [Which graphics device does the desk scan out through?](issues/12-intel-scanout.md): The Intel HD Graphics 4600 on the motherboard HDMI port. Firmware Initial Display Output is `IGFX`. The RX 560 stays unused.
- [Read this card's VESA mode-info block](issues/09-mode-info.md): Limine reports `0xd0000000`, pitch 4096, 1024 by 768, 32 bits per pixel. Red is at shift 16, green at 8, and blue at 0, each 8 bits wide.
- [Does the spec include the Ethernet driver?](issues/06-ethernet-in-spec.md): The onboard Ethernet port. A new protection domain serves `net-server`, and Dynamic Host Configuration Protocol supplies the address, gateway, and resolver.
- [Does the spec include following a link?](issues/07-follow-link.md): Enter on a focused link loads that public page on this surface. The new page replaces the current one, and a tall page scrolls inside 1024 by 752.
- [How do desk keystrokes reach browser-ui?](issues/08-keyboard-path.md): `console-driver` owns the keyboard and forwards key-downs to `browser-ui`. A USB boot keyboard wins when it is armed. The shell does not receive those keys.

## Not yet specified

- Which other protection domains share the image besides the fetch path. The fetch path is the new onboard driver, `net-server`, `request-server`, and Transport Layer Security. The COM1 shell and the filesystem are still open. The desk keyboard is not the shell's.
- How the spec is demonstrated before a metal boot. QEMU may be a stand-in; it is not the product.

## Out of scope

- Porting Ladybird or embedding Servo. This map keeps `lerux-html` and `lerux-web`.
- A Polaris display-engine modeset. This spec uses the VESA linear framebuffer.
- The Sapphire RX 560. It stays in the machine, and the spec does not program it.
- The RTL8852BE as the fetch path. Its probe stays a configuration-space read.
- Signing in, and granting a permission. A login wall and an unanswered prompt are the page.
- A script-opened window, or any second surface.
- Browser extensions.
- A digital rights management module. `wasmtime` and the WebAssembly Micro Runtime stay out too.
- A window manager, Wayland, or a second graphical application.
- Building the browser. The map ends at the spec.
- QEMU as the product.
- Extra tabs, or a second `web-content`.
- Disk, downloads, bookmarks, and history.
- The agent protection domain.
- Raspberry Pi HDMI.
