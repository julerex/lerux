# What chrome is on the screen?

Type: grilling
Label: wayfinder:grilling
Status: open

## Question

`browser-ui` today sends serial `open <url>` and does not paint chrome. Phase 76 left chrome off the framebuffer on purpose. The destination is that the browser is the whole screen and the address is typed on this desk, from the Phase 82 keyboard.

While a page is up, what does `browser-ui` show, and which keystrokes does it accept? Include the address line, how a failure is shown when the fetch or the engine cannot paint the page, and whether anything else shares the bitmap.
