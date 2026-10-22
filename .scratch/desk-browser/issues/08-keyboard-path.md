# How do desk keystrokes reach browser-ui?

Type: grilling
Label: wayfinder:grilling
Status: open

## Question

Phase 82's `console-driver` owns the PS/2 keyboard (data `0x60`, command `0x64`, IRQ1) and, when one is armed, the USB HID boot keyboard on the xHCI controller at PCI `00:14.0`. The shell reads those keys. [What chrome is on the screen?](04-chrome.md) decided that the Phase 82 keyboard is the only input, that `browser-ui` receives it, and that keys for the page are delivered to `web-content`.

Which protection domain reads this desk's keyboard, and how do those events reach `browser-ui`?
