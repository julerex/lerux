# How do desk keystrokes reach browser-ui?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

Phase 82's `console-driver` owns the PS/2 keyboard (data `0x60`, command `0x64`, IRQ1) and, when one is armed, the USB HID boot keyboard on the xHCI controller at PCI `00:14.0`. The shell reads those keys. [What chrome is on the screen?](04-chrome.md) decided that the Phase 82 keyboard is the only input, that `browser-ui` receives it, and that keys for the page are delivered to `web-content`.

Which protection domain reads this desk's keyboard, and how do those events reach `browser-ui`?

## Answer

`console-driver` keeps the PS/2 ports and the USB HID boot keyboard. It forwards each key to `browser-ui`. When a USB boot keyboard is armed, its keys are the ones forwarded and PS/2 is left unread. `browser-ui` does not map `0x60`, `0x64`, or the xHCI controller. The shell does not receive those keys.

Each forwarded event is a key-down. It names the key and whether Ctrl is held. Left Ctrl and Right Ctrl both count. A Ctrl press by itself is not an event. Shift and Caps Lock select the character a letter inserts, and that state stays in `console-driver`. Key-up is not forwarded.

A held key repeats until it is released. The USB boot keyboard repeats at the PS/2 typematic rate, and `console-driver` sends those repeats as further key-downs.
