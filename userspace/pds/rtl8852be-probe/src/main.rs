#![no_std]
#![no_main]

use lerux_logging::{log, serial};
use lerux_rtw8852be::{
    decode_bridge, identify, io_covered, mmio_covered, BridgeError, CardError, Identity, REALTEK,
};
use sel4::with_ipc_buffer_mut;
use sel4_microkit::{protection_domain, var, Channel, ChannelSet, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const BASE_IOPORT_CAP: u64 = 394;

const ROOT_BUS: u8 = 0;
const ROOT_DEV: u8 = 0x1c;
// The function number is 9-series root port 7 and is stable; the secondary bus
// is read from the bridge and must not be compiled in as bus 5.
const ROOT_FUNC: u8 = 6;

struct HandlerImpl;

fn out32(cap: u64, port: u16, value: u32) {
    with_ipc_buffer_mut(|ipc| {
        ipc.inner_mut()
            .seL4_X86_IOPort_Out32(cap, u64::from(port), u64::from(value));
    });
}

fn in32(cap: u64, port: u16) -> u32 {
    with_ipc_buffer_mut(|ipc| {
        let ret = ipc.inner_mut().seL4_X86_IOPort_In32(cap, port);
        ret.result
    })
}

fn config_address(bus: u8, dev: u8, func: u8, reg: u8) -> u32 {
    0x8000_0000
        | (u32::from(bus) << 16)
        | (u32::from(dev) << 11)
        | (u32::from(func) << 8)
        | (u32::from(reg) & 0xfc)
}

fn read_config(cap: u64, addr: u16, bus: u8, dev: u8, func: u8) -> [u8; 64] {
    let mut header = [0u8; 64];
    let data = addr + 4;
    for reg in (0u8..64).step_by(4) {
        out32(cap, addr, config_address(bus, dev, func, reg));
        let word = in32(cap, data);
        let offset = usize::from(reg);
        header[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    header
}

fn probe(cap: u64, addr: u16) {
    let root = read_config(cap, addr, ROOT_BUS, ROOT_DEV, ROOT_FUNC);
    let bridge = match decode_bridge(&root) {
        Ok(bridge) => bridge,
        Err(BridgeError::NotBridge(not_bridge)) => {
            let vendor = u16::from_le_bytes([root[0], root[1]]);
            let device = u16::from_le_bytes([root[2], root[3]]);
            log::info!(
                "rtl8852be: root 00:1c.6 not a bridge {:04x}:{:04x} header {:#x}",
                vendor,
                device,
                not_bridge.header_type,
            );
            return;
        }
        Err(BridgeError::Short(short)) => {
            log::info!("rtl8852be: root short {}", short.len);
            return;
        }
    };
    log::info!(
        "rtl8852be: root {:04x}:{:04x} secondary {} mem {:#x}-{:#x} io {:#x}-{:#x}",
        bridge.vendor.0,
        bridge.device.0,
        bridge.secondary_bus,
        bridge.mem_base,
        bridge.mem_limit_inclusive,
        bridge.io_base,
        bridge.io_limit_inclusive,
    );

    let endpoint = read_config(cap, addr, bridge.secondary_bus, 0, 0);
    match identify(&endpoint) {
        Ok(Identity::Rtl8852be(card)) => {
            log::info!(
                "rtl8852be: card {:04x}:{:04x} rev {} subsystem {:04x}:{:04x} mmio {:#x} io {:#x} mmio_covered {} io_covered {}",
                REALTEK.0,
                card.device.0,
                card.revision,
                card.subsystem_vendor,
                card.subsystem_device,
                card.mmio_phys,
                card.io_port,
                mmio_covered(&bridge, &card),
                io_covered(&bridge, &card),
            );
        }
        Ok(Identity::Other { vendor, device }) => {
            log::info!("rtl8852be: card other {:04x}:{:04x}", vendor.0, device.0);
        }
        Err(CardError::BarMismatch(mismatch)) => {
            log::info!("rtl8852be: bar mismatch {:04x}", mismatch.device.0);
        }
        Err(CardError::Short(short)) => {
            log::info!("rtl8852be: short config {}", short.len);
        }
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).expect("serial log");
    log::info!("rtl8852be: probe");

    let id = *var!(pci_ioport_id: usize = usize::MAX);
    let addr = *var!(pci_ioport_addr: usize = usize::MAX);
    if id == usize::MAX || addr == usize::MAX {
        log::info!("rtl8852be: no pci ports");
        return HandlerImpl;
    }

    probe(BASE_IOPORT_CAP + id as u64, addr as u16);
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        Ok(())
    }
}
