# What counts as seeing a page?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

`lerux-html` parses `html`, `head`, `body`, `title`, `p`, `h1`–`h3`, `a`, `div`, `span`, `ul`, `ol`, `li`, `pre`, `code`, `strong`, `em`, `img`, and `style`. `lerux-web` paints `color`, `background-color`, `font-size`, `display` (`block`, `inline`, `none`), `margin`, `padding`, and `width` into an RGB bitmap. A fetch goes through `request-server` and the smoke certificate authority to a host fixture (`https://host:8443/paint.html` on the QEMU browser image). The smoke fixtures are `support/browser/fixture.html`, `support/browser/subset.html`, and `support/browser/paint.html`.

The destination is to type an address on this desk and see that page. What address, served from where, is enough for the spec? In particular: is a page in today's subset, fetched from a machine on this desk's Ethernet, enough, or does the spec require a public URL (name lookup, a public trust anchor, and a larger engine)?

## Answer

The spec is satisfied when the monitor shows the public page at the address that was typed.

The address uses `https`. Its host is a public name resolved through the Domain Name System. The path, query, and fragment belong to the page. The fragment is visible to scripts and is not sent in the request. An omitted port means 443. An explicit port stays. The request carries `Accept-Language: en`. The certificate chains to the public trust anchors shipped with the reference browsers. Redirects are followed, and the final address stays `https`.

Seeing the page means the first load in a fresh profile: empty cookies, empty storage, and no saved credentials. The bitmap includes the text, the images, the layout from the page's CSS, the content its scripts insert, WebAssembly those scripts run, frames, fonts, and media, including subresources from other `https` origins. Mixed content is omitted. A standalone image or a PDF is shown the way the reference browsers show it. A response those browsers would only download is not a shown page. A login wall counts. A permission prompt left unanswered counts. Media that plays only with a digital rights management module does not have to play. Where the two reference browsers disagree, only the shared behavior counts. The first load is not served from an already installed service worker.

The reference browsers are the stable Firefox and Chrome releases current in the month the spec is written, with no extensions and no digital rights management module. The spec names those releases. Later releases do not move it.

`lerux-html` and `lerux-web` grow until they can do this. Ladybird, Servo, `wasmtime`, and the WebAssembly Micro Runtime stay out.

Signing in, granting a permission, and following a link are outside this ticket. A script-opened window does not get a second surface.
