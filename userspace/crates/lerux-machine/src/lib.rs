//! Board-specific device facts shared by protection domains and the image builder.

#![no_std]

pub mod ahci;

mod table;

#[cfg(feature = "system-description")]
pub mod description;

#[cfg(feature = "system-description")]
pub use description::{
    qemu_plan, serial_node, virtio_node, IoApicPin, IoApicVector, IoPort, Irq, LineIrq, MmioDevice,
    MmioWindow, PciBar, Polarity, QemuDevice, QemuPlan, SerialNode, SpelledAddr, Trigger,
    VirtioNode, X86Irq,
};

use table::{
    AARCH64_VIRTIO_BLK_OFFSET, AARCH64_VIRTIO_GPU_OFFSET, AARCH64_VIRTIO_NET_OFFSET,
    RISCV64_VIRTIO_BLK_DMA_GUEST, RISCV64_VIRTIO_GPU_DMA_GUEST, RISCV64_VIRTIO_NET_DMA_GUEST,
    X86_BLK_BAR_VALUES, X86_BLK_DEVICE, X86_BLK_FUNCTION, X86_NET_BAR_VALUES, X86_NET_DEVICE,
    X86_NET_FUNCTION, X86_VIRTIO_BLK_DMA_PHYS, X86_VIRTIO_NET_DMA_PHYS,
};

pub use ahci::{
    ahci_map, AhciMap, Claimed, PciAddress, RejectedClass, Unclaimed, AHCI_CLASS,
    AHCI_IOAPIC_VECTOR, AHCI_PORT, QEMU_ICH9_AHCI, READY_LINE,
};

pub use table::{
    X86_PCI_ECAM_GUEST, X86_PCI_ECAM_PHYS, X86_PCI_ECAM_SETVAR, X86_PCI_ECAM_SIZE,
    X86_PCI_ECAM_WINDOW,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    Aarch64,
    Riscv64,
    X86_64,
}

impl Arch {
    pub const TARGET: Self = target_arch();

    /// Split `qemu_virt_aarch64_serial` into `(Aarch64, "serial")`.
    pub fn split_prefix(stem: &str) -> Option<(Self, &str)> {
        if let Some(rest) = stem.strip_prefix("qemu_virt_aarch64_") {
            return Some((Arch::Aarch64, rest));
        }
        if let Some(rest) = stem.strip_prefix("qemu_virt_riscv64_") {
            return Some((Arch::Riscv64, rest));
        }
        if let Some(rest) = stem.strip_prefix("x86_64_generic_") {
            return Some((Arch::X86_64, rest));
        }
        None
    }
}

const fn target_arch() -> Arch {
    #[cfg(target_arch = "aarch64")]
    {
        Arch::Aarch64
    }
    #[cfg(target_arch = "riscv64")]
    {
        Arch::Riscv64
    }
    #[cfg(target_arch = "x86_64")]
    {
        Arch::X86_64
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "x86_64"
    )))]
    {
        compile_error!("lerux-machine knows aarch64, riscv64, and x86_64")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serial {
    Pl011,
    Ns16550Mmio,
    Ns16550Port,
}

pub fn serial_kind(arch: Arch) -> Serial {
    match arch {
        Arch::Aarch64 => Serial::Pl011,
        Arch::Riscv64 => Serial::Ns16550Mmio,
        Arch::X86_64 => Serial::Ns16550Port,
    }
}

pub fn serial() -> Serial {
    serial_kind(Arch::TARGET)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtioRole {
    Blk,
    Net,
    Gpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtioAttach {
    Mmio {
        offset: u64,
    },
    Pci {
        device: u8,
        function: u8,
        bars: &'static [u64],
    },
}

/// Guest virtual address of the driver direct-memory-access region for one virtio role.
pub const fn virtio_dma_phys_addr(arch: Arch, role: VirtioRole) -> Option<&'static str> {
    match (arch, role) {
        (Arch::X86_64, VirtioRole::Net) => Some(X86_VIRTIO_NET_DMA_PHYS.text()),
        (Arch::X86_64, VirtioRole::Blk) => Some(X86_VIRTIO_BLK_DMA_PHYS.text()),
        _ => None,
    }
}

pub const fn virtio_dma_guest(arch: Arch, role: VirtioRole) -> u64 {
    match arch {
        Arch::Aarch64 => 0x8_000_000_000,
        Arch::Riscv64 => match role {
            VirtioRole::Net => RISCV64_VIRTIO_NET_DMA_GUEST,
            VirtioRole::Blk => RISCV64_VIRTIO_BLK_DMA_GUEST,
            VirtioRole::Gpu => RISCV64_VIRTIO_GPU_DMA_GUEST,
        },
        Arch::X86_64 => match role {
            VirtioRole::Net | VirtioRole::Blk => 0x8_000_000_000,
            VirtioRole::Gpu => 0,
        },
    }
}

pub const fn virtio(arch: Arch, role: VirtioRole) -> Option<VirtioAttach> {
    match arch {
        Arch::Aarch64 => Some(VirtioAttach::Mmio {
            offset: match role {
                VirtioRole::Net => AARCH64_VIRTIO_NET_OFFSET,
                VirtioRole::Blk => AARCH64_VIRTIO_BLK_OFFSET,
                VirtioRole::Gpu => AARCH64_VIRTIO_GPU_OFFSET,
            },
        }),
        Arch::Riscv64 => Some(VirtioAttach::Mmio {
            offset: match role {
                VirtioRole::Blk | VirtioRole::Net | VirtioRole::Gpu => 0,
            },
        }),
        Arch::X86_64 => match role {
            VirtioRole::Blk => Some(VirtioAttach::Pci {
                device: X86_BLK_DEVICE,
                function: X86_BLK_FUNCTION,
                bars: &X86_BLK_BAR_VALUES,
            }),
            VirtioRole::Net => Some(VirtioAttach::Pci {
                device: X86_NET_DEVICE,
                function: X86_NET_FUNCTION,
                bars: &X86_NET_BAR_VALUES,
            }),
            VirtioRole::Gpu => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_kind_per_arch() {
        assert_eq!(serial_kind(Arch::Aarch64), Serial::Pl011);
        assert_eq!(serial_kind(Arch::Riscv64), Serial::Ns16550Mmio);
        assert_eq!(serial_kind(Arch::X86_64), Serial::Ns16550Port);
    }

    #[test]
    fn virtio_mmio_offsets_aarch64() {
        assert_eq!(
            virtio(Arch::Aarch64, VirtioRole::Net),
            Some(VirtioAttach::Mmio { offset: 0xe00 })
        );
        assert_eq!(
            virtio(Arch::Aarch64, VirtioRole::Blk),
            Some(VirtioAttach::Mmio { offset: 0xc00 })
        );
        assert_eq!(
            virtio(Arch::Aarch64, VirtioRole::Gpu),
            Some(VirtioAttach::Mmio { offset: 0xa00 })
        );
    }

    #[test]
    fn virtio_absent_on_non_aarch64_gpu() {
        assert_eq!(
            virtio(Arch::Riscv64, VirtioRole::Gpu),
            Some(VirtioAttach::Mmio { offset: 0 })
        );
        assert_eq!(virtio(Arch::X86_64, VirtioRole::Gpu), None);
    }
}
