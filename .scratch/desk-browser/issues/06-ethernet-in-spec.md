# Does the spec include the Ethernet driver?

Type: grilling
Label: wayfinder:grilling
Status: resolved
Blocked by: 02

## Question

[How would a page leave this motherboard?](02-desk-ethernet.md) found that nothing on this board can transmit a page. The only controller the repo can name is the Realtek RTL8852BE (`10ec:b852`), and that image only reads its configuration header. The onboard Ethernet the plans call `e1000e` has no recorded device identity.

Does this spec include a driver that can fetch — which means first identifying the controller this desk would use — or does a working network stay a precondition outside this spec?

## Answer

The spec includes a driver that can fetch a public page. The controller is the onboard Ethernet port. Its device identity is recorded before the driver is named. The RTL8852BE probe stays a configuration-space read and is not this path.

The new protection domain owns that controller and serves `net-server`, as the virtio network driver does on QEMU. `request-server` remains the only fetcher. `browser-ui` and `web-content` do not map the controller. Name lookup, Transport Layer Security, and the page fetch stay with the protection domains that already do that work.

The link uses Dynamic Host Configuration Protocol. The offer's IPv4 address, prefix, gateway, and Domain Name System resolver are the ones the spec uses. An offer with no gateway or no resolver, or a link that does not come up, means the page cannot be fetched.
