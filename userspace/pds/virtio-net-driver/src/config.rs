pub mod channels {
    use sel4_microkit::Channel;

    pub const DEVICE: Channel = Channel::new(0);
    pub const CLIENT: Channel = Channel::new(1);
}

#[cfg(any(
    feature = "board-qemu_virt_riscv64_virtio",
    feature = "board-qemu_virt_riscv64_http",
    feature = "board-qemu_virt_riscv64_net"
))]
pub const VIRTIO_NET_MMIO_OFFSET: usize = 0;

#[cfg(all(
    not(feature = "board-qemu_virt_riscv64_virtio"),
    not(feature = "board-qemu_virt_riscv64_http"),
    not(feature = "board-qemu_virt_riscv64_net"),
    not(feature = "board-x86_64_generic_virtio"),
    not(feature = "board-x86_64_generic_http")
))]
pub const VIRTIO_NET_MMIO_OFFSET: usize = 0xe00;

#[cfg(not(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
)))]
pub const VIRTIO_NET_MMIO_SIZE: usize = 0x200;
pub const VIRTIO_NET_DRIVER_DMA_SIZE: usize = 0x200_000;
pub const VIRTIO_NET_CLIENT_DMA_SIZE: usize = 0x200_000;

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
))]
pub mod pci {
    use lerux_virtio_hal::BarRegion;

    use virtio_drivers::transport::pci::bus::DeviceFunction;

    pub const BLK_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 3,
        function: 0,
    };

    pub const NET_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 4,
        function: 0,
    };

    #[cfg(feature = "board-x86_64_generic_virtio")]
    pub const BLK_BAR_PADDRS: &[u64] = &[0xfed0_0000, 0xfed1_0000];

    pub const NET_BAR1_PHYS: u64 = 0xfed2_0000;
    pub const NET_BAR1_VADDR: usize = NET_BAR1_PHYS as usize;
    pub const NET_BAR1_SIZE: usize = 0x1000;

    pub const NET_BAR4_PHYS: u64 = 0xfed3_0000;
    pub const NET_BAR4_VADDR: usize = NET_BAR4_PHYS as usize;
    pub const NET_BAR4_SIZE: usize = 0x4000;

    pub const NET_BAR_PADDRS: &[u64] = &[NET_BAR1_PHYS, NET_BAR4_PHYS];

    pub const NET_BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: NET_BAR1_PHYS as usize,
            vaddr: NET_BAR1_VADDR,
            size: NET_BAR1_SIZE,
        },
        BarRegion {
            paddr: NET_BAR4_PHYS as usize,
            vaddr: NET_BAR4_VADDR,
            size: NET_BAR4_SIZE,
        },
    ];
}
