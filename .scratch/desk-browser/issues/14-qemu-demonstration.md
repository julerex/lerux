# How is the spec demonstrated before a metal boot?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

The desk browser image is the metal product. Its framebuffer address and onboard Ethernet identity are already recorded from this desk. QEMU's VGA BIOS is a different device. Continuous integration stays on the smoke certificate authority and does not fetch the public web.

How is this spec demonstrated before a metal boot?

## Answer

Before a metal boot, a separate QEMU image boots the browser protection domains on the legacy Limine entry. Limine requests 1024 by 768 at 32 bits per pixel. `browser-ui` maps that linear framebuffer. The image contains `console-driver`, `serial-driver`, `log-server`, `browser-ui`, one `web-content`, `request-server`, `tls-proxy`, `net-server`, and the existing x86 virtio network driver. That driver serves `net-server`. The onboard Ethernet driver stays on the desk image. `tls-proxy` on the QEMU image trusts the smoke certificate authority. The Phase 82 shell tests stay the shell tests.

The address line on this image accepts `https://host:8443/`. `lerux https-one` serves three pages. PS/2 injection types `https://host:8443/desk.html` and Enter loads it. Tab focuses its link, and Enter loads `https://host:8443/desk-link.html`. Tab focuses the text field named `q`, the keys type `hi`, and Enter submits the GET form. The loaded address is `https://host:8443/desk-form.html?q=hi`.

After each load paints, COM1 records that address and the first non-white pixel. The framebuffer address in the log is QEMU's. The metal boot remains the product: a public name and the public trust anchors, on this desk's framebuffer and onboard Ethernet device. The existing `fixture.html` and `paint.html` smokes stay as they are.
