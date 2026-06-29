pub mod channels {
    use sel4_microkit::Channel;

    pub const DEVICE: Channel = Channel::new(0);
    pub const CLIENT: Channel = Channel::new(1);
}

#[cfg(any(
    feature = "board-qemu_virt_riscv64_virtio",
    feature = "board-qemu_virt_riscv64_blk"
))]
pub const VIRTIO_BLK_MMIO_OFFSET: usize = 0;

#[cfg(all(
    not(feature = "board-qemu_virt_riscv64_virtio"),
    not(feature = "board-qemu_virt_riscv64_blk"),
    not(feature = "board-x86_64_generic_virtio")
))]
pub const VIRTIO_BLK_MMIO_OFFSET: usize = 0xc00;

#[cfg(not(feature = "board-x86_64_generic_virtio"))]
pub const VIRTIO_BLK_MMIO_SIZE: usize = 0x200;
pub const VIRTIO_BLK_DRIVER_DMA_SIZE: usize = 0x200_000;
pub const VIRTIO_BLK_CLIENT_DMA_SIZE: usize = 0x200_000;

#[cfg(feature = "board-x86_64_generic_virtio")]
pub mod pci {
    use lerux_virtio_hal::BarRegion;

    use virtio_drivers::transport::pci::bus::DeviceFunction;

    pub const BLK_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 3,
        function: 0,
    };

    pub const BLK_BAR1_PHYS: u64 = 0xfed0_0000;
    pub const BLK_BAR1_VADDR: usize = BLK_BAR1_PHYS as usize;
    pub const BLK_BAR1_SIZE: usize = 0x1000;

    pub const BLK_BAR4_PHYS: u64 = 0xfed1_0000;
    pub const BLK_BAR4_VADDR: usize = BLK_BAR4_PHYS as usize;
    pub const BLK_BAR4_SIZE: usize = 0x4000;

    pub const BLK_BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: BLK_BAR1_PHYS as usize,
            vaddr: BLK_BAR1_VADDR,
            size: BLK_BAR1_SIZE,
        },
        BarRegion {
            paddr: BLK_BAR4_PHYS as usize,
            vaddr: BLK_BAR4_VADDR,
            size: BLK_BAR4_SIZE,
        },
    ];
}
