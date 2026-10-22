# Record the onboard Ethernet device identity

Type: task
Label: wayfinder:task
Status: resolved

## Question

[Does the spec include the Ethernet driver?](06-ethernet-in-spec.md) takes the onboard Ethernet port and records its device identity before the driver is named. No file records a vendor id, device id, bus address, or base-address register for that port. The plans call the follow-on `e1000e` and do not name a part. The Realtek `10ec:b852` behind root port `00:1c.6` is the RTL8852BE, and it is not this port.

This is a read of the desk's live configuration space. It does not need a lerux boot. Record the result on this ticket.

1. Find the onboard Ethernet controller. Skip `10ec:b852` and root port `00:1c.6`.
2. Record the vendor, device, revision, subsystem, bus address, and base-address registers.
3. Stop there. Do not name the driver from this record, and do not add a part name the repo does not already contain.

## Answer

Read on 2026-10-22 from the live configuration space. The onboard Ethernet controller is bus address `00:19.0`. The firmware device name is `Onboard LAN`. It is not behind root port `00:1c.6`. The Realtek `10ec:b852` at `05:00.0` stays the skipped device.

Vendor `8086`, device `153b`, revision `0`. Subsystem vendor `1458`, subsystem device `e000`. Class `0x020000`.

Base address registers: memory `0xf7e00000` for 128 KiB, memory `0xf7e3c000` for 4 KiB, and I/O `0xf080` for 32 bytes. The other three base address registers are zero.
