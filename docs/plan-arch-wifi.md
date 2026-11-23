# Arch Wi-Fi session

Last updated: 2026-11-23.

**Status:** Milestone 1 is done. Milestones 2–4 are not started.

**Goal:** The Swift console accepts the wireless session from the Arch installation guide. The operator types `ip link`, then `iwctl`, then `ping ping.archlinux.org`, and the last command prints a reply. The radio stays `swift-mt7921`.

**Why a separate plan:** [plan-swift-wifi.md](plan-swift-wifi.md) is the firmware sequence and the route to a link. [plan-lionsos.md](plan-lionsos.md) Milestone 27 and [plan-metal-parity.md](plan-metal-parity.md) Milestone 8 keep the metal exit: a DHCP address, then `ping ok`. This plan is the commands the operator types to get there. A host test of those commands does not close either exit.

## The session Arch types

The session is the installation guide, checked on 2026-11-15. NetworkManager is a later client of the same station and is outside this plan.

The guide’s “Connect to the internet” steps for a laptop are:

1. `ip link`, so the interface is listed.
2. A check that the card is not blocked with `rfkill`.
3. Authenticate with `iwctl`.
4. DHCP and DNS from systemd-networkd and systemd-resolved. The guide types no DHCP command. The installation image starts those services for the operator. An installed system does not.
5. `ping ping.archlinux.org`.

`iwctl(1)` is both an interactive prompt and a one-shot command. The interactive session the man page shows is:

```
iwctl
device list
station DEVICE scan
station DEVICE get-networks
station DEVICE connect SSID
```

The prompt prefix is `[iwd]# `. `station DEVICE scan` prints nothing. A passphrase of 8 to 63 characters is requested when the network needs one. The iwd wiki’s rejection for any other length is `PMK generation failed. Ensure Crypto Engine is properly configured`. A name that contains a space is passed in double quotes.

On this image the one station is `wlan0`. `device list` prints that name, and the following commands use it.

`ip link` shows the interface name, the flags, and the hardware address. It does not show the lease. The lease is visible to `ping`.

## How Arch binds a radio

Arch keeps one driver per chip family and bus. The PCI, USB, or SDIO core matches a vendor and device id and loads that module. The driver then reads the chip id, loads that chip’s firmware, and programs its registers. The probe does not produce a generic configuration one driver can use for every chip.

On this machine the module is `mt7921e` from kernel 7.2.8-9-omarchy. `modinfo` reports the description `MediaTek MT7921E (PCIe) wireless driver` and these aliases:

- `14c3:7961`
- `14c3:0608`
- `14c3:7922`
- `0b48:7922`
- `14c3:0616`
- `14c3:7920`
- `14c3:7902`

It depends on `mt76-connac-lib`, `mt7921-common`, `mt792x-lib`, `mt76`, and `rfkill`. The same family on USB is `mt7921u`. The same family on SDIO is `mt7921s`. Intel is `iwlwifi`, a different module. The firmware files `modinfo` names for this radio are `mediatek/WIFI_MT7961_patch_mcu_1_1_hdr.bin` and `mediatek/WIFI_RAM_CODE_MT7961_1.bin`, and the `1a` pair beside them.

Userspace never programs the chip. `iwctl` talks to `iwd` over D-Bus. `iwd` talks nl80211. cfg80211 and mac80211 sit on `mt7921e`. After the link is up, systemd-networkd takes the DHCP lease and systemd-resolved takes the DNS servers from that lease.

## What lerux does today

The Swift template [`userspace/systems/templates/console-swift.system.template`](../userspace/systems/templates/console-swift.system.template) runs `sddf-shell`. After `lerux shell ready` the only radio command is `setup-wifi` in [`userspace/pds/sddf-shell/src/main.rs`](../userspace/pds/sddf-shell/src/main.rs). It posts one wildcard survey and prints `N name`, or the mark `rx`, `ev`, `ok`, `done`, and `fc` when frames arrived and no name was stored. A stored list continues with `ssid>`, the row number, `psk>`, and the passphrase. A finished handshake prints `psk ok`, then the address and `ping ok`.

`console-shell` still implements `3 wifi`, `ssid>`, and `psk>`. That program is not the shell in the Swift template.

