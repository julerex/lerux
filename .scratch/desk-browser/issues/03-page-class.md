# What counts as seeing a page?

Type: grilling
Label: wayfinder:grilling
Status: open

## Question

`lerux-html` parses `html`, `head`, `body`, `title`, `p`, `h1`–`h3`, `a`, `div`, `span`, `ul`, `ol`, `li`, `pre`, `code`, `strong`, `em`, `img`, and `style`. `lerux-web` paints `color`, `background-color`, `font-size`, `display` (`block`, `inline`, `none`), `margin`, `padding`, and `width` into an RGB bitmap. A fetch goes through `request-server` and the smoke certificate authority to a host fixture (`https://host:8443/paint.html` on the QEMU browser image). The smoke fixtures are `support/browser/fixture.html`, `support/browser/subset.html`, and `support/browser/paint.html`.

The destination is to type an address on this desk and see that page. What address, served from where, is enough for the spec? In particular: is a page in today's subset, fetched from a machine on this desk's Ethernet, enough, or does the spec require a public URL (name lookup, a public trust anchor, and a larger engine)?
