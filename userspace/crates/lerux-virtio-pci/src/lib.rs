//! PCI virtio bring-up helpers for lerux x86 boards.

#![no_std]

mod ecam;
mod ioport;

use virtio_drivers::transport::pci::bus::{
    BarInfo, Command, ConfigurationAccess, DeviceFunction, MemoryBarType, PciRoot,
};
use virtio_drivers::transport::pci::{PciTransport, VirtioPciError};

use ecam::EcamAccess;
use ioport::IoPortCam;
use lerux_virtio_hal::HalImpl;

pub fn open_pci_root_ioport(ioport_id: u32, base_port: u16) -> PciRoot<IoPortCam> {
    PciRoot::new(IoPortCam::new(ioport_id, base_port))
}

pub fn open_pci_root_ecam(ecam_vaddr: usize) -> PciRoot<EcamAccess> {
    PciRoot::new(EcamAccess::new(ecam_vaddr))
}

fn disable_decode<C: ConfigurationAccess>(
    root: &mut PciRoot<C>,
    device: DeviceFunction,
) -> Command {
    let (_, command) = root.get_status_command(device);
    root.set_command(
        device,
        command & !(Command::IO_SPACE | Command::MEMORY_SPACE),
    );
    command
}

fn restore_decode<C: ConfigurationAccess>(
    root: &mut PciRoot<C>,
    device: DeviceFunction,
    command: Command,
) {
    root.set_command(
        device,
        command | Command::MEMORY_SPACE | Command::BUS_MASTER,
    );
}

fn assign_memory_bar<C: ConfigurationAccess>(
    root: &mut PciRoot<C>,
    device: DeviceFunction,
    bar_index: u8,
    paddr: u64,
    info: &BarInfo,
) {
    match info {
        BarInfo::Memory {
            address_type: MemoryBarType::Width64,
            ..
        } => root.set_bar_64(device, bar_index, paddr),
        BarInfo::Memory { .. } => root.set_bar_32(device, bar_index, paddr as u32),
        BarInfo::IO { .. } => root.set_bar_32(device, bar_index, paddr as u32),
    }
}

pub fn assign_memory_bars<C: ConfigurationAccess>(
    root: &mut PciRoot<C>,
    device: DeviceFunction,
    paddrs: &[u64],
) {
    let command = disable_decode(root, device);
    let bar_infos = root.bars(device).unwrap();
    let mut paddr_iter = paddrs.iter();
    for (bar_index, info) in bar_infos.iter().enumerate() {
        let Some(info) = info else {
            continue;
        };
        let BarInfo::Memory { size, .. } = info else {
            continue;
        };
        if *size == 0 {
            continue;
        }
        let paddr = *paddr_iter.next().expect("not enough BAR paddrs");
        assign_memory_bar(root, device, bar_index as u8, paddr, info);
    }
    restore_decode(root, device, command);
}

pub fn program_device_bars_ioport(
    ioport_id: u32,
    base_port: u16,
    device: DeviceFunction,
    paddrs: &[u64],
) {
    let mut root = open_pci_root_ioport(ioport_id, base_port);
    assign_memory_bars(&mut root, device, paddrs);
}

pub fn create_pci_transport_ioport(
    ioport_id: u32,
    base_port: u16,
    device: DeviceFunction,
    paddrs: &[u64],
) -> Result<PciTransport, VirtioPciError> {
    let mut root = open_pci_root_ioport(ioport_id, base_port);
    assign_memory_bars(&mut root, device, paddrs);
    PciTransport::new::<HalImpl, _>(&mut root, device)
}

pub fn create_pci_transport_ecam(
    ecam_vaddr: usize,
    device: DeviceFunction,
    paddrs: &[u64],
) -> Result<PciTransport, VirtioPciError> {
    let mut root = open_pci_root_ecam(ecam_vaddr);
    assign_memory_bars(&mut root, device, paddrs);
    PciTransport::new::<HalImpl, _>(&mut root, device)
}

pub fn create_pci_transport_ecam_bars_programmed(
    ecam_vaddr: usize,
    device: DeviceFunction,
) -> Result<PciTransport, VirtioPciError> {
    let mut root = open_pci_root_ecam(ecam_vaddr);
    PciTransport::new::<HalImpl, _>(&mut root, device)
}