`swift-mt7921` is the static form of `mt7921e`. It accepts `MT7921_IDS` (`14c3:7961` and `14c3:0608`) at `03:00.0` and refuses another id. `firmware_kind` treats chip ids `0x7922` and `0x7925` as unsupported and uses the 7961 images otherwise. The protection domain embeds the 7961 patch, the `1a` patch, the RAM image, and the `1a` RAM image. There is no module loader. USB and SDIO are not this image. The Z97 Ethernet path stays the I217-V. The Pi path stays GENET. The desk ISO maps the RTL8852BE and stops the PCI HCI. It does not scan or associate. `pc_z97_d3h_rtl8852be` stays a configuration-space probe.

The same protection domain scans, sends open-system authentication, associates, runs the software 4-way, and copies Ethernet frames onto the sDDF network queues ([`userspace/pds/swift-mt7921/src/join.rs`](../userspace/pds/swift-mt7921/src/join.rs)). `join.rs` leaves DHCP and ICMP to the shell. The shell already links smoltcp DHCP because `board-swift_sfx14_41g` pulls the fetch feature. The Swift command loop does not call `bring_up`. That feature has no DNS socket and no ICMP socket. The ICMP socket lives in `sddf-net-client`. The DNS socket is the browser feature.

The shared page is `wifi_key` at `0x1fd0000`. The shell’s channel 5 notifies the radio. The radio’s channel 3 is that notify. Survey and a posted passphrase already share it.

```
console keys
  sddf-shell            iwctl, ip link, ping
  wifi_key              survey, passphrase, station address
  swift-mt7921          scan, auth, assoc, eapol, ethernet queues
  smoltcp in the shell  DHCP, DNS, one ICMP echo
```

Arch’s stack underneath the same keystrokes is `iwctl`, `iwd`, nl80211, mac80211, `mt7921e`, then systemd-networkd and systemd-resolved.

## Global constraints

- Leave the firmware download order in [`userspace/pds/swift-mt7921/src/device.rs`](../userspace/pds/swift-mt7921/src/device.rs) as [plan-swift-wifi.md](plan-swift-wifi.md) left it. The register sequence stays the Linux v6.8 transcription. The running kernel on the development machine is 7.2.8-9-omarchy, and these milestones do not retarget `device.rs` to that tree.
- The shell and the radio keep the `wifi_key` page and the existing notify. These milestones add no D-Bus, no nl80211 socket, and no protection domain between them.
- Channel numbers stay the ones in [`support/profiles/swift.toml`](../support/profiles/swift.toml).
- The shell does not gain `lerux-logging`.
- `just check`, `just check-pd`, and `just iso` stay sequential. A host test does not close a metal exit.
- The iwctl line buffer is 128 bytes. The other shell commands stay at 80.
- A passphrase is used for the association in this boot. It is not written to a profile. The Swift disk stays read-only in [plan-metal-parity.md](plan-metal-parity.md) Milestone 9.

## Milestone 1 — The iwctl session

**Status:** done.

**Arch capability.** `iwctl` lists the station, scans, lists networks, and accepts a passphrase. `ip link` lists `wlan0`.

**Boards.** `swift_sfx14_41g` for the command table. The proof is a host test. The laptop stays off.

**Protection domains and templates.** `sddf-shell` on the Swift feature. `swift-mt7921` keeps the survey and the posted key. New lines are formatters in `lerux-swift` with a host test of the exact bytes, the same way `format_survey_report` already is. The shell prints those bytes.

**Steps.**

