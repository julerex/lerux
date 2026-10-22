# x86 machines

Names for the QEMU q35 guest and the Gigabyte Z97 desk. The same word is often used for a part on the desk and for a different part QEMU emulates. Other lerux language stays in [docs/context.md](docs/context.md). Which protection domain programs which device is in [docs/platforms.md](docs/platforms.md#x86-device-map).

## Machines

**q35**:
The QEMU machine `q35`, a model of an Intel ICH9 PC. Every x86 guest in this repo boots on it.
_Avoid_: Z97 desk

**Z97 desk**:
The Gigabyte GA-Z97-D3H on this desk. Haswell processor and an Intel 9-series chipset. The Microkit board name is still `x86_64_generic`.
_Avoid_: q35

## Serial

**COM1**:
The first PC serial port. Its registers start at I/O `0x3f8`, and the interrupt is pin 4. On the Z97 desk the connector is the COMA header.

**NS16550**:
The universal asynchronous receiver-transmitter register layout that COM1 uses.

## Display

**VGA text page**:
The 80 by 25 character buffer at physical address `0xb8000`. One cell is a character and a color.

**CRTC**:
The VGA cathode-ray-tube controller. Ports `0x3d4` and `0x3d5` select the cursor on the VGA text page.
_Avoid_: Intel display engine

**Standard VGA**:
QEMU's generic VGA device on q35. SeaBIOS leaves the VGA text page visible on it.
_Avoid_: HD Graphics 4600

**SeaBIOS**:
The legacy BIOS QEMU runs on q35 before the guest. It leaves the standard VGA in text mode.

**HD Graphics 4600**:
The processor graphics on the Z97 desk. PCI `00:02.0`, identity `8086:0412`. With Initial Display Output set to `IGFX`, firmware reports it as the boot VGA device, and the motherboard HDMI port shows the VGA text page.

**Intel display engine**:
The scanout hardware inside the HD Graphics 4600. Firmware programs it.
_Avoid_: CRTC, standard VGA

**RX 560**:
The Sapphire Radeon card in the Z97 desk. PCI `01:00.0`, identity `1002:67ff`. The HDMI cable is not on this card.

## Keyboard

**i8042**:
The PS/2 keyboard controller. Data port `0x60`, command port `0x64`.

**xHCI**:
The extensible host controller interface, which is the register layout of a USB 3 host controller. The Z97 desk's controller is Intel, at PCI `00:14.0`.
_Avoid_: qemu-xhci

**qemu-xhci**:
QEMU's own extensible host controller interface device. The USB keyboard smoke attaches it. It is not the Intel controller at `00:14.0`.
_Avoid_: xHCI, as a name for the desk part

**USB boot keyboard**:
A keyboard that speaks the human interface device boot protocol, so keys can be read without a general USB device stack.

## Clock

**CMOS real-time clock**:
The battery-backed clock in a PC chipset. It stores the wall time.

**time-stamp counter**:
A register in the processor that counts cycles.

## Disk and network

**82574**:
The Intel Ethernet controller that QEMU's `e1000e` device emulates. PCI identity `8086:10d3`.
_Avoid_: I217-V

**I217-V**:
The Intel Ethernet controller on the Z97 onboard LAN port. PCI `00:19.0`, identity `8086:153b`, subsystem `1458:e000`.
_Avoid_: 82574, RTL8852BE

**e1000e**:
The register model shared by the 82574 and the I217-V. The QEMU flag `-device e1000e` instantiates an 82574. The descriptor rings and the base-address layout match. The physical-layer blocks differ.
_Avoid_: using it as the name of the desk part

**RTL8852BE**:
The Realtek wireless card on the Z97 desk, behind root port `00:1c.6`, identity `10ec:b852`.
_Avoid_: I217-V, onboard Ethernet

**ich9-ahci**:
QEMU's Serial ATA controller on q35. PCI `8086:2922` at `00:1f.2`. It speaks the Advanced Host Controller Interface.

**Advanced Host Controller Interface**:
The register specification for a Serial ATA controller's command list and ports. `ich9-ahci` speaks it, and the Z97 Serial ATA controller speaks it too. That desk controller's PCI identity is not recorded yet.
