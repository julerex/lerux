pub use crate::table::SpelledAddr;

use crate::{
    ahci::{msi_data, PciAddress, Unclaimed, AHCI_IOAPIC_VECTOR},
    serial_kind,
    table::{
        AARCH64_UART_GUEST, AARCH64_UART_IRQ, AARCH64_UART_PHYS, AARCH64_UART_SETVAR,
        AARCH64_VIRTIO_BLK_IRQ, AARCH64_VIRTIO_BLK_OFFSET, AARCH64_VIRTIO_GPU_IRQ,
        AARCH64_VIRTIO_GPU_OFFSET, AARCH64_VIRTIO_NET_IRQ, AARCH64_VIRTIO_NET_OFFSET,
        AARCH64_VIRTIO_WINDOW_GUEST, AARCH64_VIRTIO_WINDOW_NAME, AARCH64_VIRTIO_WINDOW_PHYS,
        AARCH64_VIRTIO_WINDOW_SIZE, RISCV64_BLK_BUS, RISCV64_BLK_IRQ, RISCV64_BLK_PHYS,
        RISCV64_GPU_BUS, RISCV64_GPU_IRQ, RISCV64_GPU_PHYS, RISCV64_NET_BUS, RISCV64_NET_IRQ,
        RISCV64_NET_PHYS, RISCV64_UART_GUEST, RISCV64_UART_IRQ, RISCV64_UART_PHYS,
        RISCV64_UART_SETVAR, RISCV64_VIRTIO_MMIO_OFFSET, SERIAL_MMIO_SIZE, SERIAL_MMIO_WINDOW_NAME,
        X86_BLK_BAR0, X86_BLK_BAR1, X86_BLK_DEVICE, X86_BLK_FUNCTION, X86_BLK_PIN, X86_BLK_VECTOR,
        X86_NET_BAR0, X86_NET_BAR1, X86_NET_DEVICE, X86_NET_FUNCTION, X86_NET_PIN, X86_NET_VECTOR,
        X86_PCI_CONFIG_ADDR_SYMBOL, X86_PCI_CONFIG_ID_SYMBOL, X86_PCI_CONFIG_PORT,
        X86_PCI_CONFIG_SIZE, X86_PCI_NET_CONFIG_ADDR_SYMBOL, X86_PCI_NET_CONFIG_ID_SYMBOL,
        X86_UART_ADDR_SYMBOL, X86_UART_ID_SYMBOL, X86_UART_PIN, X86_UART_PORT, X86_UART_PORT_SIZE,
        X86_UART_VECTOR,
    },
    Arch, Serial, VirtioRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineIrq(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoApicPin(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoApicVector(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Level,
    Edge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    High,
    Low,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X86Irq {
    pub pin: IoApicPin,
    pub vector: IoApicVector,
    pub trigger: Trigger,
    pub polarity: Polarity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MsiIrq {
    pub pcidev: PciAddress,
    pub handle: u32,
    pub vector: IoApicVector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Irq {
    Line(LineIrq),
    X86(X86Irq),
    Msi(MsiIrq),
}

pub fn pci_config_port() -> IoPort {
    IoPort {
        addr: X86_PCI_CONFIG_PORT,
        size: X86_PCI_CONFIG_SIZE,
        id_symbol: X86_PCI_CONFIG_ID_SYMBOL,
        addr_symbol: X86_PCI_CONFIG_ADDR_SYMBOL,
        element_id: None,
    }
}

/// Config port at `0xcf8` with the net symbol pair, distinct from AHCI's
/// `pci_config_ioport_*`.
pub fn pci_net_config_port() -> IoPort {
    IoPort {
        addr: X86_PCI_CONFIG_PORT,
        size: X86_PCI_CONFIG_SIZE,
        id_symbol: X86_PCI_NET_CONFIG_ID_SYMBOL,
        addr_symbol: X86_PCI_NET_CONFIG_ADDR_SYMBOL,
        element_id: None,
    }
}

/// Pin 11, vector 50 for slot `00:04.0` when e1000e is the only PCI device.
/// The fetch image that also has ICH9 AHCI uses [`e1000e_fetch_irq`].
pub fn e1000e_standalone_irq() -> X86Irq {
    x86_level_low(X86_NET_PIN, X86_NET_VECTOR)
}

/// Pin 10, vector 50, level, active-low for e1000e at `00:04.0` beside ICH9 AHCI.
/// Standalone e1000e keeps [`e1000e_standalone_irq`].
pub fn e1000e_fetch_irq() -> X86Irq {
    x86_level_low(10, 50)
}

pub fn ahci_msi() -> MsiIrq {
    MsiIrq {
        pcidev: Unclaimed::compiled().address(),
        handle: msi_data(AHCI_IOAPIC_VECTOR),
        vector: IoApicVector(AHCI_IOAPIC_VECTOR),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoPort {
    pub addr: SpelledAddr,
    pub size: u16,
    pub id_symbol: &'static str,
    pub addr_symbol: &'static str,
    /// Fixed `<ioport id>` when two protection domains share one config address.
    pub element_id: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MmioWindow {
    pub name: &'static str,
    pub phys: SpelledAddr,
    pub vaddr: u64,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MmioDevice {
    pub window: MmioWindow,
    pub offset: u64,
    pub irq: LineIrq,
    pub setvar: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialNode {
    Mmio(MmioDevice),
    Port { port: IoPort, irq: X86Irq },
}

pub fn serial_node(arch: Arch) -> SerialNode {
    match serial_kind(arch) {
        Serial::Pl011 => SerialNode::Mmio(MmioDevice {
            window: MmioWindow {
                name: SERIAL_MMIO_WINDOW_NAME,
                phys: AARCH64_UART_PHYS,
                vaddr: AARCH64_UART_GUEST,
                size: SERIAL_MMIO_SIZE,
            },
            offset: 0,
            irq: LineIrq(AARCH64_UART_IRQ),
            setvar: AARCH64_UART_SETVAR,
        }),
        Serial::Ns16550Mmio => SerialNode::Mmio(MmioDevice {
            window: MmioWindow {
                name: SERIAL_MMIO_WINDOW_NAME,
                phys: RISCV64_UART_PHYS,
                vaddr: RISCV64_UART_GUEST,
                size: SERIAL_MMIO_SIZE,
            },
            offset: 0,
            irq: LineIrq(RISCV64_UART_IRQ),
            setvar: RISCV64_UART_SETVAR,
        }),
        Serial::Ns16550Port => SerialNode::Port {
            port: IoPort {
                addr: X86_UART_PORT,
                size: X86_UART_PORT_SIZE,
                id_symbol: X86_UART_ID_SYMBOL,
                addr_symbol: X86_UART_ADDR_SYMBOL,
                element_id: None,
            },
            irq: x86_level_low(X86_UART_PIN, X86_UART_VECTOR),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciBar {
    pub phys: SpelledAddr,
    pub setvar: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtioNode {
    Mmio(MmioDevice),
    Pci {
        config: IoPort,
        device: u8,
        function: u8,
        irq: X86Irq,
        bars: &'static [PciBar],
    },
}

static X86_BLK_BARS: [PciBar; 2] = [
    PciBar {
        phys: X86_BLK_BAR0,
        setvar: "virtio_blk_bar0",
    },
    PciBar {
        phys: X86_BLK_BAR1,
        setvar: "virtio_blk_bar1",
    },
];

static X86_NET_BARS: [PciBar; 2] = [
    PciBar {
        phys: X86_NET_BAR0,
        setvar: "virtio_net_bar0",
    },
    PciBar {
        phys: X86_NET_BAR1,
        setvar: "virtio_net_bar1",
    },
];

pub fn virtio_node(arch: Arch, role: VirtioRole) -> Option<VirtioNode> {
    match arch {
        Arch::Aarch64 => Some(VirtioNode::Mmio(aarch64_virtio_mmio(role))),
        // Guest virtual address equals the physical page. The AArch64 high
        // window does not boot on QEMU RISC-V. Milestone 23 records the exit
        // if Microkit can place a high guest map. Until then, keep the identity.
        Arch::Riscv64 => match role {
            VirtioRole::Blk => Some(VirtioNode::Mmio(MmioDevice {
                window: MmioWindow {
                    name: "virtio_blk_mmio",
                    phys: RISCV64_BLK_PHYS,
                    vaddr: RISCV64_BLK_PHYS.value(),
                    size: SERIAL_MMIO_SIZE,
                },
                offset: RISCV64_VIRTIO_MMIO_OFFSET,
                irq: LineIrq(RISCV64_BLK_IRQ),
                setvar: "virtio_blk_mmio_vaddr",
            })),
            VirtioRole::Net => Some(VirtioNode::Mmio(MmioDevice {
                window: MmioWindow {
                    name: "virtio_net_mmio",
                    phys: RISCV64_NET_PHYS,
                    vaddr: RISCV64_NET_PHYS.value(),
                    size: SERIAL_MMIO_SIZE,
                },
                offset: RISCV64_VIRTIO_MMIO_OFFSET,
                irq: LineIrq(RISCV64_NET_IRQ),
                setvar: "virtio_net_mmio_vaddr",
            })),
            VirtioRole::Gpu => Some(VirtioNode::Mmio(MmioDevice {
                window: MmioWindow {
                    name: "virtio_gpu_mmio",
                    phys: RISCV64_GPU_PHYS,
                    vaddr: RISCV64_GPU_PHYS.value(),
                    size: SERIAL_MMIO_SIZE,
                },
                offset: RISCV64_VIRTIO_MMIO_OFFSET,
                irq: LineIrq(RISCV64_GPU_IRQ),
                setvar: "virtio_gpu_mmio_vaddr",
            })),
        },
        Arch::X86_64 => match role {
            VirtioRole::Blk => Some(VirtioNode::Pci {
                config: pci_config_port(),
                device: X86_BLK_DEVICE,
                function: X86_BLK_FUNCTION,
                irq: x86_level_low(X86_BLK_PIN, X86_BLK_VECTOR),
                bars: &X86_BLK_BARS,
            }),
            VirtioRole::Net => Some(VirtioNode::Pci {
                config: pci_net_config_port(),
                device: X86_NET_DEVICE,
                function: X86_NET_FUNCTION,
                irq: x86_level_low(X86_NET_PIN, X86_NET_VECTOR),
                bars: &X86_NET_BARS,
            }),
            VirtioRole::Gpu => None,
        },
    }
}

const fn aarch64_virtio_mmio(role: VirtioRole) -> MmioDevice {
    let (offset, irq) = match role {
        VirtioRole::Net => (AARCH64_VIRTIO_NET_OFFSET, AARCH64_VIRTIO_NET_IRQ),
        VirtioRole::Blk => (AARCH64_VIRTIO_BLK_OFFSET, AARCH64_VIRTIO_BLK_IRQ),
        VirtioRole::Gpu => (AARCH64_VIRTIO_GPU_OFFSET, AARCH64_VIRTIO_GPU_IRQ),
    };
    MmioDevice {
        window: MmioWindow {
            name: AARCH64_VIRTIO_WINDOW_NAME,
            phys: AARCH64_VIRTIO_WINDOW_PHYS,
            vaddr: AARCH64_VIRTIO_WINDOW_GUEST,
            size: AARCH64_VIRTIO_WINDOW_SIZE,
        },
        offset,
        irq: LineIrq(irq),
        setvar: match role {
            VirtioRole::Net => "virtio_net_mmio_vaddr",
            VirtioRole::Blk => "virtio_blk_mmio_vaddr",
            VirtioRole::Gpu => "virtio_gpu_mmio_vaddr",
        },
    }
}

/// QEMU level lines are active-low. Microkit 2.3.0 programs the polarity
/// attribute as written. The 2.2.0 tool inverted it, so these descriptions
/// used to say high.
const fn x86_level_low(pin: u8, vector: u8) -> X86Irq {
    X86Irq {
        pin: IoApicPin(pin),
        vector: IoApicVector(vector),
        trigger: Trigger::Level,
        polarity: Polarity::Low,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QemuDevice {
    VirtioMmio {
        role: VirtioRole,
        bus: Option<u8>,
    },
    VirtioPci {
        role: VirtioRole,
        device: u8,
        function: u8,
    },
    UserNet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QemuPlan {
    pub devices: [Option<QemuDevice>; 4],
}

pub fn qemu_plan(arch: Arch, roles: &[VirtioRole]) -> QemuPlan {
    let mut devices = [None; 4];
    let mut slot = 0usize;

    match arch {
        Arch::Aarch64 => {
            let wants_net = roles.contains(&VirtioRole::Net);
            if !wants_net
                && roles.iter().any(|role| *role != VirtioRole::Net)
                && (roles.contains(&VirtioRole::Blk) || roles.contains(&VirtioRole::Gpu))
            {
                devices[slot] = Some(QemuDevice::UserNet);
                slot += 1;
            }
            for role in roles {
                if slot >= devices.len() {
                    break;
                }
                devices[slot] = Some(QemuDevice::VirtioMmio {
                    role: *role,
                    bus: None,
                });
                slot += 1;
            }
        }
        Arch::Riscv64 => {
            for role in roles {
                if slot >= devices.len() {
                    break;
                }
                let bus = match role {
                    VirtioRole::Blk => Some(RISCV64_BLK_BUS),
                    VirtioRole::Net => Some(RISCV64_NET_BUS),
                    VirtioRole::Gpu => Some(RISCV64_GPU_BUS),
                };
                devices[slot] = Some(QemuDevice::VirtioMmio { role: *role, bus });
                slot += 1;
            }
        }
        Arch::X86_64 => {
            for role in roles {
                if slot >= devices.len() {
                    break;
                }
                let (device, function) = match role {
                    VirtioRole::Blk => (X86_BLK_DEVICE, X86_BLK_FUNCTION),
                    VirtioRole::Net => (X86_NET_DEVICE, X86_NET_FUNCTION),
                    VirtioRole::Gpu => continue,
                };
                devices[slot] = Some(QemuDevice::VirtioPci {
                    role: *role,
                    device,
                    function,
                });
                slot += 1;
            }
        }
    }

    QemuPlan { devices }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VirtioAttach;

    #[test]
    fn aarch64_block_without_net_keeps_user_net_first() {
        let plan = qemu_plan(Arch::Aarch64, &[VirtioRole::Blk]);
        assert!(matches!(plan.devices[0], Some(QemuDevice::UserNet)));
        assert_eq!(
            crate::virtio(Arch::Aarch64, VirtioRole::Blk),
            Some(VirtioAttach::Mmio {
                offset: crate::table::AARCH64_VIRTIO_BLK_OFFSET,
            })
        );
    }

    #[test]
    fn pci_net_config_port_is_not_the_ahci_port() {
        let net = pci_net_config_port();
        let ahci = pci_config_port();
        assert_eq!(net.addr, ahci.addr);
        assert_eq!(net.size, ahci.size);
        assert_eq!(net.id_symbol, "pci_net_config_ioport_id");
        assert_eq!(net.addr_symbol, "pci_net_config_ioport_addr");
        assert_ne!(net.id_symbol, ahci.id_symbol);
        assert_ne!(net.addr_symbol, ahci.addr_symbol);
        assert_eq!(net.element_id, None);
    }

    #[test]
    fn e1000e_standalone_irq_is_the_provisional_pin() {
        let irq = e1000e_standalone_irq();
        assert_eq!(irq.pin, IoApicPin(11));
        assert_eq!(irq.vector, IoApicVector(50));
        assert_eq!(irq.trigger, Trigger::Level);
        assert_eq!(irq.polarity, Polarity::Low);
        assert_eq!(irq, x86_level_low(X86_NET_PIN, X86_NET_VECTOR));
    }

    #[test]
    fn e1000e_fetch_irq_is_pin_10_vector_50() {
        let irq = e1000e_fetch_irq();
        assert_eq!(irq.pin, IoApicPin(10));
        assert_eq!(irq.vector, IoApicVector(50));
        assert_eq!(irq.trigger, Trigger::Level);
        assert_eq!(irq.polarity, Polarity::Low);
        assert_ne!(irq, e1000e_standalone_irq());
    }

    #[test]
    fn x86_virtio_net_uses_the_shared_config_port() {
        let Some(VirtioNode::Pci { config, irq, .. }) = virtio_node(Arch::X86_64, VirtioRole::Net)
        else {
            panic!("x86 virtio net");
        };
        assert_eq!(config, pci_net_config_port());
        assert_eq!(irq, x86_level_low(X86_NET_PIN, X86_NET_VECTOR));
    }

    #[test]
    fn riscv_virtio_guest_address_matches_the_physical_page() {
        for role in [VirtioRole::Blk, VirtioRole::Net, VirtioRole::Gpu] {
            let Some(VirtioNode::Mmio(device)) = virtio_node(Arch::Riscv64, role) else {
                panic!("riscv virtio node");
            };
            assert_eq!(
                device.window.vaddr,
                device.window.phys.value(),
                "riscv virtio guest map stays the physical page"
            );
            assert_ne!(device.window.vaddr, 0x6_000_000_000);
        }
    }
}
