# What chrome is on the screen?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

`browser-ui` today sends serial `open <url>` and does not paint chrome. Phase 76 left chrome off the framebuffer on purpose. The destination is that the browser is the whole screen and the address is typed on this desk, from the Phase 82 keyboard.

While a page is up, what does `browser-ui` show, and which keystrokes does it accept? Include the address line, how a failure is shown when the fetch or the engine cannot paint the page, and whether anything else shares the bitmap.

## Answer

`browser-ui` paints one address line and presents the screen. The page fills every pixel under that line. `web-content` paints only that rectangle. The shell, the agent, a tab strip, and buttons stay off the screen.

Before any page has loaded, the line is focused and empty, and the page area is blank. The Phase 82 keyboard is the only input. `browser-ui` receives it. Keys for the page are delivered to `web-content`.

Keys reach the page, except Ctrl+L, which focuses the address line and selects its text. While the line is focused it accepts printable characters, Backspace, Left, and Right. There is no clipboard paste. A selected address is replaced by a printable character or Backspace. Left places the cursor at the start of the selection, and Right places it at the end. Any other key is ignored. A long address scrolls horizontally so the cursor stays visible.

Enter on an empty line does nothing. A name with no scheme is loaded as `https`, and the line shows that `https` address once Enter is accepted. An `http` scheme, or a host that is not a public name, paints `not a public https address` and `web-content` is not asked to load it. Enter abandons a load already in flight. The page on screen stays until the new load paints or fails. While that load runs, focus is on the page unless the line is focused again.

Escape abandons a load in flight, and it does not reach the page while a load is in flight. If a page is up, Escape shows that page's address and returns focus to the page. If the page area is blank or showing a failure, Escape leaves focus on the line.

When a page is up and the line is not focused, the line shows that page's final `https` address, including a redirect and a fragment. A load that finishes while the line is focused updates the page area and leaves the line's text and focus alone.

When the fetch or the engine produces no page bitmap, `browser-ui` paints one of these lines in the page area: `name lookup failed`, `certificate rejected`, `connection failed`, or `page could not be painted`. The line keeps the failed address. If the line was not focused, it receives focus and the address is selected. An HTTP response that the engine paints, including an error status with a body, is the page.

The line's height in pixels waits for the scanout path. Following a link stays on its own ticket.
