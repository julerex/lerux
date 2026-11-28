# Gigabyte Z97-D3H hardware

Inventory of this Gigabyte Z97-D3H, the machine the `pc_z97_d3h_i217` image boots. Taken on 2026-11-27 from the installed Linux host (Omarchy, kernel `7.2.15-4-omarchy`), read-only. Driver source citations are the local checkout at `/home/julian/repos/linux` (`Makefile` version 7.4.0-rc3). The running modules are the 7.2.15 build; the PCI IDs below are the same in both.

Firmware windows are assignments from this boot. The I217-V windows, the xHCI window, and the HD Graphics 4600 aperture match the windows already recorded in [platforms.md](../boards/platforms.md#z97-desk-devices). A later firmware setup can move a window.

This pass did not write a disk, program a register, transmit on the serial port, wake the suspended graphics card, or change radio or boot settings. `dmidecode` is not installed and the DMI table file is root-only, so slot silkscreen names and the Super I/O chip ID were not read. PCI configuration was readable for 64 bytes, which covers the header and not the capability list. Link speed and width came from sysfs.

## Identity

| | |
| --- | --- |
| Vendor | Gigabyte Technology Co., Ltd. |
| Product | Z97-D3H |
| Board | Z97-D3H-CF, version string `x.x` |
| Chassis | type 3 (desktop) |
| BIOS | American Megatrends Inc. F9, 09/18/2015, release 4.6 |
| SMBIOS | 2.7. `DMI: Gigabyte Technology Co., Ltd. Z97-D3H/Z97-D3H-CF, BIOS F9 09/18/2015` |
| EFI | 2.3.1 by American Megatrends. Secure Boot disabled |
| ACPI | RSDP `ALASKA` at `0xc7dbd000`. Sleep states S0, S3, S4, S5. `/sys/power/mem_sleep` offers `s2idle` and `deep`, with `deep` selected |
| Kernel command line | encrypted btrfs root on `sda`, zswap off, quiet splash |

SMBIOS leaves the PCB revision as `x.x`, so this boot does not say whether the board is rev 1.0, 1.1, or 1.2. Gigabyte's F9 for the GA-Z97-D3H is dated 18 September 2015 on the rev 1.2 support page, and the specification below is the one those revisions share. Manual: [GA-Z97-D3H / GA-H97-D3H, rev 1101](https://download1.gigabyte.com/Files/Manual/mb_manual_ga-z97%28h97%29-d3h_v1.1_e.pdf). Specification page: [GA-Z97-D3H rev 1.2](https://www.gigabyte.com/Motherboard/GA-Z97-D3H-rev-12/sp).

The manual's board block is an LGA1150 CPU, Intel Z97, dual-channel DDR3, an ITE I/O controller on the LPC bus, Intel I217-V, Realtek ALC1150, a CPU PCI Express 3.0 x16 slot, a chipset PCI Express 2.0 x4 slot that shares its lanes with two PCI Express 2.0 x1 slots, three conventional PCI slots behind a PCI Express-to-PCI bridge, six SATA 6 Gb/s connectors plus SATA Express and an M.2 socket that share SATA ports 4 and 5, six USB 3.0 ports and eight USB 2.0 ports, DualBIOS (two 64 Mbit flashes), and a PS/2 combo port. This boot confirms the parts that enumerate. It does not see an M.2 or SATA Express device, a card in the PCI slots, or a PS/2 controller in the ACPI namespace.

## Processor

Intel Core i5-4690K. Family 6, model 0x3c (60), stepping 3. Four cores, one thread per core. APIC IDs 0, 2, 4, 6. Vendor `GenuineIntel`. Intel's specification is 22 nm, 4 cores / 4 threads, 3.50 GHz base, 3.90 GHz turbo, 6 MB cache, 88 W, dual-channel DDR3-1333/1600, 16 PCI Express 3.0 lanes, HD Graphics 4600 (device id `0x0412`, 20 execution units, 350–1200 MHz). [Intel ARK](https://www.intel.com/content/www/us/en/products/sku/80811/intel-core-i54690k-processor-6m-cache-up-to-3-90-ghz/specifications.html).

| | measured |
| --- | --- |
| Time-stamp counter | 3499.995 MHz after the refined calibration. Early PIT calibration was 3499.900 MHz |
| cpufreq | `intel_cpufreq`, intel_pstate in `passive`, governor `schedutil`. Minimum 800 MHz, maximum 3900 MHz, `no_turbo=0`, 32 P-states |
| Caches | L1 data 32 KiB 8-way and L1 instruction 32 KiB 8-way per core, 64-byte line. L2 256 KiB 8-way unified per core. L3 6 MiB 12-way shared by cpus 0–3 |
| Address size | 39-bit physical, 48-bit virtual |
| Microcode | early update from `0x1c` to `0x28` |
| Virtualization | VT-x. The `vmx` flag is set. x2APIC is off because the BIOS sets the x2APIC opt-out bit in the DMA remapping table |
| Thermal | `coretemp`. During this pass the package was about 42 °C. High is 80 °C, critical is 100 °C. Each core reported 38–41 °C |

The `ht` flag is present in `/proc/cpuinfo` and `lscpu` still reports one thread per core. That matches the i5-4690K, which has no Hyper-Threading.

Mitigations active on this kernel: PTI (Meltdown), MDS buffer clearing, L1TF PTE inversion with conditional VMX cache flushes, retpolines plus conditional IBPB and IBRS_FW (Spectre v2), speculative store bypass disabled via prctl, SRBDS microcode, and IBPB before exit to userspace (`vmscape`). The kernel reports the part unaffected by Gather Data Sampling, MMIO stale data, Retbleed, and TSX Async Abort.

## Firmware, clocks, and the memory map

| Block | Address | What this boot showed |
| --- | --- | --- |
| I/O APIC | `0xfec00000`, 4 KiB | id 8, version 32, GSI 0–23 |
| HPET | `0xfed00000` | ACPI id `0x8086a701`. `hpet0`: 8 comparators, 64-bit, 14.318180 MHz. IRQs 2 and 8 |
| Local APIC | `0xfee00000` | standard page |
| DMA remapping | `0xfed90000` and `0xfed91000` | both version 1:0. The second unit (flags `0x1`) includes the I/O APIC and the HPET. Host address width 39. IRQ remapping is on, in xAPIC mode |
| Reserved memory regions | `0xc9e8b000`–`0xc9e99fff` and `0xcb000000`–`0xcf1fffff` | the two RMRR ranges |
| PCI configuration | ports `0xcf8`–`0xcff` | "Using configuration type 1". The MCFG window is `0xf8000000`–`0xfbffffff` for buses 00–3f. The kernel first refused it ("not reserved") and then accepted it as an ACPI motherboard resource |
| ACPI PM timer | I/O `0x1808` | `acpi_pm` clocksource registered |
| ACPI interrupt links | LNKA and LNKD and LNKE and LNKH on IRQ 11. LNKB, LNKC, and LNKG on IRQ 10. LNKF disabled | |
| IRQ overrides | bus IRQ 0 → GSI 2. Bus IRQ 9 → GSI 9, high level | IRQ 4 is not overridden, so COM1 stays on GSI 4 |
| CMOS RTC | `PNP0B00` under the LPC bridge | `rtc_cmos`. A read at local 11:25 returned `2026-11-27 09:25:40`, which is UTC for that moment |
| PCI cache line | 64 bytes | |

Root-bridge windows from ACPI: I/O `0x0000`–`0x0cf7` and `0x0d00`–`0xffff`, memory `0xa0000`–`0xbffff`, `0xd0000`–`0xdffff`, and `0xcf200000`–`0xfeafffff`, buses 00–3e.

ACPI tables present: XSDT, FACP, DSDT (`INTL 20120711`, OEM revision `0x88`), FACS, APIC, FPDT, three static SSDTs (`Ther_R`, `Cpu0Ist`, `CpuSsdt`), MCFG, HPET, `SataTabl`, `SaSsdt`, BGRT, DMAR (`INTEL BDW`). Dynamic loads: `Cpu0Cst`, `ApIst`, `ApCst`. Nine GPEs in block `00`–`3F`. Thermal zones `TZ00` (about 28 °C) and `TZ01` (about 30 °C). Buttons `PWRB`, `SLPB`, and `PWRF`.

The iTCO watchdog binds and then stops: `unable to reset NO_REBOOT flag, device disabled by hardware/BIOS`. `lpc_ich` reports a resource conflict on `gpio_ich`, so the GPIO driver does not attach. No `it87` or Nuvoton module is loaded. The manual names the I/O controller only as an ITE chip. This boot's evidence for that chip is the 16550A UART and the ACPI thermal zones, not a chip ID.

```text
System RAM       0x000000000-0x000057fff
device reserved  0x000058000-0x000058fff
System RAM       0x000059000-0x00009efff
device reserved  0x00009f000-0x00009ffff
gap              0x0000a0000-0x0000fffff
System RAM       0x000100000-0x0a923cfff
ACPI NVS         0x0a923d000-0x0a9243fff
System RAM       0x0a9244000-0x0a9d35fff
device reserved  0x0a9d36000-0x0aa241fff
System RAM       0x0aa242000-0x0c7be7fff
device reserved  0x0c7be8000-0x0c7c51fff
ACPI data        0x0c7c52000-0x0c7c61fff
ACPI NVS         0x0c7c62000-0x0c7df7fff
device reserved  0x0c7df8000-0x0c9f69fff
System RAM       0x0c9f6a000-0x0c9f6bfff
device reserved  0x0c9f6c000-0x0c9ffefff
System RAM       0x0c9fff000-0x0c9ffffff
gap              0x0ca000000-0x0caffffff
device reserved  0x0cb000000-0x0cf1fffff
gap              0x0cf200000-0x0f7ffffff
device reserved  0x0f8000000-0x0fbffffff
gap              0x0fc000000-0x0febfffff
device reserved  0x0fec00000-0x0fec00fff
gap              0x0fec01000-0x0fecfffff
device reserved  0x0fed00000-0x0fed03fff
gap              0x0fed04000-0x0fed1bfff
device reserved  0x0fed1c000-0x0fed1ffff
gap              0x0fed20000-0x0fedfffff
device reserved  0x0fee00000-0x0fee00fff
gap              0x0fee01000-0x0feffffff
device reserved  0x0ff000000-0x0ffffffff
System RAM       0x100000000-0x42fdfffff
```

`Memory: 15917640K/16634324K available`. SMBIOS says 2 of 4 memory slots are populated. The VGA text page and the option ROM sit in the gap at `0xa0000`. The HD Graphics 4600 aperture at `0xd0000000` sits in the gap at `0xcf200000`.

## PCI devices

Subsystem vendor `0x1458` is Gigabyte. Command values are from the configuration header.

| Device | ID, rev, subsystem | Driver, Linux IRQ | Windows |
| --- | --- | --- | --- |
| DRAM controller | `8086:0c00` rev `06`, `1458:5000` | `hsw_uncore`, IRQ 0 | none published |
| CPU PCI Express x16 | `8086:0c01` rev `06`, `1458:5000` | `pcieport`, IRQ 24 | buses 00/01/01. I/O `0xe000`–`0xefff`. Memory `0xf7d00000`–`0xf7dfffff`. Prefetch `0xe0000000`–`0xf01fffff`. Link 8.0 GT/s x8, slot max 8.0 GT/s x16 |
| HD Graphics 4600 | `8086:0412` rev `06`, `1458:d000` | `i915`, MSI IRQ 34 | memory `0xf7800000` size 4 MiB. Prefetch aperture `0xd0000000` size 256 MiB. I/O `0xf000` size 64. Legacy VGA `0xc0000` size 128 KiB. Pin INTA, line 11. Boot VGA |
| Haswell HDMI audio | `8086:0c0c` rev `06`, `8086:2010` | `snd_hda_intel`, MSI IRQ 38 | memory `0xf7e34000` size 16 KiB. Bound to `00:02.0` |
| xHCI | `8086:8cb1` rev `00`, `1458:5007` | `xhci_hcd`, MSI IRQ 29 | memory `0xf7e20000` size 64 KiB. PME from D3hot and D3cold |
| MEI | `8086:8cba` rev `00`, `1458:1c3a` | `mei_me`, MSI IRQ 36 | memory `0xf7e3d000` size 16. `/dev/mei0` |
| I217-V | `8086:153b` rev `00`, `1458:e000` | `e1000e`, MSI IRQ 37 | memory `0xf7e00000` size 128 KiB, `0xf7e3c000` size 4 KiB. I/O `0xf080` size 32. Pin INTA, line 11 |
| EHCI #2 | `8086:8cad` rev `00`, `1458:5006` | `ehci-pci`, IOAPIC IRQ 16 | memory `0xf7e3b000` size 1 KiB. Debug port 2 |
| PCH HD Audio | `8086:8ca0` rev `00`, `1458:a182` | `snd_hda_intel`, MSI IRQ 39 | memory `0xf7e30000` size 16 KiB. Pin INTA, line 10 |
| Root port 1 | `8086:8c90` rev `d0`, `1458:5001` | `pcieport`, IRQ 25 | bus 02, no endpoint. Link width 0. Max 5.0 GT/s x2 |
| Root port 4 | `8086:8c96` rev `d0`, `1458:5001` | `pcieport`, IRQ 26 | buses 03–04. Link 2.5 GT/s x1, max 5.0 GT/s x1 |
| Root port 7 | `8086:8c9c` rev `d0`, `1458:5001` | `pcieport`, IRQ 27 | bus 05. I/O `0xd000`–`0xdfff`. Memory `0xf7c00000`–`0xf7cfffff`. Link 2.5 GT/s x1, max 5.0 GT/s x1 |
| EHCI #1 | `8086:8ca6` rev `00`, `1458:5006` | `ehci-pci`, IOAPIC IRQ 23 | memory `0xf7e3a000` size 1 KiB. Debug port 2 |
| Z97 LPC | `8086:8cc4` rev `00`, `1458:5001` | `lpc_ich`, IRQ 0 | COM1 and the CMOS RTC hang off this function |
| AHCI | `8086:8c82` rev `00`, `1458:b005` | `ahci`, MSI IRQ 28 | I/O `0xf0d0`, `0xf0c0`, `0xf0b0`, `0xf0a0`, `0xf060`. ABAR `0xf7e39000` size 2 KiB. Pin INTB, line 11. Class `0x010601` |
| SMBus | `8086:8ca2` rev `00`, `1458:5001` | `i801_smbus`, IOAPIC IRQ 18 | memory `0xf7e38000` size 256. I/O `0xf040` size 32. Pin INTC, line 10 |
| Radeon | `1002:67ff` rev `cf`, `1da2:e348` | `amdgpu`, MSI IRQ 35 | prefetch `0xe0000000` size 256 MiB and `0xf0000000` size 2 MiB. I/O `0xe000` size 256. MMIO `0xf7d00000` size 256 KiB. ROM `0xf7d40000` size 128 KiB. Link 8.0 GT/s x8, max 8.0 GT/s x8 |
| Radeon HDMI audio | `1002:aae0` rev `00`, `1da2:aae0` | `snd_hda_intel`, MSI IRQ 41 | memory `0xf7d60000` size 16 KiB. Power state follows `01:00.0` |
| PCI bridge | `8086:244e` rev `41`, `1458:8892` | none | at `03:00.0`. Conventional PCI, subtractive decode, bus 04 empty. `extended config space not accessible` |
| RTL8852BE | `10ec:b852` rev `00`, `1a3b:5470` | `rtw89_8852be`, MSI IRQ 40 | I/O `0xd000` size 256. Memory `0xf7c00000` size 1 MiB. Link 2.5 GT/s x1, and that is also the card's maximum |

`01:00.0` is behind the CPU root port, so it is in the PCIEX16 slot. The card's maximum width is x8, and the link trained at generation 3 x8. The empty root port 1 and the x1 link to the PCI bridge are the chipset slots. The manual gives the x4 slot and the two x1 slots a shared lane pool, and this boot does not identify which silkscreen is which empty port. Root port 7 is the wireless card. The bridge at `03:00.0` is the manual's PCI Express-to-PCI bridge (other Z97-D3H probes show the same `8086:244e` / `1458:8892`). Bus 04 has no card, so the three PCI slots are empty. The `thunderbolt` module registered its bus and found no endpoint. The manual does not list a Thunderbolt port.

## Memory modules

SMBus controller `i801_smbus` on PCI interrupt, with SPD Write Disable set. Two 256-byte SPD EEPROMs answer, at `0x51` and `0x53`. Nothing answers at `0x50` or `0x52`. Both are DDR3 (SPD byte 2 = `0x0b`), SPD revision `0x11`, UDIMM (byte 3 = `0x02`), dual rank, x8 devices, 64-bit bus, 8 banks, 8 GiB. Medium timebase 0.125 ns, so a minimum cycle time of 1.250 ns is 1600 MT/s. The reads below are the on-module EEPROM, not a BIOS setting.

| SMBus | Part | Maker ID | Date / serial | CAS bitmap (CL = bit + 4) | tAA, tRCD, tRP, tRAS | tRC, tRFC, tFAW |
| --- | --- | --- | --- | --- | --- | --- |
| `0x51` | `9905471-073.A00LF` | continuation `0x01`, id `0x98` (Kingston) | year `0x19`, week `0x48`, serial `0d3ba449` | CL6–CL11 (`0x00fc`) | 13.125, 13.125, 13.125, 35.000 ns | 48.125, 260, 30 ns |
| `0x53` | `CMZ8GX3M1A1600C10` | continuation `0x02`, id `0x9e` (Corsair) | date and serial bytes are zero | CL6, CL7, CL9, CL10 (`0x006c`) | 12.500, 12.500, 12.500, 33.750 ns | 48.125, 260, 30 ns |

Voltage byte 6 is `0x02` on the Kingston module and `0x00` on the Corsair module. Corsair's sheet for `CMZ8GX3M1A1600C10` is one 8 GB DDR3-1600 DIMM, 1.5 V, tested CL10, with an XMP 1.3 profile ([Corsair](https://www.corsair.com/us/en/p/memory/cmz8gx3m1a1600c10/vengeancea-a-8gb-ddr3-memory-kit-cmz8gx3m1a1600c10)). Bytes 176–195 of that EEPROM start `0c4a` and are otherwise the profile block. The same bytes on the Kingston module are zero. At 1600 MT/s the Kingston tAA of 13.125 ns is legal at CL11 (13.75 ns) and tight for CL10 (12.5 ns). The Kingston part is the 8 GB DDR3-1600 UDIMM recorded by [linux-hardware.org](https://linux-hardware.org/?id=mem%3Akingston-9905471-073-a00lf-dimm).

Both ranks of each module give 16 GiB installed. The kernel's e820 total is 16634324 KiB.

## Storage

AHCI `8086:8c82` is the 9 Series match in `drivers/ata/ahci.c` (`board_ahci_pcs_quirk`). The host reports version `0x00010300` (AHCI 1.3.0), 32 command slots, 6 Gb/s, SATA mode, and capability flags `64bit ncq led clo pio slum part ems apst`. The ports-implemented register is `0xc`: ports 2 and 3 of the six the capability register counts. Linux `ata3` is port 2 and `ata4` is port 3. `ata1`, `ata2`, `ata5`, and `ata6` are `DUMMY`. The Intel PCS quirk in that file ORs this map into the PCS register. It does not clear ports. The manual still has six SATA connectors, an M.2 socket, and a SATA Express connector, and M.2 or SATA Express occupies SATA3 4/5. This boot has no third ATA device and no NVMe device.

| Linux | Port | Link | Identity |
| --- | --- | --- | --- |
| `ata3` → `sda` | AHCI 2, ABAR port `0xf7e39200` | 6.0 Gbps, SStatus `133`, UDMA/133 | WDC WDS500G2B0A-00SM50, firmware `415020WD` (ATA identify `20WD`), ATA-11. 976773168 logical 512-byte sectors (500 GB / 466 GiB). Physical block 512. NCQ depth 32. Dev-Sleep, DIPM. Write cache on. Discard granularity 512, max discard 2147450880 bytes. Rotational 0. WWN `naa.5001b444a70a980f`. Serial `211282445014` |
| `ata4` → `sdb` | AHCI 3, ABAR port `0xf7e39280` | 6.0 Gbps, SStatus `133`, UDMA/133 | WDC WD20EZAZ-00L9GB0, firmware `80.00A80`. ATA-10. 3907029168 logical 512-byte sectors (2.00 TB / 1.82 TiB). Physical block 4096. NCQ depth 32, HIPM, DIPM, NCQ priority. Write cache on. Discard granularity 4096. Rotational 1. WWN `naa.50014ee2bee82a29`. Serial `WD-WX92A616U7Y3` |

`sda` is the booted disk. `sda1` is 2 GiB FAT at `/boot` (partition start sector 2048, 4194304 sectors). `sda2` is the rest, LUKS, opened as `root`, btrfs, mounted at `/`. `sdb` is the second disk and is not the root: `sdb1` about 977 GiB ext4 with partition label `ubuntu`, `sdb2` about 187 GiB FAT labelled `lerux`, `sdb3` 2 GiB FAT labelled `OMARCHY_EFI`, `sdb4` about 697 GiB LUKS with partition label `OMARCHY_ROOT`. Queue scheduler on both is `kyber`. The image-write recipes in [boards.md](../boards/boards.md) already refuse these internal disks.

The plugged-in USB stick is `sdc`, SanDisk `0781:5597`, product string `SanDisk 3.2Gen1`, revision `1.00`, on the xHCI SuperSpeed bus at 5000 Mb/s. 60088320 logical sectors (30.8 GB / 28.7 GiB). It enumerates as an ISO9660 volume `ISOIMAGE` with volume date `2026-11-27-06-47-39`. The kernel attached it with `usb-storage` (`usb-storage 4-1:1.0`). Write cache off. Three small partitions (32 KiB, 8 MiB FAT, 304 KiB). This pass did not mount it.

Swap is zram, 16609443840 bytes, compressor `zstd`. That is host configuration, not a device on the board.

## Ethernet

`e1000e` matches `8086:153b` as `E1000_DEV_ID_PCH_LPT_I217_V` (`drivers/net/ethernet/intel/e1000e/hw.h`). The probe line is:

```text
e1000e 0000:00:19.0 eth0: (PCI Express:2.5GT/s:Width x1) fc:aa:14:c8:00:9e
e1000e 0000:00:19.0 eth0: MAC: 11, PHY: 12, PBA No: FFFFFF-0FF
```

MAC type 11 is `e1000_pch_lpt` and PHY type 12 is `e1000_phy_i217` in the enums in that header. The interface was renamed `eno1`. `addr_assign_type` is 0, so `fc:aa:14:c8:00:9e` is the permanent station address. Interrupt throttling is the module default, dynamic conservative. The driver registered a PHC clock on this device.

At the time of the read the carrier was up, speed 1000, duplex full, MTU 1500. The kernel log says `NIC Link is Up 1000 Mbps Full Duplex, Flow Control: None`. DHCP had installed `192.168.111.211/24` and a default route via `192.168.111.1` with metric 100. Two ICMP echoes to that gateway on `eno1` returned, 2.10 ms and 1.44 ms, no loss. The interface was left up.

## Wireless and Bluetooth

The PCI function is an RTL8852BE, subsystem `1a3b:5470`. linux-hardware.org labels that subsystem AzureWave ([entry](https://linux-hardware.org/?id=pci%3A10ec-b852-1a3b-5470)). The driver is `rtw89_8852be` (`drivers/net/wireless/realtek/rtw89/rtw8852be.c`), which matches `10ec:b852` and `10ec:b85b` and uses `rtw8852b_chip_info` with the AX PCI profile (truncate mode, 2048-byte transmit bursts, 128-byte receive bursts, 8 tags). This card's link is 2.5 GT/s x1, which is the maximum the endpoint advertises. The root port could do 5.0 GT/s x1.

Firmware request `rtw89/rtw8852b_fw-2.bin`. The running driver logged version `0.29.29.18` (`9e3d777f`), command version 0, types 5 and 3. Element versions: BB `00 28`, radio A `00 32`, NCTL `00 0a`, TXPWR `00 43`, power track `00 32`, regulatory `00 49 00 33`. Chip info `CID: 0, CV: 1, AID: 0, ACV: 1, RFE: 1`. On disk that file is `/lib/firmware/rtw89/rtw8852b_fw-2.bin.zst` (264 KiB compressed). The interface is `wlp5s0`. rfkill `phy0` is unblocked, soft and hard.

`iw dev wlp5s0 link` during this pass:

| | |
| --- | --- |
| Association | joined an access point |
| Channel | 40, 5200 MHz, 80 MHz, center 5210 MHz |
| TX power | 23.00 dBm |
| Signal | −17 dBm |
| Rate | 1200.9 Mbit/s receive and transmit, 80 MHz HE-MCS 11, HE-NSS 2, HE-GI 0, HE-DCM 0 |
| BSS | short preamble, short slot, DTIM 1, beacon 100 |
| Address | `1c:ce:51:77:cd:c0` (`addr_assign_type` 3, so the address was set on the interface) |

DHCP had installed `192.168.111.33/24` and a second default route via `192.168.111.1` with metric 600, so the Ethernet route stays preferred. Two ICMP echoes to that gateway on `wlp5s0` returned, 1.50 ms and 2.32 ms, no loss. The association was left in place. Counters at that moment were about 36.5 MB received and 250 KB transmitted.

Bluetooth is the USB function of the same card, not a second radio. `13d3:3570`, manufacturer `Realtek`, product `Bluetooth Radio`, serial `00e04c000001` (the Realtek placeholder serial), full speed, 500 mA, on xHCI port path `2-14`. `btusb` matches `USB_DEVICE(0x13d3, 0x3570)` with `BTUSB_REALTEK | BTUSB_WIDEBAND_SPEECH` in `drivers/bluetooth/btusb_main.c`. The class is wireless controller `e0/01/01`. The probe logged `hci_ver=0b hci_rev=000b lmp_ver=0b lmp_subver=8852`, ROM version 1, then loaded `rtl_bt/rtl8852bu_fw.bin` and `rtl_bt/rtl8852bu_config.bin` (`cfg_sz 6`, total size 62539) and reported firmware `0x127cfd78`. On this host the config file is a symlink to `rtl8761bu_config.bin.zst`. The firmware file is 72 KiB compressed. `hci0` came up, rfkill unblocked, AOSP extensions v1.00. The sysfs address file was empty for this user, so the controller address was not read. No peer was paired.

## USB and input

xHCI `8086:8cb1` is HCI version `0x100`, HCC params `0x200077c1`, quirks `0x0000000000009810`. It publishes USB 2 bus 2 and USB 3 bus 4. EHCI `00:1d.0` is bus 1 and EHCI `00:1a.0` is bus 3, each EHCI 1.00. Each EHCI bus has one Intel rate-matching hub and no other device: `8087:8001` on bus 1 and `8087:8009` on bus 3, both high speed. The peripherals sit on xHCI.

| Path | Speed | ID | What enumerated |
| --- | --- | --- | --- |
| `2-4` | full | `045e:07b2` bcd `0743` | Microsoft, `Microsoft® 2.4GHz Transceiver v7.0`, 100 mA. Input nodes: keyboard, mouse, consumer control, system control |
| `2-10` | full | `04d9:fc4d` bcd `0152` | Holtek vendor id, product `USB Gaming Mouse`, 100 mA. Mouse plus consumer-control nodes |
| `2-14` | full | `13d3:3570` | Realtek Bluetooth, above |
| `4-1` | SuperSpeed | `0781:5597` | SanDisk stick, above |

`i8042` is not loaded. The only PNP device is `PNP0501`, and the kernel line is `i8042: PNP: No PS/2 controller found.` The manual has a rear PS/2 combo port and a setup option that can disable PS/2 devices before the OS starts. This boot's ACPI namespace has the serial port and does not have a PS/2 keyboard or mouse. The keyboard that did enumerate is the Microsoft transceiver on port 4, the same `045e:07b2` the desk image types with. [z97-keyboard-retrospective.md](z97-keyboard-retrospective.md) records that bring-up. A lerux boot in that note also saw a device on port 16. This Linux boot's connected paths are 4, 10, and 14.

## Display

`00:02.0` is boot VGA (`boot_vga` through vgaarb: the Intel device owns I/O and memory, the Radeon owns neither). `i915` reports Haswell device id `0412`, integrated display version 7.00, and `hsw_gt2_info` is the match: `include/drm/intel/pciids.h` lists `0x0412` as GT2 desktop inside `INTEL_HSW_GT2_IDS`, and `drivers/gpu/drm/i915/i915_pci.c` binds that set to `hsw_gt2_info` (`.gt = 2`). During this pass `gt_cur_freq_mhz` was 500, `gt_act_freq_mhz` 450, `gt_max_freq_mhz` 1200, `gt_boost_freq_mhz` 1250. Intel's published dynamic maximum for this CPU's graphics is 1.20 GHz. Runtime PM state was active.

Connectors on card1 (the Intel device):

| Connector | Status |
| --- | --- |
| `HDMI-A-1` | connected, enabled, DPMS On |
| `HDMI-A-2` | disconnected |
| `VGA-1` | disconnected |

`fb0` is `i915drmfb`, 1920×1080, 32 bits per pixel, stride 7680, pan 0,0. A `simpledrm` framebuffer was registered first and `i915` then took `fb0`. The EDID on `HDMI-A-1` is 256 bytes, version 1.3, one CEA extension:

| Field | Value |
| --- | --- |
| Manufacturer | `DEL` |
| Product | `0xd0a1` |
| Serial number field | 827807827 |
| Week / year | 2 / 2017 |
| Screen size | 60 cm by 34 cm. Detailed timing image size 598 mm by 336 mm |
| Name descriptor | `SE2717H/HX` |
| Serial descriptor | `K9V8V71F1WXS` |
| Preferred timing | 148.50 MHz, 1920×1080, horizontal blank 280, vertical blank 45, which is 60 Hz |
| CEA | revision 3, video block and two vendor blocks. The first vendor block starts `030c00` (HDMI) |

The cable is on the Intel digital output that Linux calls `HDMI-A-1`. This pass did not move the cable, so it does not decide which of the rear HDMI and DVI-D jacks is `HDMI-A-2`. The VGA jack is the disconnected `VGA-1`. A 2026-10-22 firmware boot recorded in [boards.md](../boards/boards.md) published `0xd0000000`, pitch 4096, 1024×768. The aperture base is the same 256 MiB window. The mode inside it on this boot is the 1920×1080 mode above.

`01:00.0` is subsystem vendor `1da2` (Sapphire, per AMD's subsystem-vendor list) and subsystem device `e348`. `amdgpu` initialized it as POLARIS11, PCI id `1002:67ff` / `1da2:e348`, revision `0xcf`. The log before runtime suspend:

| | |
| --- | --- |
| VBIOS | fetched from the platform. `113-534801HM-U01`, build 392351, version `015.050.002.001.000000`, 2017/07/28 |
| Memory | 4096 MiB VRAM, 128-bit GDDR5. BAR 256 MiB. GART 256 MiB. GTT 7920 MiB |
| Geometry | 2 shader engines, 1 shader array per engine, 8 compute units per array, 16 active compute units |
| Blocks | `vi_common`, `gmc_v8`, `tonga_ih`, `gfx_v8`, `sdma_v3`, powerplay, display-core DCE 11.2, UVD 1.130 family 16, VCE firmware 53.26 binary id 3 |
| Audio bind | `snd_hda_intel` on `01:00.1`, forced non-snoop, bound through `amdgpu_dm_audio_component_bind_ops` |

`Cannot find any crtc or sizes` repeated, and every connector on card0 (`DP-1`, `DVI-D-1`, `HDMI-A-3`) was disconnected and disabled. Runtime status was `suspended`, control `auto`, and `sensors` then returned no fan, edge, or clock reading. The link width recorded above was sampled while the device was suspended, from the PCI link registers. Public VBIOS dumps for subsystem `1da2:e348` include 2 GB Sapphire RX 560 images with a different board string and an earlier build date. This card's probed VRAM size is 4096 MiB and its ATOM string is `113-534801HM-U01`, so those 2 GB dumps are a different board that reused the subsystem id.

## Audio

Three `snd_hda_intel` cards.

| Card | PCI | Codec | At |
| --- | --- | --- | --- |
| `HDMI` | `00:03.0` | Intel Haswell HDMI, vendor `0x80862807`, subsystem `0x80860101`, revision `0x100000` | `0xf7e34000` |
| `PCH` | `00:1b.0` | Realtek ALC1150, vendor `0x10ec0900`, subsystem `0x1458a182`, revision `0x100001` | `0xf7e30000` |
| `HDMI_1` | `01:00.1` | driver name `ATI R6xx HDMI`, vendor `0x1002aa01`, subsystem `0x00aa0100` | `0xf7d60000` |

`snd_hda_codec_alc882` matches `hdaudio:v10EC0900`. The autoconfig line is `ALC1150: SKU not ready 0x00000000`, then line outs 4 (`0x14`/`0x15`/`0x16`/`0x17`, type line), headphone `0x1b`, digital out `0x11`, rear mic `0x18`, front mic `0x19`, line in `0x1a`, no speaker outputs. That is the rear six jacks plus the front-panel header. Analog playback rates reported on the front DAC: 44100, 48000, 96000, 192000, 16/20/24-bit PCM. The Haswell HDMI codec exposes three converters at 32–192 kHz, 16/20/24/32-bit, PCM and AC3. The Radeon function exposes five HDMI PCM nodes. No stream was played.

## Serial, SMBus, and the Management Engine

`PNP0501` is active at I/O `0x3f8`–`0x3ff`, IRQ 4, high edge, no DMA. The 8250 driver created `ttyS0` there: `16550A`, `base_baud` 115200. The ACPI options also list `0x2f8`, `0x3e8`, and `0x2e8` at lower priority. Only `0x3f8` is the active set. The manual calls this header `COMA`. Nothing was written to the port.

The SMBus driver enabled the function (`0001` → `0003`), reported SPD Write Disable, and uses the PCI interrupt rather than polling. The two DIMM EEPROMs above are the devices that answered.

`mei_me` bound `8086:8cba`. The device node is `/dev/mei0`. A handful of interrupts had been delivered by the time of the snapshot. No Management Engine command was sent.

## What this boot confirmed

| Check | Result |
| --- | --- |
| I217-V link | carrier up, 1000 Mb/s full duplex, flow control off, two gateway echoes on `eno1` |
| RTL8852BE link | associated on 5 GHz channel 40, 80 MHz, HE-MCS 11, two streams, 1200.9 Mbit/s each way, signal −17 dBm, two gateway echoes on `wlp5s0` |
| Bluetooth controller | firmware `0x127cfd78` loaded, `hci0` up, rfkill clear |
| AHCI | both implemented ports at 6.0 Gbps, both drives identified, `sda` mounted as the host root |
| SPD | both EEPROMs read, 8 GiB DDR3-1600 each, parts as above. SPD writes are disabled |
| Intel display | `HDMI-A-1` connected at 1920×1080, EDID name `SE2717H/HX` |
| Radeon | enumerated, 4096 MiB reported, no connected output, runtime-suspended afterwards |
| COM1 | `ttyS0` is a 16550A at `0x3f8` IRQ 4 |
| RTC | CMOS clock reads back UTC |
| CPU | four cores online, frequencies above the 3.50 GHz base, microcode `0x28` |
| PS/2 | no PNP keyboard or mouse. The USB HID devices are the ones that enumerated |

## Where lerux stands on these devices

The one Z97 image is `pc_z97_d3h_i217`, described in [platforms.md](../boards/platforms.md#z97-desk-devices). This boot is the Linux side of the same board.

| Device | This boot | In the lerux tree |
| --- | --- | --- |
| COM1 `0x3f8` IRQ 4 | 16550A | the I217 image has no serial driver |
| Firmware framebuffer | aperture `0xd0000000`, this Linux mode 1920×1080 | `console-driver` paints the 80 by 25 shell in that aperture. The recorded GOP mode is 1024×768 |
| USB keyboard | Microsoft transceiver `045e:07b2` on xHCI port 4, BAR `0xf7e20000` | `console-driver`. The typing path is [z97-keyboard-retrospective.md](z97-keyboard-retrospective.md) |
| PS/2 | not in this ACPI namespace | `console-driver` still has the i8042 path for the rear port |
| I217-V | `e1000e`, the windows in the table above | `sddf-e1000e-driver` |
| RTL8852BE | `rtw89_8852be` plus the USB Bluetooth function | no image drives the card |
| AHCI `8086:8c82` | ports 2 and 3, two drives, host OS on `sda` | `sddf-ahci-driver` accepts the id. The desk image does not map the controller |
| HD Graphics 4600 and the RX 560 | both bound, Intel is scanout | the display engine is not programmed. The shell is painted into the firmware buffer |
| ALC1150, MEI, SMBus, EHCI | bound | none |
