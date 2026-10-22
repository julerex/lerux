# Desk browser

Written 2026-10-22. This is the spec for booting lerux on this desk and showing one public page on the monitor. The decisions are recorded under [`.scratch/desk-browser/`](../.scratch/desk-browser/map.md). Building the browser is later work.

The desk is the Gigabyte Z97-D3H (`pc_z97_d3h`). The monitor cable is on the motherboard HDMI port. The product is a metal boot of the image below. Terms live in [`context.md`](context.md). Scanout is [ADR-013](decisions/013-desk-intel-scanout.md), which amends [ADR-009](decisions/009-interactive-surface.md) for this board. QEMU images still present through `display-server`.

## Satisfaction

The spec is satisfied when the monitor shows the public page at the address that was typed.

That address uses `https`. Its host is a public name: a hostname resolved through the Domain Name System, using the resolver from the Dynamic Host Configuration Protocol offer. A literal address, or a name that resolves only from a local hosts table, is rejected. The path, query, and fragment belong to the page. The fragment is visible to scripts and is omitted from the request. An omitted port means 443. An explicit port stays. The request carries `Accept-Language: en`. Redirects are followed, and the final address stays `https`.

The typed address is the first load in a fresh profile: empty cookies, empty storage, and no saved credentials. Cookies and storage a page sets during that boot stay for a later load on this surface. The next boot starts empty. The first load is served from the network. It is not served from an already installed service worker.

## Reference browsers

The reference browsers are Firefox 158.0.1, first offered on the Release channel on 20 October 2026, and Chrome 156.0.8078.47, the Linux stable build promoted on 20 October 2026. Both are desktop builds with no extensions and no digital rights management module. Later releases do not move this spec. Where the two disagree, only the shared behavior counts.

A certificate is accepted when both of those builds would accept it, using the trust anchors each ships and no added certificates. `tls-proxy` on the desk image holds those anchors.

`lerux-html` and `lerux-web` grow until `web-content` can show this page. Ladybird, Servo, `wasmtime`, and the WebAssembly Micro Runtime stay out.

## What the page shows

The bitmap includes the text, the images, the layout from the page's CSS, the content its scripts insert, WebAssembly those scripts run, frames, fonts, and media, including subresources from other `https` origins. Mixed content is omitted. A standalone image or a PDF is shown the way the reference browsers show it. A response both would only download produces no page bitmap. A login wall counts. A permission prompt left unanswered counts. Media that plays only with a digital rights management module does not have to play.

An HTTP response that the engine paints, including an error status with a body, is the page.

## The screen

`browser-ui` paints one address line and presents the screen. The page fills every pixel under that line. `web-content` paints only that rectangle, into a shared bitmap of 1024 by 752. That bitmap is not the framebuffer, and `web-content` does not map the framebuffer. The shell, the agent, a tab strip, and buttons stay off the screen.

Before any page has loaded, the line is focused and empty, and the page area is blank. The desk keyboard is the only input.

### Address line

The address line is the top 16 rows of the framebuffer. A long address scrolls horizontally so the cursor stays visible.

Ctrl+L focuses the line and selects its text. While the line is focused it accepts printable characters, Backspace, Left, and Right. There is no clipboard paste. A selected address is replaced by a printable character or Backspace. Left places the cursor at the start of the selection, and Right places it at the end. Any other key is ignored.

Enter on an empty line does nothing. A name with no scheme is loaded as `https`, and the line shows that `https` address once Enter is accepted. An `http` scheme, or a host that is not a public name, paints `not a public https address` in the page area, and `web-content` is not asked to load it.

Enter abandons a load already in flight. The page on screen stays until the new load paints or fails. While that load runs, focus is on the page unless the line is focused again.

Escape abandons a load in flight, and it does not reach the page while a load is in flight. If a page is up, Escape shows that page's address and returns focus to the page. If the page area is blank or showing a failure, Escape leaves focus on the line.

When a page is up and the line is not focused, the line shows that page's final `https` address, including a redirect and a fragment. A load that finishes while the line is focused updates the page area and leaves the line's text and focus alone.

