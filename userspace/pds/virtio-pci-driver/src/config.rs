pub mod channels {
    use sel4_microkit::Channel;

    pub const BLK_DEVICE: Channel = Channel::new(0);
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    pub const NET_CLIENT: Channel = Channel::new(1);
    pub const BLK_CLIENT: Channel = Channel::new(2);
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    pub const NET_DEVICE: Channel = Channel::new(3);
}

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_blk"
))]
pub const VIRTIO_DRIVER_DMA_SIZE: usize = 0x400_000;
#[cfg(feature = "board-x86_64_generic_http")]
pub const VIRTIO_DRIVER_DMA_SIZE: usize = 0x200_000;
#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
))]
pub const VIRTIO_NET_CLIENT_DMA_SIZE: usize = 0x200_000;

pub const VIRTIO_BLK_CLIENT_DMA_SIZE: usize = 0x200_000;

pub mod pci {
    use lerux_virtio_hal::BarRegion;
    use virtio_drivers::transport::pci::bus::DeviceFunction;

    pub const BLK_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 3,
        function: 0,
    };

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    pub const NET_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 4,
        function: 0,
    };

    pub const BLK_BAR1_PHYS: u64 = 0xfed0_0000;
    pub const BLK_BAR4_PHYS: u64 = 0xfed1_0000;
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    pub const NET_BAR1_PHYS: u64 = 0xfed2_0000;
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    pub const NET_BAR4_PHYS: u64 = 0xfed3_0000;

    pub const BLK_BAR_PADDRS: &[u64] = &[BLK_BAR1_PHYS, BLK_BAR4_PHYS];
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    pub const NET_BAR_PADDRS: &[u64] = &[NET_BAR1_PHYS, NET_BAR4_PHYS];

    #[cfg(feature = "board-x86_64_generic_virtio")]
    pub const BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: BLK_BAR1_PHYS as usize,
            vaddr: BLK_BAR1_PHYS as usize,
            size: 0x1000,
        },
        BarRegion {
            paddr: BLK_BAR4_PHYS as usize,
            vaddr: BLK_BAR4_PHYS as usize,
            size: 0x4000,
        },
        BarRegion {
            paddr: NET_BAR1_PHYS as usize,
            vaddr: NET_BAR1_PHYS as usize,
            size: 0x1000,
        },
        BarRegion {
            paddr: NET_BAR4_PHYS as usize,
            vaddr: NET_BAR4_PHYS as usize,
            size: 0x4000,
        },
    ];

    #[cfg(feature = "board-x86_64_generic_http")]
    pub const BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: NET_BAR1_PHYS as usize,
            vaddr: NET_BAR1_PHYS as usize,
            size: 0x1000,
        },
        BarRegion {
            paddr: NET_BAR4_PHYS as usize,
            vaddr: NET_BAR4_PHYS as usize,
            size: 0x4000,
        },
    ];

    #[cfg(feature = "board-x86_64_generic_blk")]
    pub const BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: BLK_BAR1_PHYS as usize,
            vaddr: BLK_BAR1_PHYS as usize,
            size: 0x1000,
        },
        BarRegion {
            paddr: BLK_BAR4_PHYS as usize,
            vaddr: BLK_BAR4_PHYS as usize,
            size: 0x4000,
        },
    ];
}