- After the sign-on prompt, `ip link` and `iwctl` are shell commands. `ping` arrives in Milestone 3.
- `iwctl` prints `[iwd]# ` and reads further lines until `exit` or byte `0x04`. `ip link` and, later, `ping` are typed outside that prompt, as the installation guide types them at the shell.
- The radio writes two fields on `wifi_key` when it reaches `wait`: a powered flag, and the six-byte station address it already uses in the authentication frame. The shell reads those fields. It does not parse the framebuffer. The survey list layout that `setup-wifi` still reads stays where it is. Station, beacon, scan frame, channel, host, and data ring are the Swift radio survey in [context.md](context.md#swift-radio-survey).
- `device list` prints a header and one row: name `wlan0`, the station address as colon-separated lowercase hex, Powered `on` when the flag is set, Adapter `phy0`, Mode `station`. When the flag is clear, Powered is `off`, and `scan`, `get-networks`, and `connect` print `Operation failed` and do not notify the radio.
- `station wlan0 scan` prints nothing. It posts the existing wildcard survey and notifies the radio.
- `station wlan0 get-networks` prints a header, `Network name`, `Security`, and `Signal`, then one row per stored name. Security is `psk` when the beacon’s RSN AKM list contains PSK, and `open` when the beacon has no RSN element. Any other RSN, including SAE only, is refused by `connect` in Milestone 2. `Signal` is empty until the page holds a dBm. An empty survey prints the header and no rows. The mark `rx`, `ev`, `ok`, `done`, and `fc` stays on panel row 47 and is not a row of this table.
- `station wlan0 connect NAME` prints `Type the network passphrase for NAME psk.` and `Passphrase: `, then writes the key page and notifies the radio. A passphrase outside 8 to 63 bytes prints `PMK generation failed. Ensure Crypto Engine is properly configured` and does not notify. A double-quoted name is accepted, including a name that contains a space. A device name other than `wlan0` prints `Device NAME not found`.
- `setup-wifi` and the `console-shell` menu `3 wifi` stay until this session is the one the Swift image documents. The change that removes them updates `AGENTS.md` in the same commit.

**QEMU.** None. The host test builds a scripted survey page and checks the formatter bytes. Keep `lerux shell ready` out of `expect` and `assert` messages.

**Metal evidence.** None. This milestone closes on the host test.

**Exit.** The host test shows `ip link` naming `wlan0`, `[iwd]# `, a `device list` row, a silent `scan`, a `get-networks` table, the passphrase prompt, the PMK rejection, and `Device NAME not found`.

## Milestone 2 — Connect, then an address

**Status:** not started.

**Arch capability.** `connect` returns when association finishes. DHCP starts because the link came up. The operator types no DHCP command. The next `ip link` shows the link up.

**Boards.** `swift_sfx14_41g`. Depends on Milestone 1. Depends on the radio reaching `wait` for a metal boot. The host half does not.

**Protection domains and templates.** The shell calls the existing `bring_up` in [`userspace/pds/sddf-shell/src/net.rs`](../userspace/pds/sddf-shell/src/net.rs). `swift-mt7921` does not gain a DHCP client. Association stays in `join.rs`.

**Steps.**

- `connect` returns to `[iwd]# ` when `associate` finishes. A failure, including a SAE-only beacon, prints `Operation failed`. This path stays WPA-Personal: PBKDF2-HMAC-SHA1 and an HMAC-MD5 MIC.
- On success the powered station is associated. The next `ip link` prints index 1, flags `BROADCAST,MULTICAST,UP,LOWER_UP`, `mtu 1500`, `state UP`, and `link/ether` with the station address and `brd ff:ff:ff:ff:ff:ff`. Before association the same line uses flags `NO-CARRIER,BROADCAST,MULTICAST,UP` and `state DOWN`. There is no `lo` line.
- The worker starts `bring_up` after association and polls that socket while it waits for the next line. `connect` does not wait for the lease. `ping` in Milestone 3 waits up to the budget `bring_up` already uses.
- The stage line on row 47 stays the radio’s report (`scan`, `auth tx`, `auth rx`, `assoc`, `eapol`, then `wait`). The shell does not paint over it from the command transcript.

**QEMU.** None for the radio. A host test of the command result covers `Operation failed` and the two `ip link` lines. The scripted-radio test that reaches `ip 192.168.1.20` and `ping ok` stays the Milestone 8 regression.

**Metal evidence.** None for this milestone alone. The address is the lease Milestone 3’s ping uses. [plan-metal-parity.md](plan-metal-parity.md) Milestone 8 still owns the panel’s address and `ping ok`.

**Exit.** `connect` returns to the prompt after association, a failed association prints `Operation failed`, and `ip link` changes from `NO-CARRIER` / `DOWN` to `LOWER_UP` / `UP`. DHCP is running without a command that names it.

## Milestone 3 — Ping the guide’s name

**Status:** not started.

**Arch capability.** `ping ping.archlinux.org` resolves the name through the DNS servers in the DHCP lease and prints a reply.

**Boards.** `swift_sfx14_41g`, and the e1000e fetch image for the QEMU proof. Depends on Milestone 2.

**Protection domains and templates.** `sddf-shell`. The fetch feature gains `smoltcp/socket-dns` and the ICMP socket `sddf-net-client` already uses. The Swift feature already pulls that fetch feature, so both images gain the sockets. This does not enable `browser-net` and does not link `lerux-web`.

**Steps.**

- `ping ping.archlinux.org` is a shell command, typed outside `[iwd]# `.
- The resolver uses the DNS servers from the lease. It does not use the QEMU user-net address except on the fetch image, where that address is what the lease contains.
- The command sends one ICMP echo and returns. The console has no SIGINT, so the command does not run until Ctrl-C the way iputils does on Arch.
- A reply prints two lines whose prefixes are `PING ping.archlinux.org (` and `64 bytes from `.
- A name that does not resolve prints `ping: ping.archlinux.org: Name or service not known`.
- A ping before the lease prints `ping: connect: Network is unreachable`.
- The command transcript does not contain `ping ok`. The stage report that [plan-lionsos.md](plan-lionsos.md) Milestone 27 requires, the address and then `ping ok`, stays a separate line on the panel. The stage row may still say `ping`.
- Existing fetch smokes do not type `ping`. They stay as they are. `just test-x86-e1000e` still prints `ping ok` from `sddf-net-client` and is not this command.

**QEMU.** The e1000e fetch image (`just test-x86-fetch-native`’s shell) resolves a name through user-net DNS and prints the reply prefix. The host test covers the unreachable line and the missing-name line.

**Metal evidence.** One reply line on the Swift panel, on the WPA-Personal network the operator joined in Milestone 2. That boot is also LionsOS milestone 27 once the panel has shown the address and `ping ok`.

**Exit.** `ping ping.archlinux.org` prints a reply line. The QEMU fetch image prints the same prefix. The command transcript has no `ping ok`.

## Milestone 4 — Boards that are not this radio

**Status:** not started.

**Arch capability.** Ethernet in the installation guide is the cable, the same DHCP, and the same ping. Those machines have no `iwctl` device. A different Wi-Fi chip is a different driver.

**Boards.** The e1000e fetch image, then `pc_z97_d3h` when the I217-V enqueues frames, then `rpi4b_4gb` when GENET enqueues frames. Depends on Milestone 3 for `ip link` and `ping`.

**Protection domains and templates.** The shell that already owns the network queues on that board. `swift-mt7921` is not added to the Z97 or the Pi. A new Wi-Fi chip is a new protection domain and a new id table, the way `mt7921e` and `iwlwifi` are different modules.

**Steps.**

- `ip link` on a board whose queues are bound to an Ethernet driver prints index 1 and the name `eth0`, with that device’s hardware address. Arch’s predictable names are a udev policy this image does not run. One Ethernet device keeps the one name `eth0`.
- `ping` is the Milestone 3 command.
- `iwctl` then `device list` on a board with no radio prints the device header and no row. `station wlan0 scan` prints `Device wlan0 not found`.
- The I217-V and GENET use `ip link` and `ping` when their drivers enqueue frames. They do not gain an `iwctl` row. Those metal lines wait for [plan-metal-parity.md](plan-metal-parity.md) Milestones 6 and 12. They do not close ahead of those milestones.
- USB and SDIO radios (`mt7921u`, `mt7921s`) are not this image. Chip ids outside `MT7921_IDS` stay refused.

**QEMU.** `ip link` and `ping` on the e1000e fetch image, shared with Milestone 3. `device list` with no radio is a host test.

**Metal evidence.** None beyond Milestone 3’s Swift reply. The I217 and GENET lines are recorded when those drivers already pass frames.

**Exit.** A board with no radio prints an empty `device list`. `eth0` is the `ip link` name for the one Ethernet device. A second Wi-Fi chip is specified as a new protection domain, and this milestone does not add one.

## Order

1. Milestone 1. It is host-only and does not wait for the firmware stall.
2. Milestone 2 after Milestone 1. The metal half waits until the radio reaches `wait`.
3. Milestone 3 after Milestone 2. The QEMU half can be written against the e1000e fetch image while the Swift radio is still stalled.
4. Milestone 4 after Milestone 3. The I217 and GENET lines wait for their own drivers.

## Explicit non-goals

- NetworkManager, `nmcli`, `wpa_supplicant`, and `iw`.
- An `rfkill` command. The installation guide’s step is a check that the card is not blocked. This image has no software block. Powered `on` in `device list` is that check once the stage line is `wait`.
- WPS, 802.1X, a hidden-network connect, and a second phy.
- Saved networks under `/var/lib/iwd`, and autoconnect on the next boot.
- A D-Bus daemon, an nl80211 socket, or a mac80211 protection domain.
- Retargeting the firmware download to the 7.2 `mt7921e` sources.
- Replacing LionsOS milestone 27’s `ping ok` line with the iputils transcript.