When the fetch or the engine produces no page bitmap, `browser-ui` paints one of these lines in the page area:

- `name lookup failed`
- `certificate rejected`
- `connection failed`
- `page could not be painted`

The line keeps the failed address. If the line was not focused, it receives focus and the address is selected. A link that does not come up, or a Dynamic Host Configuration Protocol offer with no gateway, is `connection failed` when a load needs the network. An offer with no resolver is `name lookup failed`.

## Links and scrolling

While the page is focused, Enter activates the focused link and the address line shows the `https` address that will be loaded. If no link is focused, Enter does not navigate. Other keys still go to the page.

A link with no host is resolved against the current page's address first. A link that only changes the fragment does not start a fetch. The address line shows the new fragment, and the page scrolls to it. A link that asks for a new window or tab loads on this surface. A `javascript` link runs as script and does not start a load.

The new page replaces the current one. There is no back stack. A failed load follows the address-line failure rules.

A page taller than 1024 by 752 scrolls inside that rectangle, including the first page. It is not scaled to fit. The keys that move the scroll are the ones both reference browsers use for that focused page.

## Forms

Submitting a form loads the public page the response names, on this surface, under the same load rules as a link. The new page replaces the current one. There is no back stack. A failed load follows the address-line failure rules.

Focus is one control. Tab moves to the next link or control, wraps from the last to the first, and skips a control the reference browsers would skip. A page loads with nothing focused unless the page asks for that focus.

A single-line text field accepts a character when both reference browsers would, and refuses it when both would. Backspace, Left, and Right edit it. There is no clipboard paste. A long field scrolls horizontally so the cursor stays visible. Enter in a multiline text field inserts a newline. Up and Down change a focused drop-down list. Space toggles a checkbox. Space, Up, or Down selects a radio button and clears the others in its group. Space toggles a focused row of a multi-select list.

Enter submits the focused control's form when both reference browsers would submit it. That is a single-line text field whose form has a submit button, or a form with only one single-line text field. The button they would activate is included. Space or Enter on a focused submit button submits that form and includes that button. Space or Enter on a focused reset button restores the form's initial values. Any other focused button does neither. When those browsers would not submit, the page stays as it is.

The form's action is resolved the same way as a link. An empty action is the current page's address. A GET form puts the fields in the query. A POST form sends the body those browsers would send. The address line shows the `https` address that will be loaded, and after the load it shows the final address, including a redirect. A fragment-only action scrolls to that fragment and does not fetch. A `javascript` action runs as script and does not load. A target that asks for a new window loads on this surface. An `http` action, or a host that is not a public name, is rejected the way the address line rejects it. When the page's script cancels the submit, the script runs and the page stays.

A password field is shown and ignores keys. A form that contains one stays as it is. A file field is shown and stays empty, and the submit includes the other fields. When those browsers would refuse the submit because a file is required, the page stays. A disabled control is left out of the submit.

Signing in, and granting a permission, stay out of this spec. The login wall and the unanswered prompt are the page.

## Keyboard

`console-driver` owns the PS/2 ports (`0x60`, `0x64`, IRQ1) and, when one is armed, the USB HID boot keyboard on the xHCI controller at PCI `00:14.0`. It forwards each key to `browser-ui`. When a USB boot keyboard is armed, its keys are the ones forwarded and the PS/2 ports are left unread. `browser-ui` does not map `0x60`, `0x64`, or the xHCI controller. The shell does not receive those keys. Keys for the page are delivered to `web-content`.

Each forwarded event is a key-down. It names the key and whether Ctrl is held. Left Ctrl and Right Ctrl both count. A Ctrl press by itself is not an event. Shift and Caps Lock select the character a letter inserts, and that state stays in `console-driver`. Key-up is not forwarded.

A held key repeats until it is released. The USB boot keyboard repeats at the PS/2 typematic rate, and `console-driver` sends those repeats as further key-downs.

## Scanout

