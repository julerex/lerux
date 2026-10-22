# Which protection domains share the desk browser image?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

[Does the spec include the Ethernet driver?](06-ethernet-in-spec.md) puts the onboard Ethernet driver, `net-server`, `request-server`, and Transport Layer Security on the fetch path. The screen is `browser-ui`, one `web-content`, and `console-driver` for the keyboard. The Phase 82 image also has a shell, `log-server`, and `serial-driver`. The COM1 shell and the filesystem were still open.

Which other protection domains share the desk browser image?

## Answer

The desk browser image contains `console-driver`, `serial-driver`, `log-server`, `browser-ui`, one `web-content`, `request-server`, `tls-proxy`, `net-server`, and the new onboard Ethernet driver. That driver owns bus address `00:19.0` and serves `net-server`. `browser-ui` and `web-content` do not map the controller.

`console-driver` owns the keyboard and forwards key-downs to `browser-ui`. It does not paint the screen. `browser-ui` owns the address line and maps the linear framebuffer. `serial-driver` and `log-server` keep COM1 as the log. There is no prompt on COM1.

`tls-proxy` holds the public trust anchors. There is no disk driver and no filesystem. The shell, `display-server`, and the agent are not on this image. The Phase 82 shell image remains the QEMU console test.
