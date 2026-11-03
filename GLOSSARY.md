# Glossary

Words for the machines in this repo, the devices on the [Swift X](#swift-x), and the mechanisms a [driver](#driver) uses to program them. A linked name jumps to the heading that defines it.

The shell, the browser, and the agent stay in [docs/context.md](docs/context.md). Which [protection domain](#protection-domain) programs which device is in [docs/platforms.md](docs/platforms.md#x86-device-map) and [docs/platforms.md](docs/platforms.md#swift-x-devices).

## Machines

### q35

The QEMU machine `q35`, a model of an Intel ICH9 PC. Every x86 guest in this repo boots on it.

_Avoid_: [Z97 desk](#z97-desk), [Swift X](#swift-x)

### Z97 desk

The Gigabyte GA-Z97-D3H on this desk. Haswell processor and an Intel 9-series chipset. The [Microkit](#microkit) board name is still `x86_64_generic`.

_Avoid_: [q35](#q35), [Swift X](#swift-x)

## Serial

### COM1

The first PC serial port. Its registers start at [I/O port](#io-port) `0x3f8`, and the [interrupt](#interrupt) is [I/O APIC](#io-apic) pin 4. On the [Z97 desk](#z97-desk) the connector is the COMA header. The [Swift X](#swift-x) has no COM port.

### NS16550

The universal asynchronous receiver-transmitter register layout that [COM1](#com1) uses.

## Display

### VGA text page

The 80 by 25 character buffer at physical address `0xb8000`. One cell is a character and a color. [q35](#q35) and the [Z97 desk](#z97-desk) [scanout](#scanout) use this page. The [Swift X](#swift-x) panel is a [GOP](#gop) buffer.

### CRTC

The VGA cathode-ray-tube controller. [I/O ports](#io-port) `0x3d4` and `0x3d5` select the cursor on the [VGA text page](#vga-text-page).

_Avoid_: [Intel display engine](#intel-display-engine)

### Standard VGA

QEMU's generic VGA device on [q35](#q35). [SeaBIOS](#seabios) leaves the [VGA text page](#vga-text-page) visible on it.

_Avoid_: [HD Graphics 4600](#hd-graphics-4600)

### SeaBIOS

The legacy BIOS [firmware](#firmware) QEMU runs on [q35](#q35) before the guest. It leaves [Standard VGA](#standard-vga) in text mode.

_Avoid_: [OVMF](#ovmf)

### HD Graphics 4600

The processor graphics on the [Z97 desk](#z97-desk). [PCI](#pci) `00:02.0`, identity `8086:0412`. With Initial Display Output set to `IGFX`, [firmware](#firmware) reports it as the boot VGA device, and the motherboard HDMI port shows the [VGA text page](#vga-text-page).

_Avoid_: [Swift Radeon](#swift-radeon), [RX 560](#rx-560)

### Intel display engine

The [scanout](#scanout) [hardware block](#hardware-block) inside the [HD Graphics 4600](#hd-graphics-4600). [Firmware](#firmware) programs it.

_Avoid_: [CRTC](#crtc), [Standard VGA](#standard-vga)

### RX 560

The Sapphire Radeon card in the [Z97 desk](#z97-desk). [PCI](#pci) `01:00.0`, identity `1002:67ff`. The HDMI cable is not on this card.

_Avoid_: [Swift Radeon](#swift-radeon), [HD Graphics 4600](#hd-graphics-4600)

### scanout

The path that reads a framebuffer and sends those pixels to a panel or a port. On the [Swift X](#swift-x), [firmware](#firmware) has already programmed the [Swift Radeon](#swift-radeon) before [seL4](#sel4) starts, and the console writes the [GOP](#gop) buffer that path is reading. On the [Z97 desk](#z97-desk), the [HD Graphics 4600](#hd-graphics-4600) scans the [VGA text page](#vga-text-page) when Initial Display Output is `IGFX`.

## Keyboard

### i8042

The PS/2 keyboard controller. The data [I/O port](#io-port) is `0x60` and the command [I/O port](#io-port) is `0x64`. The [Swift X](#swift-x) built-in keyboard and the [Z97 desk](#z97-desk) rear combo port both use this controller. On the [Swift X](#swift-x) the [interrupt](#interrupt) is [I/O APIC](#io-apic) pin 1, vector 49.

### xHCI

The register layout of a USB 3 host controller. The [Z97 desk](#z97-desk) controller is Intel, at [PCI](#pci) `00:14.0`. The [Swift X](#swift-x) controllers are the [AMD xHCI](#amd-xhci) functions. QEMU's device is [qemu-xhci](#qemu-xhci).

_Avoid_: [HCI](#hci)

### qemu-xhci

QEMU's own USB 3 host controller. The USB keyboard smoke attaches it.

_Avoid_: [xHCI](#xhci), as a name for the [Z97 desk](#z97-desk) part or an [AMD xHCI](#amd-xhci) function

### USB boot keyboard

A keyboard that speaks the human interface device boot protocol, so keys can be read without a general USB device stack. The [Z97 desk](#z97-desk) image accepts one on the Intel [xHCI](#xhci). The [Swift X](#swift-x) menu uses the [i8042](#i8042).

### AMD xHCI

The [Swift X](#swift-x) USB host controllers at [PCI](#pci) `04:00.3` (`1022:1639`) and `04:00.4` (`1022:1638`). The image maps their registers at `0xd1100000` and `0xd1200000`. `swift-bt` uses one of them to reach the Bluetooth [HCI](#hci).

_Avoid_: [qemu-xhci](#qemu-xhci), and the Intel [xHCI](#xhci) at `00:14.0`

## Clock

### CMOS real-time clock

The battery-backed clock in a PC chipset. It stores the wall time.

### time-stamp counter

A register in the processor that counts cycles.

## Disk and network

### 82574

The Intel Ethernet controller that QEMU's [e1000e](#e1000e) device emulates. [PCI](#pci) identity `8086:10d3`.

_Avoid_: [I217-V](#i217-v)

### I217-V

The Intel Ethernet controller on the [Z97 desk](#z97-desk) onboard LAN port. [PCI](#pci) `00:19.0`, identity `8086:153b`, subsystem `1458:e000`.

_Avoid_: [82574](#82574), [RTL8852BE](#rtl8852be)

### e1000e

The register model shared by the [82574](#82574) and the [I217-V](#i217-v). The QEMU flag `-device e1000e` instantiates an [82574](#82574). The [descriptor](#descriptor) [rings](#ring) and the [base address register](#base-address-register) layout match. The physical-layer blocks differ.

_Avoid_: using it as the name of the [Z97 desk](#z97-desk) part

### RTL8852BE

The Realtek wireless card on the [Z97 desk](#z97-desk), behind root port `00:1c.6`, identity `10ec:b852`. The probe reads [configuration space](#configuration-space) and does not map the card.

_Avoid_: [I217-V](#i217-v), [MT7921](#mt7921)

### ich9-ahci

QEMU's Serial ATA controller on [q35](#q35). [PCI](#pci) `8086:2922` at `00:1f.2`. It speaks the [Advanced Host Controller Interface](#advanced-host-controller-interface).

### Advanced Host Controller Interface

The register specification for a Serial ATA controller's command list and SATA ports. [ich9-ahci](#ich9-ahci) speaks it, and the [Z97 desk](#z97-desk) Serial ATA controller speaks it too. That desk controller's [PCI](#pci) identity is not recorded yet.

_Avoid_: [NVMe](#nvme), which is the [Swift X](#swift-x) disk

## Kernel and domains

### seL4

The microkernel this guest runs on. The upstream sources provide threads, address spaces, and [capabilities](#capability). [Drivers](#driver) and the shell are [protection domains](#protection-domain). The kernel source is not lerux code.

### Microkit

The static framework that builds `loader.img` from a [system description](#system-description) and the [protection domain](#protection-domain) binaries. The installed kit is 2.3.0.

### protection domain

One isolated userspace program, with its own address space and [capabilities](#capability). A [driver](#driver) is one of these programs. The crate name uses hyphens (`swift-mt7921`). The [Microkit](#microkit) name uses underscores (`swift_mt7921`).

### capability

The [seL4](#sel4) right to use one object. A [protection domain](#protection-domain) maps a device only when the [system description](#system-description) grants the memory region or the [interrupt](#interrupt). Port `0xcf8` is granted to `swift-pci-probe` alone. [Microkit](#microkit) rejects a second domain on that range.

### system description

The `.system` file [Microkit](#microkit) reads. It names each [protection domain](#protection-domain), the regions that domain maps, and the [channels](#channel) between them. The [Swift X](#swift-x) template is `console-swift.system.template`.

### driver

The [protection domain](#protection-domain) that programs one device. It maps [memory-mapped I/O](#memory-mapped-io), fills [rings](#ring) in a [DMA window](#dma-window), and writes a [doorbell](#doorbell). [seL4](#sel4) keeps that program in userspace.

### channel

The [Microkit](#microkit) path that wakes one [protection domain](#protection-domain) from another, or from an [interrupt](#interrupt). [Channel](#channel) numbers come from the [system description](#system-description). On `swift-mt7921`, [channel](#channel) 0 is the [MSI](#msi) and [channel](#channel) 1 is the key the shell writes.

### interrupt

A device signal delivered to a [protection domain](#protection-domain) on a [channel](#channel). The [Swift X](#swift-x) keyboard uses [I/O APIC](#io-apic) pin 1, vector 49, which is the number the processor enters. A probe line `irq ff` means [firmware](#firmware) assigned no [legacy interrupt](#legacy-interrupt), and the image uses [MSI](#msi) for the other devices.

## Buses and registers

### PCI

The configuration model the probe and the [drivers](#driver) use to name a function. A function has a [BDF](#bdf), a [configuration space](#configuration-space), and [base address registers](#base-address-register). The [Swift X](#swift-x) endpoints are PCIe. The same headers describe them.

### configuration space

The registers a [PCI](#pci) function exposes before its [BAR](#base-address-register) is mapped. `swift-pci-probe` reads 64 bytes per function through [I/O ports](#io-port) `0xcf8` and `0xcfc`, and stores the six headers at `0x01FB0000`.

### BDF

The bus, device, and function numbers that name one [PCI](#pci) function. The [Swift X](#swift-x) image probes `01:00.0`, `02:00.0`, `03:00.0`, `04:00.0`, `04:00.3`, and `04:00.4`. A line that says `absent` is a location this boot did not find.

### base address register

The field in [configuration space](#configuration-space) where [firmware](#firmware) writes the physical address of a function's registers. Also called a [BAR](#base-address-register). The [MT7921](#mt7921) memory [BAR](#base-address-register) on the recorded boot is `0xfc82000000`. The low dword `8200000c` still has the type bits set, so it is not the address the image maps.

_Avoid_: [DMA window](#dma-window), [I/O port](#io-port)

### bus master

The [PCI](#pci) command bit that lets a function read and write RAM on its own. The Swift probe sets this bit, and the memory-space bit, on the [NVMe](#nvme), the [AMD xHCI](#amd-xhci) functions, and the [MT7921](#mt7921). The [firmware](#firmware) [BAR](#base-address-register) values stay where they are.

_Avoid_: [driver own](#driver-own)

### memory-mapped I/O

Device registers the processor reads and writes as memory. Also called [MMIO](#memory-mapped-io). The [Swift X](#swift-x) image maps the [MT7921](#mt7921) [BAR](#base-address-register) at virtual `0x82000000`, the [NVMe](#nvme) registers at `0xd1500000`, and the [AMD xHCI](#amd-xhci) registers at `0xd1100000` and `0xd1200000`. The physical [MT7921](#mt7921) address is above 4 GiB, so it is not used as a virtual address.

_Avoid_: [DMA window](#dma-window)

### I/O port

An x86 address the processor reaches with `in` and `out`. [COM1](#com1) starts at `0x3f8`. The [i8042](#i8042) uses `0x60` and `0x64`. [PCI](#pci) [configuration space](#configuration-space) uses `0xcf8` and `0xcfc`.

### I/O APIC

The chip that routes a [legacy interrupt](#legacy-interrupt) pin to a CPU vector, the [interrupt](#interrupt) number the processor enters. The [Swift X](#swift-x) keyboard pin is 1 and the vector is 49. [COM1](#com1) on the [Z97 desk](#z97-desk) is pin 4.

### MSI

A message signaled [interrupt](#interrupt). The device writes a configured address. The [Swift X](#swift-x) image uses this for the [NVMe](#nvme), the [MT7921](#mt7921), and the [AMD xHCI](#amd-xhci). The recorded line `irq ff` means [firmware](#firmware) assigned no [legacy interrupt](#legacy-interrupt).

### legacy interrupt

The pin-based [interrupt](#interrupt) named by the [PCI](#pci) [interrupt](#interrupt) line register. `irq ff` means that register was left unassigned. The [Swift X](#swift-x) keyboard uses one, through the [I/O APIC](#io-apic). The [NVMe](#nvme), the [MT7921](#mt7921), and the [AMD xHCI](#amd-xhci) use [MSI](#msi).

## Moving bytes

### DMA

A device reading or writing RAM without the processor copying each byte. Also called [direct memory access](#dma). The [NVMe](#nvme) controller reads a [submission queue](#submission-queue) and writes a sector. The [MT7921](#mt7921) [DMA engine](#dma-engine) walks [descriptors](#descriptor) in the [DMA window](#dma-window). A [virtio](#virtio) device copies through its own region at `0x8_000_000_000` on the QEMU ARM images.

_Avoid_: [memory-mapped I/O](#memory-mapped-io)

### DMA engine

The [hardware block](#hardware-block) that walks a [descriptor](#descriptor) [ring](#ring) and moves the bytes. On the [MT7921](#mt7921) that block is the [WFDMA](#wfdma). The [NVMe](#nvme) controller does the same job inside the disk function. A [virtio](#virtio) device does the copy itself.

### DMA window

The RAM a device is allowed to use for [DMA](#dma). The [MT7921](#mt7921) window is physical `0x1f60000`, 256 KiB, eight [descriptors](#descriptor) per [ring](#ring). The [NVMe](#nvme) window is `0x01f00000` and ends at `0x01f20000`. The Bluetooth command [ring](#ring) sits at `0x1f20000`, beside that window.

_Avoid_: [memory-mapped I/O](#memory-mapped-io), [firmware hole](#firmware-hole)

### firmware hole

The range at `0xf8000000`. The Bluetooth command [ring](#ring) used to sit there and did not run. A [ring](#ring) the device must read sits in a low [DMA window](#dma-window) instead.

### descriptor

One slot in a [ring](#ring). It holds a buffer address, a length, and status. An [MT7921](#mt7921) slot is 16 bytes. An [NVMe](#nvme) [submission queue](#submission-queue) entry is 64 bytes, and the sector address in it is a [PRP](#prp). A completion entry is 16 bytes.

### ring

A circular array of [descriptors](#descriptor) in a [DMA window](#dma-window). The [driver](#driver) fills a slot, then writes the [doorbell](#doorbell) with the new index. The [MT7921](#mt7921) uses transmit and receive [rings](#ring) of eight slots. [NVMe](#nvme) uses a [submission queue](#submission-queue) and a [completion queue](#completion-queue). An [sDDF queue](#sddf-queue) is this kind of array between [protection domains](#protection-domain).

### doorbell

A register write that tells the device its [ring](#ring) index moved. The [descriptor](#descriptor) bytes are already in the [DMA window](#dma-window). The [NVMe](#nvme) [driver](#driver) writes the [submission queue](#submission-queue) tail or the [completion queue](#completion-queue) head at `0x1000 + queue id * stride`. The stride comes from the controller CAP register. The [xHCI](#xhci) [driver](#driver) writes the slot and the endpoint into the [doorbell](#doorbell) array, at the offset the [doorbell](#doorbell) field at register `0x14` names. The [MT7921](#mt7921) [driver](#driver) writes the CPU index at offset 8 of that [ring](#ring)'s [WFDMA](#wfdma) registers. A [virtio](#virtio) [driver](#driver) writes the queue notify register for the same reason.

_Avoid_: [interrupt](#interrupt). This write goes from the [driver](#driver) to the device. An [interrupt](#interrupt) goes from the device to the [driver](#driver).

### hardware block

A named piece of a chip with its own registers, inside one [PCI](#pci) function. The [MT7921](#mt7921) function contains the [WFSYS](#wfsys), the [WFDMA](#wfdma), and the [MCU](#mcu). The [Swift Radeon](#swift-radeon) contains the [scanout](#scanout) block, which this image leaves to [firmware](#firmware). The [Intel display engine](#intel-display-engine) is this kind of piece on the [Z97 desk](#z97-desk).

### PRP

The physical address an [NVMe](#nvme) command gives the controller for the sector buffer. The read path uses an address inside the [DMA window](#dma-window).

### sDDF queue

A shared-memory [ring](#ring) from the [seL4](#sel4) Device [Driver](#driver) Framework, copied into `lerux-sddf`. The [Swift X](#swift-x) disk path enqueues one read on the block queue, then issues [NVMe](#nvme) read opcode `0x02`. The [MT7921](#mt7921) path enqueues a frame the radio has accepted. The index the other side writes is an acquire load. The index this side owns is a release store. The signal between the two [protection domains](#protection-domain) is a [channel](#channel).

_Avoid_: [doorbell](#doorbell)

### virtio

The device interface QEMU provides for disk, network, and the graphics smoke. The [driver](#driver) puts [descriptors](#descriptor) in a [ring](#ring) in a [DMA](#dma) region and writes a notify register, which is that device's [doorbell](#doorbell). On the QEMU ARM images the register page is at `0x6_000_000_000` and the [DMA](#dma) region is at `0x8_000_000_000`. The [Swift X](#swift-x) has no such device and no QEMU model.

## Boot path

### firmware

A program this image did not compile, and that still has to run before a device will answer.

Platform [firmware](#firmware) is [UEFI](#uefi) on the [Swift X](#swift-x) and the [Z97 desk](#z97-desk). On QEMU it is [SeaBIOS](#seabios) or [OVMF](#ovmf). It assigns each [BAR](#base-address-register), chooses the boot display, and owns the machine until [ExitBootServices](#exitbootservices).

Device [firmware](#firmware) is a blob a [driver](#driver) copies onto a chip so an on-chip processor can run. The [MT7921](#mt7921) needs a patch image and a RAM image. `support/mt7921.bin` overrides only the RAM image. The patch has no file under `support/`. An empty pair paints `mt7921: firmware err`. Bluetooth uses `support/mt7921-bt.bin`, which is a different blob. An empty Bluetooth blob paints `bt: firmware err`.

_Avoid_: the [seL4](#sel4) kernel. The [mb2-stub](#mb2-stub) loads the kernel. Device [firmware](#firmware) is the blob linked into the [protection domain](#protection-domain).

### UEFI

The [firmware](#firmware) interface the [Swift X](#swift-x) and the [Z97 desk](#z97-desk) boot. The USB boot entry has to say [UEFI](#uefi). GRUB is `EFI/BOOT/BOOTX64.EFI`. There is no BIOS El Torito entry.

_Avoid_: [SeaBIOS](#seabios)

### GOP

The [UEFI](#uefi) [Graphics Output Protocol](#gop) buffer. [Firmware](#firmware) leaves it mapped, and [scanout](#scanout) is already reading it. On the [Swift X](#swift-x) the panel is this buffer. The 2026-10-29 boot showed `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`, which is `0x7e9000` bytes. [Pitch](#pitch) is 7680. `console-driver` paints 32-bit pixels there. The [mb2-stub](#mb2-stub) paints the mode line before [seL4](#sel4).

_Avoid_: [VGA text page](#vga-text-page)

### pitch

The number of bytes from one row of the [GOP](#gop) buffer to the next. The [Swift X](#swift-x) mode line recorded 7680 on a 1920 by 1080 frame, four bytes per pixel.

### ExitBootServices

The [UEFI](#uefi) call after which [firmware](#firmware) boot services are gone. `sel4.elf` is linked at 1 MiB, a range [firmware](#firmware) still owns before this call. The [mb2-stub](#mb2-stub) places the kernel there only after boot services have exited.

### OVMF

The [UEFI](#uefi) [firmware](#firmware) QEMU runs for an x86 ISO boot.

_Avoid_: the [firmware](#firmware) on the [Swift X](#swift-x), and [SeaBIOS](#seabios). A QEMU hello line does not prove the [Swift X](#swift-x) panel.

### mb2-stub

The Multiboot 2 trampoline linked at 32 MiB. GRUB loads it under [UEFI](#uefi). It asks for a framebuffer, paints the [GOP](#gop) mode line, writes the mode record at `0x01fe0000`, and then jumps to [seL4](#sel4). A [GOP](#gop) buffer above 4 GiB is painted with paging on, on a stack inside the identity map.

## Acer Swift X

### Swift X

The Acer [Swift X](#swift-x) SFX14-41G, Ryzen 5 5500U. The [Microkit](#microkit) board name is `swift_sfx14_41g`. There is no [COM1](#com1), no serial [driver](#driver), and no QEMU model. The keyboard is the built-in [i8042](#i8042). The panel is the [GOP](#gop) buffer of the [Swift Radeon](#swift-radeon). `just iso` packs this image.

_Avoid_: [Z97 desk](#z97-desk), [q35](#q35)

### MT7921

The MediaTek Wi-Fi function at [PCI](#pci) `03:00.0`, identity `14c3:7961` or `14c3:0608`. `swift-mt7921` owns it. The recorded memory [BAR](#base-address-register) is `0xfc82000000`, mapped at virtual `0x82000000` with 4 KiB pages. The [DMA window](#dma-window) is physical `0x1f60000`. The register names [WFDMA](#wfdma), [WFSYS](#wfsys), and [MCU](#mcu) are the Linux v6.8 mt76 names this [driver](#driver) follows. Association is WPA-Personal. The menu item is `3 wifi`.

_Avoid_: the Bluetooth [HCI](#hci). That is a USB device behind an [AMD xHCI](#amd-xhci), with a different [firmware](#firmware) blob.

### WFDMA

The [DMA engine](#dma-engine) inside the [MT7921](#mt7921). Its registers sit in the [BAR](#base-address-register). The Linux map names the block base `0xd4000`. The [driver](#driver) programs the [ring](#ring) base and the count, then writes the CPU index at offset 8, which is the [doorbell](#doorbell).

### WFSYS

The Wi-Fi subsystem [hardware block](#hardware-block) in the [MT7921](#mt7921). The [driver](#driver) resets it and waits for init-done before the [firmware](#firmware) load. A failure at this step, and a later MAC setup failure, both paint `mt7921: mac`.

### MCU

The processor inside the [MT7921](#mt7921) that runs the device [firmware](#firmware). The [driver](#driver) scatters the patch and the RAM image into it and waits until N9 is ready. A chunk that does not leave the download [ring](#ring) paints `mt7921: fw scatter`.

### driver own

The host-ownership handshake on the [MT7921](#mt7921). It covers the opening polls and the later grab after [DMA](#dma) is enabled. Failure paints `mt7921: own`.

_Avoid_: [bus master](#bus-master). [Bus master](#bus-master) lets the function use [DMA](#dma). This handshake is the chip's power ownership.

### NVMe

The disk command set, and on this machine the SK hynix BC711 at [PCI](#pci) `02:00.0`, class `0x010802`. `swift-nvme` reads `LERUX.TXT`. IO write `0x01` and admin format `0x80` are rejected. The volume is not formatted. A controller that returns sector 0 with no FAT or NTFS boot sector paints `nvme: raw`.

### submission queue

The [NVMe](#nvme) [ring](#ring) the [driver](#driver) fills with commands. Admin and IO each have one. [Doorbell](#doorbell) ids 0 and 2 carry the tails.

### completion queue

The [NVMe](#nvme) [ring](#ring) the controller fills with status. The [driver](#driver) polls the phase bit, then writes the head to [doorbell](#doorbell) ids 1 and 3.

### HCI

The Bluetooth host-controller interface. `swift-bt` looks for USB class `0xE0`, subclass `0x01`, protocol `0x01` behind [PCI](#pci) `04:00.3` or `04:00.4`. Reset is the bytes `03 0c 00`. `bt: reset ok` needs the command-complete event after the Bluetooth [firmware](#firmware) is present. A line `bt: port N STAGE` means no interface was addressed.

_Avoid_: [xHCI](#xhci), [AMD xHCI](#amd-xhci)

### Swift Radeon

The AMD display function at [PCI](#pci) `04:00.0`, identity `1002:164c`. The panel belongs to it. [Firmware](#firmware) programs the [scanout](#scanout). This image writes the [GOP](#gop) buffer and leaves that [hardware block](#hardware-block) alone.

_Avoid_: [RX 560](#rx-560), [GTX 1650](#gtx-1650), [HD Graphics 4600](#hd-graphics-4600)

### GTX 1650

The NVIDIA function at [PCI](#pci) `01:00.0`, identity `10de:1f9d`. The probe prints its header. No [driver](#driver) maps it.

_Avoid_: [Swift Radeon](#swift-radeon)