The desk scans out through the Intel HD Graphics 4600 (`00:02.0`) on the motherboard HDMI port. Firmware Peripherals → Initial Display Output is `IGFX`, and Intel Processor Graphics stays enabled. After the reboot on 2026-10-22, firmware reports that device as the boot VGA device. The Sapphire RX 560 stays in the machine and stays unused. Once `IGFX` is the primary device, the VGA text page at `0xb8000` is scanned out on this same port.

The path is the VESA linear framebuffer on the legacy Limine entry. Limine asks for 1024 by 768 at 32 bits per pixel and sets the mode before the kernel starts. The spec uses the linear direct-color mode the mode-info block reports, including a different size or depth. `browser-ui` receives that block's address, pitch, width, height, and pixel masks in a region it maps. It does not call the firmware. lerux does not program the Intel display engine or the Polaris display engine.

`browser-ui` maps the framebuffer. It clears the framebuffer, paints the address line in the top 16 rows, and copies the page bitmap into the top-left of the rows beneath. Each row advances by the mode-info pitch, and each pixel uses that block's masks. A larger mode does not grow the 1024 by 752 bitmap. Pixels that do not fit are left uncopied, and the rest of a larger mode stays clear. A mode shorter than 16 rows, or no linear direct-color mode, means this path failed. The spec does not then program a display engine. `display-server` stays on the QEMU path.

Limine 12.9.0, on the legacy entry on 2026-10-22, published this mode. The height is 768 rows, so the path holds.

| Field | Value |
|-------|-------|
| Physical address | `0xd0000000` |
| Pitch | 4096 bytes |
| Size | 1024 by 768 |
| Depth | 32 bits per pixel |
| Red | 8 bits at shift 16 |
| Green | 8 bits at shift 8 |
| Blue | 8 bits at shift 0 |

The Multiboot 2 framebuffer tag is type 8, size 40, framebuffer type 1. Its bytes are `08 00 00 00 28 00 00 00 00 00 00 d0 00 00 00 00 00 10 00 00 00 04 00 00 00 03 00 00 20 01 00 00 10 08 08 08 00 08 00 00`. The address matches this machine's Intel graphics prefetchable region. It is used because the tag names it. The probe and the tag are recorded in [Read this card's VESA mode-info block](../.scratch/desk-browser/issues/09-mode-info.md).

## Fetch

`request-server` is the only fetcher. Name lookup, Transport Layer Security, and the page fetch stay with the protection domains that already do that work. `browser-ui` and `web-content` do not map the controller.

The controller is the onboard Ethernet port. A new protection domain owns it and serves `net-server`, as the virtio network driver does on QEMU. The driver stays unnamed here. The identity was read from configuration space on 2026-10-22.

| Field | Value |
|-------|-------|
| Bus address | `00:19.0` |
| Vendor, device, revision | `8086`, `153b`, revision `0` |
| Subsystem | `1458:e000` |
| Class | `0x020000` |
| Memory | `0xf7e00000`, 128 KiB |
| Memory | `0xf7e3c000`, 4 KiB |
| I/O | `0xf080`, 32 bytes |

The other three base-address registers are zero. The RTL8852BE probe (`10ec:b852` at `05:00.0`, behind root port `00:1c.6`) stays a configuration-space read and is not this path.

The link uses Dynamic Host Configuration Protocol. The offer's IPv4 address, prefix, gateway, and Domain Name System resolver are the ones the spec uses.

## Image

The desk image contains these protection domains.

| Protection domain | Role on this image |
|-------------------|--------------------|
| `console-driver` | Owns the desk keyboard and forwards key-downs. Does not paint. |
| `serial-driver` | COM1, with `log-server`. |
| `log-server` | The log. There is no prompt on COM1. |
| `browser-ui` | Address line, keyboard dispatch, and the linear framebuffer. |
| `web-content` | One protection domain. `lerux-html` and `lerux-web`. Paints the 1024 by 752 bitmap. |
| `request-server` | The only fetcher. |
| `tls-proxy` | Public trust anchors from the reference browsers. |
| `net-server` | Served by the onboard Ethernet driver. |
| onboard Ethernet driver | Owns `00:19.0`. |

