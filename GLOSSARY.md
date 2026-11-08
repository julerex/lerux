# Glossary

Words for the machines in this repo, the devices on the [Swift X](#swift-x), and the mechanisms a [driver](#driver) uses to program them. A linked name jumps to the heading that defines it. The [MT7921](#mt7921) line, from the [patch semaphore](#patch-semaphore) through [ping](#ping), is under [Wi-Fi bring-up](#wi-fi-bring-up).

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

The configuration model the probe and the [drivers](#driver) use to name a function. A function has a [BDF](#bdf), a [configuration space](#configuration-space), and [base address registers](#base-address-register). The [Swift X](#swift-x) endpoints sit on [PCIe](#pcie). The same headers describe them.

### PCIe

The link the [Swift X](#swift-x) endpoints share. A [DMA](#dma) read that never finishes can stay on this link and stall another function. That is how `nvme: admin` can show up beside a stuck [MT7921](#mt7921) transmit. Sitting on the bus means sitting on this link.

_Avoid_: the bus number in a [BDF](#bdf), and [bus master](#bus-master). The bus number names where the function is attached. [Bus master](#bus-master) is the bit that allows [DMA](#dma). [Configuration space](#configuration-space) holds the [PCI](#pci) registers.

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

### IOMMU

The AMD unit that can translate a device [DMA](#dma) address before it reaches RAM. This [Swift X](#swift-x) reports its registers at `0xfdf00000`. The probe does not build device tables. It excludes `0x1f00000` through `0x1fa0000`, the [NVMe](#nvme), [xHCI](#xhci), and [MT7921](#mt7921) [DMA windows](#dma-window), so those addresses are not translated. The display path stays translated. Row 16 reads that programming back: `excl 01f00001 01f9f000 on` is translation on, base `0x01f00001`, limit `0x01f9f000`. `off` means translation is off. `excl miss` means the probe left the unit alone.

_Avoid_: the processor [memory page](#memory-page) tables. Those translate CPU loads and stores. This unit would translate device [DMA](#dma).

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

### memory page

The 4 KiB unit [Microkit](#microkit) uses to map device registers. The [MT7921](#mt7921) [BAR](#base-address-register) is mapped with 4 KiB [memory pages](#memory-page). A [WFDMA](#wfdma) [ring](#ring) base has to be the start of one. `0x200` sits inside a [memory page](#memory-page), and the [DMA engine](#dma-engine) can ignore a base that is not the start of one. Linux [mt76](#mt76) allocates those [rings](#ring) on that boundary.

_Avoid_: the [VGA text page](#vga-text-page), the [virtio](#virtio) register page, and the [GOP](#gop) framebuffer.

### cache line

The 64 bytes this processor keeps together. A device [DMA](#dma) reads RAM, not this cache. The [descriptor](#descriptor) has to be written out before the [doorbell](#doorbell). A store fence orders the CPU's own writes. It does not by itself push the [cache line](#cache-line) out. The [mb2-stub](#mb2-stub) runs `wbinvd` before [seL4](#sel4) when the [GOP](#gop) buffer is above 4 GiB.

_Avoid_: a [memory page](#memory-page). A [memory page](#memory-page) is 4 KiB. This [cache line](#cache-line) is 64 bytes.

### descriptor

One slot in a [ring](#ring). It holds a buffer address, a length, and status. An [MT7921](#mt7921) slot is 16 bytes, and its length fields are the [descriptor segments](#descriptor-segment). An [NVMe](#nvme) [submission queue](#submission-queue) entry is 64 bytes, and the sector address in it is a [PRP](#prp). A completion entry is 16 bytes.

### descriptor segment

One length field in the [MT7921](#mt7921) [descriptor](#descriptor) control word, in the Linux [mt76](#mt76) layout. The [first segment](#descriptor-segment) is bits 29–16 and is the buffer the [DMA engine](#dma-engine) transfers. The [second segment](#descriptor-segment) is bits 13–0. A length that sits only in the [second segment](#descriptor-segment) moves zero bytes, so the [ROM](#rom) never sees the command. A receive length taken from that same field drops the reply the same way. Bit 30 marks the end of the [first segment](#descriptor-segment). Bit 31 is the [write-back](#write-back) flag.

_Avoid_: the [NVMe](#nvme) command length. That lives in the 64-byte [submission queue](#submission-queue) entry, not in this control word.

### write-back

Bit 31 of the [MT7921](#mt7921) [descriptor](#descriptor) control word. The [DMA engine](#dma-engine) may set it when a transfer finishes. Linux [mt76](#mt76) also treats a transmit as done when the [DMA index](#dma-index) moves. Waiting only for this bit can time out while the read is still outstanding.

_Avoid_: the [DMA index](#dma-index). The [DMA index](#dma-index) moving is the other completion. This bit is the flag in the [descriptor](#descriptor).

### DMA index

The [WFDMA](#wfdma) [ring](#ring) register at offset 12. The [DMA engine](#dma-engine) advances it when it takes a [descriptor](#descriptor). The panel field is `d`. `d00` means the [DMA index](#dma-index) stayed 0, so the [DMA engine](#dma-engine) never fetched the slot.

_Avoid_: the [CPU index](#cpu-index), which is the [doorbell](#doorbell).

### CPU index

The [WFDMA](#wfdma) [ring](#ring) register at offset 8. Writing it is the [doorbell](#doorbell). The panel field is `c`. `c01` means the [doorbell](#doorbell) value is 1.

_Avoid_: the [DMA index](#dma-index).

### ring

A circular array of [descriptors](#descriptor) in a [DMA window](#dma-window). The [driver](#driver) fills a slot, then writes the [doorbell](#doorbell) with the new [CPU index](#cpu-index). The [DMA index](#dma-index) is how far the device has walked. The base register is the physical address of the first slot and has to start a [memory page](#memory-page). The [MT7921](#mt7921) uses transmit and receive [rings](#ring) of eight slots. [NVMe](#nvme) uses a [submission queue](#submission-queue) and a [completion queue](#completion-queue). An [sDDF queue](#sddf-queue) is this kind of array between [protection domains](#protection-domain).

### doorbell

A register write that tells the device its [ring](#ring) index moved. The [descriptor](#descriptor) bytes are already in the [DMA window](#dma-window), and a dirty [cache line](#cache-line) still has to be written out before this write. The [NVMe](#nvme) [driver](#driver) writes the [submission queue](#submission-queue) tail or the [completion queue](#completion-queue) head at `0x1000 + queue id * stride`. The stride comes from the controller CAP register. The [xHCI](#xhci) [driver](#driver) writes the slot and the endpoint into the [doorbell](#doorbell) array, at the offset the [doorbell](#doorbell) field at register `0x14` names. The [MT7921](#mt7921) [driver](#driver) writes the [CPU index](#cpu-index) at offset 8 of that [ring](#ring)'s [WFDMA](#wfdma) registers. A [virtio](#virtio) [driver](#driver) writes the queue notify register for the same reason.

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

Device [firmware](#firmware) is a blob a [driver](#driver) copies onto a chip so an on-chip processor can run. The [MT7921](#mt7921) needs a [patch image](#patch-image) and a [RAM image](#ram-image). An empty pair paints `mt7921: firmware err`. Bluetooth uses `support/mt7921-bt.bin`, which is a different blob. An empty Bluetooth blob paints `bt: firmware err`.

_Avoid_: the [seL4](#sel4) kernel. The [mb2-stub](#mb2-stub) loads the kernel. Device [firmware](#firmware) is the blob linked into the [protection domain](#protection-domain).

### UEFI

The [firmware](#firmware) interface the [Swift X](#swift-x) and the [Z97 desk](#z97-desk) boot. The USB boot entry has to say [UEFI](#uefi). [GRUB](#grub) is `EFI/BOOT/BOOTX64.EFI`. There is no BIOS [El Torito](#el-torito) entry.

### ISO

The image `just iso` writes for the [Swift X](#swift-x). The file is `build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso`. The recipe's own status line includes `Wrote ISO`. [GRUB](#grub) is the loader on that image. The boot entry has to say [UEFI](#uefi).

_Avoid_: [OVMF](#ovmf). That is the [firmware](#firmware) QEMU uses for an x86 [ISO](#iso). This file is the [Swift X](#swift-x) image.

### metal boot

A boot of the [ISO](#iso) on the [Swift X](#swift-x) itself. The panel is the only output. There is no [COM1](#com1). The host test that prints an address and `ping ok` drives a scripted radio and does not stand in for this boot.

### GRUB

The [UEFI](#uefi) loader at `EFI/BOOT/BOOTX64.EFI` on the [ISO](#iso). It loads the [mb2-stub](#mb2-stub). There is no BIOS [El Torito](#el-torito) entry.

### El Torito

The BIOS boot record used for optical discs. The [Swift X](#swift-x) [ISO](#iso) does not use one. The USB entry has to say [UEFI](#uefi).

_Avoid_: [SeaBIOS](#seabios)

### GOP

The [UEFI](#uefi) [Graphics Output Protocol](#gop) buffer. [Firmware](#firmware) leaves it mapped, and [scanout](#scanout) is already reading it. On the [Swift X](#swift-x) the panel is this buffer. The 2026-10-29 boot showed `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`, which is `0x7e9000` bytes. [Pitch](#pitch) is 7680. `console-driver` paints 32-bit pixels there. The [mb2-stub](#mb2-stub) paints the mode line before [seL4](#sel4).

_Avoid_: [VGA text page](#vga-text-page)

### pitch

The number of bytes from one row of the [GOP](#gop) buffer to the next. The [Swift X](#swift-x) mode line recorded 7680 on a 1920 by 1080 frame, four bytes per pixel.

### cell buffer

The character grid the console [driver](#driver) paints from. On the [Swift X](#swift-x) the grid is 25 rows, and the pixels land in the [GOP](#gop) buffer. `swift_mt7921` writes row 22, `swift_nvme` writes row 23, and `swift_bt` writes row 24. A scroll or a full clear is `Damage::All`, so `present_all` copies this grid back over those rows. Read the [firmware stage](#firmware-stage) before that happens.

_Avoid_: the [GOP](#gop) mode line the [mb2-stub](#mb2-stub) paints. That line is the framebuffer record. This grid is the console.

### ExitBootServices

The [UEFI](#uefi) call after which [firmware](#firmware) boot services are gone. `sel4.elf` is linked at 1 MiB, a range [firmware](#firmware) still owns before this call. The [mb2-stub](#mb2-stub) places the kernel there only after boot services have exited.

### OVMF

The [UEFI](#uefi) [firmware](#firmware) QEMU runs for an x86 [ISO](#iso) boot.

_Avoid_: the [firmware](#firmware) on the [Swift X](#swift-x), and [SeaBIOS](#seabios). A QEMU hello line does not prove the [Swift X](#swift-x) panel.

### mb2-stub

The Multiboot 2 trampoline linked at 32 MiB. [GRUB](#grub) loads it under [UEFI](#uefi). It asks for a framebuffer, paints the [GOP](#gop) mode line, writes the mode record at `0x01fe0000`, and then jumps to [seL4](#sel4). A [GOP](#gop) buffer above 4 GiB is painted with paging on, on a stack inside the identity map. `wbinvd` runs before [seL4](#sel4), so those [cache lines](#cache-line) reach the panel.

## Acer Swift X

### Swift X

The Acer [Swift X](#swift-x) SFX14-41G, Ryzen 5 5500U. The [Microkit](#microkit) board name is `swift_sfx14_41g`. There is no [COM1](#com1), no serial [driver](#driver), and no QEMU model. The keyboard is the built-in [i8042](#i8042). The panel is the [GOP](#gop) buffer of the [Swift Radeon](#swift-radeon). `just iso` packs this image.

_Avoid_: [Z97 desk](#z97-desk), [q35](#q35)

### MT7921

The MediaTek Wi-Fi function at [PCI](#pci) `03:00.0`, identity `14c3:7961` or `14c3:0608`. `swift-mt7921` owns it. The recorded memory [BAR](#base-address-register) is `0xfc82000000`, mapped at virtual `0x82000000` with 4 KiB [memory pages](#memory-page). The [DMA window](#dma-window) is physical `0x1f60000`. The register names [WFDMA](#wfdma), [WFSYS](#wfsys), and [MCU](#mcu) are the Linux v6.8 [mt76](#mt76) names this [driver](#driver) follows. [Association](#association) is [WPA-Personal](#wpa-personal). The menu item is `3 wifi`. The bring-up words are the [firmware stage](#firmware-stage).

_Avoid_: the Bluetooth [HCI](#hci). That is a USB device behind an [AMD xHCI](#amd-xhci), with a different [firmware](#firmware) blob.

### WFDMA

The [DMA engine](#dma-engine) inside the [MT7921](#mt7921). Its registers sit in the [BAR](#base-address-register). The Linux [mt76](#mt76) map names the block base `0xd4000`. Each [ring](#ring) has a base, a size, the [CPU index](#cpu-index) at offset 8, and the [DMA index](#dma-index) at offset 12. The base has to start a [memory page](#memory-page). It is written again after the [ring](#ring) pointers are reset and after [driver own](#driver-own), because those steps can drop it. Bit 0 of the global config's low byte enables transmit. The panel field `g` is that byte. The [write-back](#write-back) bit is not the only way a transmit finishes.

### WFSYS

The Wi-Fi subsystem [hardware block](#hardware-block) in the [MT7921](#mt7921). The [driver](#driver) resets it and waits for init-done before the [firmware](#firmware) load. A failure at this step, and a later [MAC setup](#mac) failure, both paint `mt7921: mac`.

### MCU

The processor inside the [MT7921](#mt7921) that runs the device [firmware](#firmware). Commands it accepts before the [RAM image](#ram-image) is running are answered by the on-chip [ROM](#rom). The [driver](#driver) sends those commands with a [hardware transmit descriptor](#hardware-transmit-descriptor) and an [MCU header](#mcu-header), then [scatters](#scatter) the [patch image](#patch-image) and the [RAM image](#ram-image). It waits until [N9](#n9) is ready.

### driver own

The [host-own](#driver-own) handshake on the [MT7921](#mt7921). It covers the opening polls, before the [patch image](#patch-image) load, and the later grab after [DMA](#dma) is enabled. Failure paints `mt7921: own`. A line that says `own` still includes those polls.

_Avoid_: [bus master](#bus-master). [Bus master](#bus-master) lets the function use [DMA](#dma). This handshake is the chip's power ownership.

### NVMe

The disk command set, and on this machine the SK hynix BC711 at [PCI](#pci) `02:00.0`, class `0x010802`. `swift-nvme` reads `LERUX.TXT`. IO write `0x01` and admin format `0x80` are rejected. The volume is not formatted. A controller that returns sector 0 with no FAT or NTFS boot sector paints `nvme: raw`. A read that never returns paints `nvme: admin`. See [admin command](#admin-command).

### admin command

A command on the [NVMe](#nvme) admin [submission queue](#submission-queue). Creating the IO queues is one. The controller has come up, and then one [admin command](#admin-command) never returns, which paints `nvme: admin`. An [MT7921](#mt7921) [DMA](#dma) read that never finishes can sit on the [PCIe](#pcie) link and stall it.

_Avoid_: admin format, opcode `0x80`, which this [driver](#driver) rejects on purpose. IO read is opcode `0x02` and uses the IO queue, not this one.

### submission queue

The [NVMe](#nvme) [ring](#ring) the [driver](#driver) fills with commands. The [admin command](#admin-command) queue and the IO queue each have one. [Doorbell](#doorbell) ids 0 and 2 carry the tails.

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

## Wi-Fi bring-up

### mt76

The Linux v6.8 MediaTek [driver](#driver) family this [MT7921](#mt7921) sequence follows. The files are `mt7921/pci.c`, `mt792x_dma.c`, and `mt76_connac_mcu.c`. A register those files do not map is not invented here.

_Avoid_: the [RTL8852BE](#rtl8852be) [driver](#driver). That card is the [Z97 desk](#z97-desk).

### ROM

The on-chip program in the [MT7921](#mt7921) that answers [MCU](#mcu) commands until the [RAM image](#ram-image) is running. The [patch semaphore](#patch-semaphore) is the first command it is asked.

### hardware transmit descriptor

The 32-byte prefix on an [MCU](#mcu) command, eight dwords, placed in front of the [MCU header](#mcu-header). The [DMA engine](#dma-engine) moves the whole buffer. The length the [ROM](#rom) trusts is counted after this prefix.

_Avoid_: the [descriptor](#descriptor) in the [ring](#ring). `fw sem tx` means that [ring](#ring) slot never finished. This prefix is the 32 bytes at the start of the command that slot points at.

### MCU header

The bytes after the [hardware transmit descriptor](#hardware-transmit-descriptor) and before the command payload. The [hardware transmit descriptor](#hardware-transmit-descriptor) and the [MCU header](#mcu-header) together are 64 bytes, and the payload starts on the next byte. The length the [ROM](#rom) trusts counts every byte after the [hardware transmit descriptor](#hardware-transmit-descriptor), the [MCU header](#mcu-header) included. The [patch semaphore](#patch-semaphore) payload is 4 bytes, so that length is 36. The command id sits at byte 36 of the buffer. A length that counts only the payload stops before the command id, and the [ROM](#rom) does not answer.

### patch image

The first [firmware](#firmware) half, the one the [ROM](#rom) takes before the [RAM image](#ram-image). The build decompresses `WIFI_MT7961_patch_mcu_1_2_hdr.bin.zst`, and the `1a` file beside it. There is no file under `support/` for this half. A file header that does not parse, a region setup that is rejected, or a finish command with no reply paints `mt7921: fw patch`.

_Avoid_: the [RAM image](#ram-image). `support/mt7921.bin` overrides the RAM half only.

### RAM image

The second [firmware](#firmware) half, the program the [MCU](#mcu) runs after the [patch image](#patch-image). The build decompresses `WIFI_RAM_CODE_MT7961_1.bin.zst`, and the `1a` file beside it. `support/mt7921.bin` overrides only the primary file. A trailer or a region setup that is rejected paints `mt7921: fw ram`. The start command after this half, with no reply, paints `mt7921: fw start`. An empty [patch image](#patch-image) pair or an empty [RAM image](#ram-image) pair paints `mt7921: firmware err`.

_Avoid_: `support/mt7921-bt.bin`. That blob is the [HCI](#hci).

### chip id

The id the [MT7921](#mt7921) reports before the [patch image](#patch-image) is loaded. `0x7922` and `0x7925` need a different pair from the one this build links. That mismatch paints `mt7921: fw chip`.

_Avoid_: the [PCI](#pci) identity `14c3:7961`. That names the function. This id is the one the [firmware](#firmware) path checks.

### patch semaphore

The first command the [ROM](#rom) is asked once the [firmware](#firmware) is ready to load. The command id is `0x10`. The payload is 4 bytes. The [MCU header](#mcu-header) plus that payload is a length of 36.

The status byte is the byte the [ROM](#rom) returns. `01` means the [patch image](#patch-image) is already loaded. `02` means it is not, and the download continues. `03` releases the [semaphore](#patch-semaphore) after the finish command. A held [semaphore](#patch-semaphore) is released when the transfer fails too.

The line can name which stop it was. `tx` means the transmit [descriptor](#descriptor) never finished. `rx` means the [descriptor](#descriptor) finished and no event matched it. Two hex digits are the status byte, and `02` is the value that continues into the [patch image](#patch-image). A readback is `d`, `c`, `s`, `b`, and `g`. `d` is the [DMA index](#dma-index). `c` is the [CPU index](#cpu-index). `s` is the [ring](#ring) size. `b` is the low 16 bits of the [descriptor](#descriptor) base. `g` is the low byte of the [DMA](#dma) config, and bit 0 means transmit is enabled. `fw sem d00 c01 s08 b0200 g75` means the [DMA index](#dma-index) stayed 0, the [doorbell](#doorbell) is 1, the size is 8, the base is `0x0200` and not a [memory page](#memory-page), and transmit is enabled. Row 17 reads the posted [descriptor](#descriptor) back: `slot 00 01f66000 d0` is index 0, buffer `0x01f66000`, and a clear done bit. `d1` means that bit is set.

_Avoid_: a [channel](#channel) signal. This [semaphore](#patch-semaphore) is a command to the [ROM](#rom).

### scatter

One chunk of the [patch image](#patch-image) or the [RAM image](#ram-image) on the download [ring](#ring). A chunk that does not leave that [ring](#ring) paints `mt7921: fw scatter`.

### N9

The ready flag the [MCU](#mcu) raises after the [RAM image](#ram-image) has started. [MAC setup](#mac) waits until it is set. The normal [firmware stage](#firmware-stage) after that is `wait`.

### MAC

The [MT7921](#mt7921) radio setup after [N9](#n9). It covers the [WFSYS](#wfsys) reset and init-done, `dma_disable`, and the later `mac_init`. Failure paints `mt7921: mac`. A line that says `mac` or `own` still includes the earlier returns.

_Avoid_: the 6-byte station address. The [driver](#driver) uses `02:14:c3:79:21:01`. This word is the setup step.

### firmware stage

The word `swift_mt7921` paints on row 22 of the [cell buffer](#cell-buffer). The line starts with `mt7921:`.

`own` is [driver own](#driver-own). `fw sem` is the [patch semaphore](#patch-semaphore). `fw patch` is the [patch image](#patch-image). `fw scatter` is one [scatter](#scatter). `fw ram` and `fw start` are the [RAM image](#ram-image). `fw chip` is the [chip id](#chip-id). `mac` is [MAC setup](#mac). `wait` is the normal state after [N9](#n9) and that setup. `scan` is a [Wi-Fi scan](#wi-fi-scan). `auth` is [authentication](#authentication). `assoc` is [association](#association). `eapol` is [EAPOL](#eapol). `dhcp` is [DHCP](#dhcp). `ping` is [ping](#ping). `bar` is a [BAR](#base-address-register) that is not the address the [driver](#driver) maps. `firmware err` means the [patch image](#patch-image) or the [RAM image](#ram-image) was empty.

A line that says `mac` or `own` still includes the earlier returns. Type `3` only when the line says `wait`. The shell then stores the [SSID](#ssid) and the [PSK](#psk) and notifies the radio on [channel](#channel) 1. A notify while the line is still one of these words repaints that word and does not keep the key. The `bt:` line on row 24 is the [HCI](#hci) behind an [AMD xHCI](#amd-xhci). It does not join the [access point](#access-point).

### Wi-Fi scan

The [MT7921](#mt7921) looks for the [SSID](#ssid) the shell stored. The paint word is `scan`. This runs only after the [firmware stage](#firmware-stage) says `wait`.

_Avoid_: [scanout](#scanout). That path reads pixels. This command looks for an [access point](#access-point).

### authentication

The step the radio takes after a [Wi-Fi scan](#wi-fi-scan), before [association](#association). The paint word is `auth`.

### association

The step that joins an [access point](#access-point) after [authentication](#authentication). The paint word is `assoc`. [WPA-Personal](#wpa-personal) is the kind this [driver](#driver) performs.

### access point

The network named by the [SSID](#ssid). [Association](#association) joins it. The [HCI](#hci) line does not.

### SSID

The network name, 1 to 32 bytes. Menu item `3` asks for it. The shell paints `ssid ok` when it stores the name, then asks for the [PSK](#psk).

_Avoid_: the [access point](#access-point) itself. This is the name.

### PSK

The [passphrase](#psk), 8 to 63 bytes. The shell paints `psk ok` when it stores the bytes and notifies the radio. A notify before the [firmware stage](#firmware-stage) is `wait` does not keep the key for the next boot.

_Avoid_: the [SSID](#ssid). The name and the [passphrase](#psk) are separate prompts.

### WPA-Personal

The [association](#association) this [driver](#driver) performs: the [SSID](#ssid) plus the [PSK](#psk). Also called [WPA2-PSK](#wpa-personal). The frames on a [metal boot](#metal-boot) are wireless. The host test's scripted frames are not this.

_Avoid_: the Bluetooth [HCI](#hci).

### EAPOL

The four-way handshake after [association](#association), before [DHCP](#dhcp). The paint word is `eapol`.

### DHCP

The address request after [EAPOL](#eapol). The paint word is `dhcp`. The bring-up wants the address before [ping](#ping).

### ping

One [ICMP](#ping) echo after [DHCP](#dhcp). An address and then `ping ok` is the end of this bring-up. The host test that prints `ip 192.168.1.20 ping ok` drives a scripted radio. A [metal boot](#metal-boot) is the one that counts.

_Avoid_: treating the host test as that boot.
