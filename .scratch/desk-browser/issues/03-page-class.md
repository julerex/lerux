# What counts as seeing a page?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

`lerux-html` parses `html`, `head`, `body`, `title`, `p`, `h1`–`h3`, `a`, `div`, `span`, `ul`, `ol`, `li`, `pre`, `code`, `strong`, `em`, `img`, and `style`. `lerux-web` paints `color`, `background-color`, `font-size`, `display` (`block`, `inline`, `none`), `margin`, `padding`, and `width` into an RGB bitmap. A fetch goes through `request-server` and the smoke certificate authority to a host fixture (`https://host:8443/paint.html` on the QEMU browser image). The smoke fixtures are `support/browser/fixture.html`, `support/browser/subset.html`, and `support/browser/paint.html`.

The destination is to type an address on this desk and see that page. What address, served from where, is enough for the spec? In particular: is a page in today's subset, fetched from a machine on this desk's Ethernet, enough, or does the spec require a public URL (name lookup, a public trust anchor, and a larger engine)?

## Answer

Seeing a page means `web-content` paints the UTF-8 body of the fetch, and that body stays inside the subset `lerux-html` and `lerux-web` already implement.

The elements are `html`, `head`, `body`, `title`, `p`, `h1` through `h3`, `a`, `div`, `span`, `ul`, `ol`, `li`, `pre`, `code`, `strong`, `em`, `img`, and `style`. The properties are `color`, `background-color`, `font-size`, `display` (`block`, `inline`, and `none`), `margin`, `padding`, and `width`, in block flow. An `img` paints a fixed 16 by 16 gray box. The engines stay as they are. An unknown tag is skipped and its text is kept. An unknown CSS property is dropped. JavaScript, a decoded image, and further CSS stay out. A body outside the subset is outside the promise.

The address is an `https` URL. It may name a public DNS name or an IPv4 address, then a path. An omitted port means 443. An omitted path means `/`. An explicit port or path is used as written. A public name is looked up through `net.dns`, and the connection uses the IPv4 answer. The static aliases `host` and `dns` still win when they match. A name with no IPv4 address is a failed fetch.

The fetch follows an `https` redirect. The last body is the page. The chain stops at 5 hops, on a repeated URL, or when the next URL is not `https`. It does not fetch an image, a stylesheet, or a script.

This desk trusts the smoke certificate authority and the `webpki-roots` set. The certificate has to name the URL host. An IPv4 URL that uses the smoke authority needs that address in the certificate. The check uses this desk's CMOS clock. A build that leaves `webpki-roots` off, including the fetch smoke, trusts only the smoke authority and keeps the fixed instant 2026-09-21 14:13:20 UTC.

An `a` is painted as text. Where the `title` is drawn, how a link is activated, and how a failure is shown stay on [What chrome is on the screen?](04-chrome.md).