There is no disk driver and no filesystem. The shell, `display-server`, and the agent are not on this image. The Phase 82 shell image remains the QEMU console test (`x86_64_generic_console` and `x86_64_generic_usb_kbd`).

## Demonstration before a metal boot

A separate QEMU image boots the browser protection domains on the legacy Limine entry. Limine requests 1024 by 768 at 32 bits per pixel. `browser-ui` maps that linear framebuffer. The image contains `console-driver`, `serial-driver`, `log-server`, `browser-ui`, one `web-content`, `request-server`, `tls-proxy`, `net-server`, and the existing x86 virtio network driver. That driver serves `net-server`. The onboard Ethernet driver stays on the desk image. `tls-proxy` on the QEMU image trusts the smoke certificate authority. The Phase 82 shell tests stay the shell tests.

The address line on this image accepts `https://host:8443/`. `lerux https-one` serves three pages. PS/2 injection types `https://host:8443/desk.html` and Enter loads it. Tab focuses its link, and Enter loads `https://host:8443/desk-link.html`. Tab focuses the text field named `q`, the keys type `hi`, and Enter submits the GET form. The loaded address is `https://host:8443/desk-form.html?q=hi`.

After each load paints, COM1 records that address and the first non-white pixel. The framebuffer address in the log is QEMU's. The existing `fixture.html` and `paint.html` smokes stay as they are.

This image is the demonstration. The metal boot remains the product: a public name and the public trust anchors, on this desk's framebuffer and onboard Ethernet device.

## Out of scope

- Porting Ladybird or embedding Servo.
- A Polaris or Intel display-engine modeset.
- Programming the Sapphire RX 560.
- Using the RTL8852BE as the fetch path.
- Signing in, and granting a permission.
- A script-opened window, or any second surface.
- Browser extensions.
- A digital rights management module.
- A window manager, Wayland, or a second graphical application.
- Extra tabs, or a second `web-content`.
- Disk, downloads, bookmarks, and history.
- The agent protection domain.
- Raspberry Pi HDMI.
- QEMU as the product.

## Record

| Decision | Record |
|----------|--------|
| What counts as seeing a page | [03-page-class.md](../.scratch/desk-browser/issues/03-page-class.md) |
| What chrome is on the screen | [04-chrome.md](../.scratch/desk-browser/issues/04-chrome.md) |
| Scanout path | [05-scanout-path.md](../.scratch/desk-browser/issues/05-scanout-path.md), [ADR-013](decisions/013-desk-intel-scanout.md) |
| Intel graphics | [12-intel-scanout.md](../.scratch/desk-browser/issues/12-intel-scanout.md) |
| Mode-info block | [09-mode-info.md](../.scratch/desk-browser/issues/09-mode-info.md) |
| Ethernet driver in the spec | [06-ethernet-in-spec.md](../.scratch/desk-browser/issues/06-ethernet-in-spec.md) |
| Onboard Ethernet identity | [10-onboard-ethernet-id.md](../.scratch/desk-browser/issues/10-onboard-ethernet-id.md) |
| Following a link | [07-follow-link.md](../.scratch/desk-browser/issues/07-follow-link.md) |
| Keyboard path | [08-keyboard-path.md](../.scratch/desk-browser/issues/08-keyboard-path.md) |
| Submitting a form | [11-submit-form.md](../.scratch/desk-browser/issues/11-submit-form.md) |
| Image membership | [13-desk-image.md](../.scratch/desk-browser/issues/13-desk-image.md) |
| QEMU demonstration | [14-qemu-demonstration.md](../.scratch/desk-browser/issues/14-qemu-demonstration.md) |
| Earlier research | [01-rx560-scanout.md](../.scratch/desk-browser/issues/01-rx560-scanout.md), [02-desk-ethernet.md](../.scratch/desk-browser/issues/02-desk-ethernet.md) |
