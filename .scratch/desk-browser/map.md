# Browser on this desk

Label: wayfinder:map

## Destination

A written spec for booting lerux on this desk and using the full-screen Ladybird-shaped browser: type an address, and see that page on the monitor. The map ends when that spec is written.

## Notes

This desk is the Gigabyte Z97-D3H (`pc_z97_d3h`). The monitor is HDMI on the Sapphire RX 560, the firmware boot VGA card. The browser is one full-screen surface: `browser-ui` owns chrome, `web-content` owns `lerux-html` and `lerux-web`, and `request-server` fetches. `browser-ui` paints the address line and presents the screen, as recorded in [What chrome is on the screen?](issues/04-chrome.md). The address is typed on that screen from the Phase 82 keyboard (PS/2, or a USB HID boot keyboard). Serial `open` is not the product path.

The spec may amend ADR-009's QEMU-only framebuffer ceiling far enough to present a software bitmap on this HDMI. Ladybird's topology stays. `lerux-html` and `lerux-web` stay the engines and grow to the public page recorded in [What counts as seeing a page?](issues/03-page-class.md).

Every session reads [docs/context.md](../../docs/context.md), [ADR-009](../../docs/decisions/009-interactive-surface.md), the Z97 section of [docs/boards.md](../../docs/boards.md), and [docs/plan-interactive.md](../../docs/plan-interactive.md). Grilling tickets use the grilling and domain-modeling skills. Research tickets use the research skill.

Tracker: local markdown in this directory. This repo has no `docs/agents/issue-tracker.md`.

## Decisions so far

- [How would a page leave this motherboard?](issues/02-desk-ethernet.md): No protection domain can transmit a page; the RTL8852BE probe only reads config space (`10ec:b852`), and native e1000e has no recorded device identity.
- [What can legacy boot scan out on this RX 560?](issues/01-rx560-scanout.md): Legacy Limine leaves VGA text at `0xb8000`; a linear framebuffer needs a VBE mode set on that same CSM entry, and the address is unknown until a boot reads it.
- [What counts as seeing a page?](issues/03-page-class.md): The typed `https` public name, first load in a fresh profile, matching the shared behavior of the stable Firefox and Chrome named when the spec is written. `lerux-html` and `lerux-web` grow to that.
- [What chrome is on the screen?](issues/04-chrome.md): One address line above the page. `browser-ui` paints it, presents the screen, and takes the keyboard. A failure is one of five fixed lines in the page area.

## Not yet specified

- Bitmap size, pixel format, and who maps the framebuffer, once a scanout path is committed. If the spec commits to the VESA linear framebuffer, a legacy boot still has to read this card's mode-info block. That boot is not a ticket until the path is chosen. The address line's height in pixels waits with that.
- Which other protection domains share the image (COM1 shell, filesystem, TLS proxy, network driver).
- The onboard Ethernet's device identity, if the spec takes a driver. The repo only names the RTL8852BE configuration-space probe (`10ec:b852`). `e1000e` has no recorded device id.
- How the spec is demonstrated before a metal boot. QEMU may be a stand-in; it is not the product.

## Out of scope

- Porting Ladybird or embedding Servo. This map keeps `lerux-html` and `lerux-web`.
- Signing in, and granting a permission. A login wall and an unanswered prompt are the page.
- A script-opened window, or any second surface.
- Browser extensions.
- A digital rights management module. `wasmtime` and the WebAssembly Micro Runtime stay out too.
- A window manager, Wayland, or a second graphical application.
- Building the browser. The map ends at the spec.
- QEMU as the product.
- Extra tabs, or a second `web-content`.
- Disk, downloads, bookmarks, and history.
- The Intel HD Graphics 4600. Nothing is plugged into it, and it is not the boot VGA device.
- The agent protection domain.
- Raspberry Pi HDMI.
