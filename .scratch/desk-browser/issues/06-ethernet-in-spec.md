# Does the spec include the Ethernet driver?

Type: grilling
Label: wayfinder:grilling
Status: open
Blocked by: 02

## Question

[How would a page leave this motherboard?](02-desk-ethernet.md) found that nothing on this board can transmit a page. The only controller the repo can name is the Realtek RTL8852BE (`10ec:b852`), and that image only reads its configuration header. The onboard Ethernet the plans call `e1000e` has no recorded device identity.

Does this spec include a driver that can fetch — which means first identifying the controller this desk would use — or does a working network stay a precondition outside this spec?
