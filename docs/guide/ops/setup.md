# Set up the Lerux live image

This is the first boot of Lerux on the Acer Swift X SFX14-41G. You build a USB image on a Linux computer, write that image to a USB stick, boot the Swift from the stick, and join a Wi-Fi network with the built-in MediaTek MT7921.

The image is a live image. It runs from the USB stick. It leaves the internal NVMe as it is. The NVMe driver is read-only, and this guide does not format that disk.

The radio is the one MT7921 at PCI `03:00.0`. The shell command that joins it is `setup-wifi`.

The board name is `swift_sfx14_41g`. The reference for that board is [boards.md](../../hardware/boards/boards.md#acer-swift-x-install-path). The words on a scan line are defined in [context.md](../overview/context.md#swift-radio-survey).

## What you need

- A Linux computer to build the image, with `git`, `just`, `rustup`, `cmake`, `ninja`, `libclang-dev`, and `clang`.
- Packing tools: `xorriso` and `grub-mkimage`. On Arch Linux the packages are `libisoburn` and `grub` (`sudo pacman -S libisoburn grub`). On Debian the packages are `xorriso` and `grub-efi-amd64-bin`.
- A network connection on that computer while the build downloads seL4, the Microkit SDK, and the Rust crates.
- A USB flash drive you can erase. The build computer's own disk is refused by the write command.
- The Acer Swift X, its built-in keyboard, and a Wi-Fi network whose passphrase is 8 to 63 bytes.

`rustup` reads `rust-toolchain.toml` in this repo and installs the nightly that file names on the first build.

## Build the ISO and write the stick

The commands below are run on the Linux computer, in a checkout of this repo. `just make-lerux-iso-swift` packs the Swift image. `just burn-lerux-iso-swift` packs that same image and then writes it to a USB stick.

### Get the source

```bash
git clone https://github.com/julerex/lerux.git
cd lerux
just fetch
just fetch-sdk
```

`just fetch` clones seL4 and Microkit into `deps/workspace/`. `just fetch-sdk` downloads Microkit SDK 2.3.0.

### Pack the image and write the stick

Plug in the USB stick. From the repo directory:

```bash
just burn-lerux-iso-swift
```

That command first packs `build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso`. When the pack finishes, the log contains a line of this shape:

```text
==> Wrote ISO (N bytes) → build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso
```

`N` is the size of the file just written. The pack step does not write the stick. The script that follows does.

If the stick is not plugged in yet, the script prints:

```text
burn-lerux-iso: plug in the USB stick
```

Leave the command running. It looks again once a second.

When it sees a USB disk, it prints a numbered list and waits:

```text
burn-lerux-iso: USB sticks
  1  /dev/sdb  29.0G  Your stick model
burn-lerux-iso: type a number to erase that stick, or q to stop
```

The size and the model come from the stick. A size of a gigabyte or more is printed with one decimal (`29.0G`). A smaller stick is printed in whole megabytes. Type the number of the stick you mean, then Enter. Typing the device name from that list, such as `/dev/sdb`, selects it as well. The script prints:

```text
burn-lerux-iso: selected /dev/sdb
```

`q` and Enter stops with `burn-lerux-iso: stopped` and writes nothing. A number that is not on the list prints `burn-lerux-iso: type a number from the list` and shows the list again.

The script then asks the Linux computer for permission to write the stick. You may see:

```text
burn-lerux-iso: asking for root to write /dev/sdb
```

`sudo` then asks for the password of your Linux account on the build computer. That password is for the build computer. The Lerux login on the Swift has none.

After that, the script shows the stick and asks you to type its device name:

```text
burn-lerux-iso: this erases /dev/sdb:
```

An `lsblk` table for that device follows, then:

```text
burn-lerux-iso: type /dev/sdb to continue
```

Type the device name exactly, including `/dev/`, then Enter. Any other text stops with `burn-lerux-iso: not confirmed` and writes nothing.

The write looks like this:

```text
burn-lerux-iso: writing /path/to/lerux/build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso
```

`dd` prints its own progress. When the stick has been written and flushed:

```text
burn-lerux-iso: wrote /path/to/lerux/build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso to /dev/sdb
```

That last line means the stick is ready. Wait until the command has returned to the Linux shell before you unplug the stick. If this computer is the Swift (or any Linux host that already runs Omarchy or another UEFI OS you want to keep as the default), leave the stick plugged in and run:

```bash
just boot-lerux-usb
```

That command lists the USB sticks, sets UEFI `BootNext` to the Lerux entry for one reboot only, asks you to type `reboot`, and reboots. It does not change the lasting `BootOrder`, so the next power-on returns to Omarchy (or whatever already boots). Pass the device name to skip the list, or `--no-reboot` to set `BootNext` and reboot yourself:

```bash
just boot-lerux-usb /dev/sdc
just boot-lerux-usb --no-reboot
```

The recipe takes no arguments. If you already know the device name, the script itself takes it and skips the numbered list. It writes the Swift ISO that `just make-lerux-iso-swift` already packed, with the same confirmation:

```bash
bash support/boot/scripts/burn-lerux-iso.sh /dev/sdb
```

Replace `/dev/sdb` with the disk from `lsblk`. Pass the whole disk, such as `/dev/sdb`. A partition such as `/dev/sdb1` is refused with `burn-lerux-iso: pass the whole stick`.

### If the write command stops

| What it prints | What to do |
| --- | --- |
| `burn-lerux-iso: refusing /dev/sda because it holds the host OS (/)` | That disk is the computer you are building on. Pick the USB stick. |
| `burn-lerux-iso: refusing /dev/sda (transport is nvme)` | That disk is not a USB stick. The word in parentheses is the transport `lsblk` reported. |
| `burn-lerux-iso: stick is too small: the image is N bytes and /dev/sdb is M bytes` | Use a larger stick. |
| `burn-lerux-iso: image not found: … (run the matching just make-lerux-iso-* recipe)` | The pack step did not leave the ISO. Run `just make-lerux-iso-swift` and read the error above that line. |
| `burn-lerux-iso: no terminal to ask which stick to erase: run the recipe in a terminal` | The command has no terminal to ask you. Run it again in a terminal. |

`just make-lerux-iso-swift` alone packs the ISO and stops. It does not write a stick. `just burn-lerux-iso-swift` always packs the Swift image, including after a previous `just make-lerux-iso-z97` or `just make-lerux-iso-inspiron`. Those other recipes are different machines.

## Boot the Swift

Move the stick to the Acer. If you built on the Acer itself, leave the stick plugged in and reboot.

The ISO is UEFI only. There is no legacy BIOS entry on this image.

1. Reboot and open the firmware boot menu. This tree does not record which firmware key the Swift uses for that menu. The key is the one your firmware shows for a one-time boot menu.
2. Choose the USB entry that says **UEFI**. GRUB on the stick is `EFI/BOOT/BOOTX64.EFI`.
3. GRUB has one entry, `lerux`, and its timeout is 0, so that entry starts at once. GRUB loads `/boot/mb2-stub.elf` with modules `/boot/sel4.elf` and `/boot/loader.img`.

The first thing the panel paints is a blue bar and the firmware mode line, in white glyphs on that bar. On 2026-10-29 this machine showed:

```text
fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0
```

The shell then paints a band of 240 columns by 40 rows, light gray on black, in the VGA 8×16 font. The cursor is a two-pixel underline. The built-in keyboard types into that band. This laptop has no serial port.

The band starts with the issue line, a blank line, the login line, a blank line, the message of the day, and the prompt:

```text
Lerux <git-id> (tty1)

lerux login: root (automatic login)

```

`<git-id>` is the short `git describe --always --dirty` of the tree you packed, with one leading `v` removed. A tree that had uncommitted changes when you packed it puts `-dirty` on that id. There is no password. The login line is the whole login.

The message of the day is the Arch install text. It tells you to follow the Arch installation guide, and it names `iwctl` for Wi-Fi. Those words are painted on the screen. This shell does not run `iwctl`, `mmcli`, or `Installation_guide`. The Wi-Fi command on this image is `setup-wifi`.

The prompt is:

```text
[root@lerux ~]# 
```

The cursor sits after the space. Type a command and press Enter.

Under the 40-row band, other programs paint device lines in the same font. The Wi-Fi line is the one that starts with `mt7921:`. A line that starts with `bt:` is Bluetooth. It is a separate status line.

### If the prompt never appears

| What you see | What it means |
| --- | --- |
| `WARNING: no console will be available to OS` | GRUB had no framebuffer to hand to the stub. Rebuild with `just burn-lerux-iso-swift` from a current checkout and rewrite the stick. |
| The mode line, and no sign-on | The panel address on that line is not the buffer this image maps, or the shell did not start. A current Swift image maps `0xfc90000000`. Read the hex on the mode line, then rebuild and rewrite the stick. |
| The sign-on and `[root@lerux ~]# ` | The shell is ready. Continue at the prompt. |

## Connect to Wi-Fi

Wait until the Wi-Fi line says:

```text
mt7921: wait
```

That word means the MT7921 firmware has finished starting. The sign-on is painted after that startup returns, so the line is usually already `mt7921: wait` when the prompt appears.

While the card is still starting, the same line can show `mt7921 .` with a moving dot. Nearby lines can show `cpu: write cmd`, `ram: write desc`, `wfdma: doorbell`, and `wfdma: fetch`. Those are startup marks. The result is the word after `mt7921:` once the dot is gone.

`setup-wifi` before `mt7921: wait` prints `wifi err` and leaves the Wi-Fi line on the word it already shows.

### The prompts

The shell has three places you type. Each one ends when you press Enter.

| Prompt on screen | What you type |
| --- | --- |
| `[root@lerux ~]# ` | `setup-wifi` |
| `ssid> ` | The row number from the list, or `quit` |
| `psk> ` | The passphrase, or `quit` |

`setup-wifi` is accepted in either ASCII case, so `SETUP-WIFI` is the same command. The network name and the passphrase are the bytes you type. Caps Lock changes those bytes. The command itself still matches.

The keyboard echoes every character, including the passphrase. Backspace is the Delete byte `0x7f`. The shell rubs out one character.

A line at the `[root@lerux ~]# ` prompt is at most 80 bytes. A longer line is dropped, and the shell prints `err empty`.

### Run setup-wifi

At the prompt, type:

```text
setup-wifi
```

Press Enter. Type that word alone. `setup-wifi` plus another word prints:

```text
setup-wifi err args
[root@lerux ~]# 
```

The shell asks the radio which names it can hear. The radio finishes that scan before the next line appears. Keys you type during the scan are discarded. Wait for the list.

A scan that stored names prints one line per name, at most 16, in byte order, then the name prompt. The number is the row, starting at 1:

```text
[root@lerux ~]# setup-wifi
1 ExampleNet
2 OtherNet
ssid> 
```

`ExampleNet` and `OtherNet` stand in for the names your radio heard. The shape of each line is the row, a space, and the name.

The Wi-Fi line under the band then shows that scan as `mt7921:` plus the survey words from the table below. A recorded boot that listed networks showed `mt7921: rx 1 ev 16 ok 12 done 1 ch 21 bcn 15`.

### Choose the network

At `ssid> `, type the row number and press Enter. `1` selects the first printed name. The row is chosen first: if a heard name is the digits `2`, typing `2` still selects row 2.

You can also type the name itself, exactly as printed, including spaces in the middle of the name. A line that is neither a printed row nor a printed name is rejected.

These answers are rejected, and the shell prints `wifi err` and returns to the prompt:

- A row number that was not printed, such as `9` when the list has two names.
- A leading zero, such as `01`. The shell looks that up as a name. Row 1 is the single digit `1`.
- An empty line.
- A name that is not on the list.

```text
ssid> 9
wifi err
[root@lerux ~]# 
```

`quit` and Enter leaves the command with no error line:

```text
ssid> quit
[root@lerux ~]# 
```

### Type the passphrase

After a name is accepted, the shell prints:

```text
psk> 
```

Type the passphrase and press Enter. It must be 8 to 63 bytes. A letter or a digit is one byte. Spaces at the start or the end are removed. Spaces in the middle are kept and count toward the length.

A passphrase the shell accepts is written, and the radio associates before the next shell line appears. Wait for that line. While it associates, the Wi-Fi line walks `mt7921: scan`, `mt7921: auth tx`, `mt7921: auth rx`, `mt7921: assoc`, and `mt7921: eapol`. A finished handshake paints `mt7921: wait` and then the traffic line in [A finished connection](#a-finished-connection), still before `psk ok` is printed. A step that fails stays on that step's word. The passphrase is kept only in memory for this boot. The next boot asks again.

`quit` and Enter at `psk> ` returns to the prompt and does not start the radio:

```text
psk> quit
[root@lerux ~]# 
```

A passphrase shorter than 8 bytes, or longer than 63 bytes, does not start the radio. A line longer than 80 bytes is dropped and takes the same path:

```text
psk> short
wifi err
[root@lerux ~]# 
```

### A finished connection

A handshake that finishes prints `psk ok`. The shell then asks the network for an address and sends one ping to the router named in that lease. The prompt does not return until that finishes.

Success has this shape. `192.168.1.50` is an example lease. Your line uses the address the router offered:

```text
psk ok
ip 192.168.1.50
ping ok
[root@lerux ~]# 
```

`ping ok` means that router answered. Once the link is up, the Wi-Fi line leaves `mt7921: wait` for a traffic line of this shape:

```text
mt7921: rx N tx N drop N
```

`N` is a count. `rx` is a frame queued for the shell, `tx` is a shell frame the card accepted, and `drop` is a received frame that was not queued. When `drop` is not 0, the line adds `fc` and two hex digits, or `fc --` when no frame byte was copied.

The address line is also what the `ip` command prints later. At the prompt, type `ip` and press Enter. With a lease, the shell prints `ip` and the same four numbers. With no lease, it prints `ip err`.

### When the scan finds no name

An empty scan prints one survey line and returns to the prompt. There is no `ssid> ` on that path. A finished scan that copied no frame has this shape. `N` is a count from that boot:

```text
[root@lerux ~]# setup-wifi
rx 0 ev 0 ok 0 done 1 ch N bcn N
[root@lerux ~]# 
```

`rx 0` and `ev 0` are the counts for "no frame copied", which is why `fc` is absent. A finished scan that copied frames and kept no name inserts `fc` after `done`:

```text
rx N ev N ok 0 done 1 fc HH ch N bcn N
```

`HH` is two hex digits, or `--` when every copied frame was rejected. When `done` is `0`, the line stops after that digit. The words stay in this order:

| Word | What it counts |
| --- | --- |
| `rx` | Frames that arrived on the data ring. |
| `ev` | Frames that arrived on an MCU ring. Most scan frames arrive there. |
| `ok` | Names the radio kept. |
| `done` | `1` when the firmware finished the scan, `0` when it did not. |
| `fc` | Printed only when a frame arrived and `ok` is 0. Two hex digits are the first byte of the first copied frame. `--` means every copied frame was rejected. |
| `ch` | How many of the 21 survey channels the firmware finished. The list is 13 channels on 2.4 GHz and 8 on 5 GHz. Printed when `done` is 1. |
| `bcn` | Beacons the firmware counted, including beacons the host never received. Printed when `done` is 1. |

A scan that kept names prints the numbered list in the shell. The survey lines above are the empty-list case. When the card is already at `mt7921: wait` and the radio does not run the survey, the shell prints `wifi err` and the Wi-Fi line stays `mt7921: wait`. The page the shell cannot mark prints:

```text
setup-wifi err page
```

### When the join fails

`wifi err` in the shell band is one line. The Wi-Fi line under the band names the step that stopped. The shell prints `wifi err` for each of these, and it does not print `psk ok`:

| Wi-Fi line | What stopped |
| --- | --- |
| `mt7921: scan` | After the passphrase, the named network was not found. |
| `mt7921: auth tx` | The open-system authentication frame did not leave the transmit ring. |
| `mt7921: auth rx` | That frame was sent and no matching reply arrived. |
| `mt7921: assoc` | Association did not finish. |
| `mt7921: eapol` | The four-way handshake returned without a key. |

A failed `auth rx` or `eapol` also paints four detail rows, then:

```text
END OF OUTPUT
```

The device lines under that marker are blanked. Those rows stay until the next boot. The shell-band prompt still returns after `wifi err`.

On `auth rx` the status line grows into this shape. `HH` is two hex digits or `--`, and `N` is a count:

```text
mt7921: auth rx fc HH rx N ev N skip N
```

`skip` counts MCU completions that were not frames.

On `eapol` the two letters after `eapol` name the handshake return:

| Letters | What they mean |
| --- | --- |
| `w1` | No message 1 arrived. |
| `m1` | A frame arrived and it was not message 1. |
| `b2` | Message 2 did not fit. |
| `t2` | Message 2 did not leave the ring. |
| `w3` | No message 3 arrived. |
| `m3` | A frame arrived and it was not message 3. |
| `b4` | Message 4 did not fit. |
| `t4` | Message 4 did not leave the ring. |

The line then continues with `fc`, the `rx` / `ev` / `skip` counts, and `tr`. `tr` counts frames whose receive descriptor has header translation. With `w1` and no copied frame, the shape is:

```text
mt7921: eapol w1 fc -- rx N ev N skip N tr N
```

The four detail rows under that line are, in order: `sq`, `st`, `alg`, `ad`, `hdr`, `pn`, `ms`, and `len`; then the two addresses (`a1` and `a2`, or `da` and `sa` when the frame is translated); then `a3` and `sta`; then 16 bytes of the frame body (`m0`, `m24`, or `m26`). `sta` is this station. A field printed as `-` or `--` was not present in the copied frame.

### When the handshake finishes and the network does not

`psk ok` means the handshake finished. The lines after it are the lease and the ping.

No lease:

```text
psk ok
ip err
[root@lerux ~]# 
```

A lease whose router did not answer the ping. `192.168.1.50` is an example. Your line uses the leased address:

```text
psk ok
ip 192.168.1.50
ping err
[root@lerux ~]# 
```

The shell waits for the lease, then for one answer, and then prints that result. It does not ask for another passphrase.

### If the card never reaches wait

The Wi-Fi line names the startup step that stopped. `setup-wifi` then prints `wifi err`.

| Wi-Fi line | What it means |
| --- | --- |
| `mt7921: absent` | The device at `03:00.0` is not the MT7921 this image drives (`14c3:7961` or `14c3:0608`). |
| `mt7921: firmware err` | The firmware images linked into this build were empty. |
| `mt7921: bar ` and 16 hex digits, or `mt7921: bar io` | The card's register window is not the address this image maps. |
| `mt7921: own` | The card did not give the host the radio. |
| `mt7921: fw sem …` | The patch semaphore did not finish. The rest of the line is a register reading. [swift-boot-screen.md](../../hardware/swift/swift-boot-screen.md) reads that line field by field. |
| `mt7921: fw patch` | The patch header or a patch command was rejected. |
| `mt7921: fw scatter` | A firmware chunk did not leave the download ring. |
| `mt7921: fw ram` | The RAM image header or a RAM command was rejected. |
| `mt7921: fw start` | The start command after the RAM image got no reply. |
| `mt7921: fw chip` | The chip id is not the MT7961 pair baked into this image. |
| `mt7921: mac` | Radio setup after the firmware download did not finish. |

## What you have when ping ok is on screen

The shell has a leased IPv4 address, and the router from that lease answered one ping. Run `setup-wifi` again on a later boot. The passphrase from this boot is gone when the machine powers off.

The message of the day is text. This image does not install Arch, and it does not format the internal disk.
