pub use crate::table::SpelledAddr;

use crate::{
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
pub enum Irq {
    Line(LineIrq),
    X86(X86Irq),
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
            irq: x86_level_high(X86_UART_PIN, X86_UART_VECTOR),
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
                config: IoPort {
                    addr: X86_PCI_CONFIG_PORT,
                    size: X86_PCI_CONFIG_SIZE,
                    id_symbol: X86_PCI_CONFIG_ID_SYMBOL,
                    addr_symbol: X86_PCI_CONFIG_ADDR_SYMBOL,
                    element_id: None,
                },
                device: X86_BLK_DEVICE,
                function: X86_BLK_FUNCTION,
                irq: x86_level_high(X86_BLK_PIN, X86_BLK_VECTOR),
                bars: &X86_BLK_BARS,
            }),
            VirtioRole::Net => Some(VirtioNode::Pci {
                config: IoPort {
                    addr: X86_PCI_CONFIG_PORT,
                    size: X86_PCI_CONFIG_SIZE,
                    id_symbol: X86_PCI_NET_CONFIG_ID_SYMBOL,
                    addr_symbol: X86_PCI_NET_CONFIG_ADDR_SYMBOL,
                    element_id: None,
                },
                device: X86_NET_DEVICE,
                function: X86_NET_FUNCTION,
                irq: x86_level_high(X86_NET_PIN, X86_NET_VECTOR),
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

const fn x86_level_high(pin: u8, vector: u8) -> X86Irq {
    X86Irq {
        pin: IoApicPin(pin),
        vector: IoApicVector(vector),
        trigger: Trigger::Level,
        polarity: Polarity::High,
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
}
