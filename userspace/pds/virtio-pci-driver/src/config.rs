pub mod channels {
    use sel4_microkit::Channel;

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk"
    ))]
    pub const BLK_DEVICE: Channel = Channel::new(0);
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const NET_CLIENT: Channel = Channel::new(1);
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk"
    ))]
    pub const BLK_CLIENT: Channel = Channel::new(2);
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const NET_DEVICE: Channel = Channel::new(3);
}

/// `HalImpl` region at the base of `virtio_pci_driver_dma`.
#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_blk"
))]
pub const VIRTIO_DRIVER_DMA_SIZE: usize = 0x400_000;
/// Net-only: 1 MiB Hal + 1 MiB bounce in one 2 MiB MR (Phase 61).
#[cfg(any(
    feature = "board-x86_64_generic_http",
    feature = "board-x86_64_generic_net"
))]
pub const VIRTIO_DRIVER_DMA_SIZE: usize = 0x100_000;
#[cfg(all(
    feature = "unified-dma",
    any(
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    )
))]
pub const VIRTIO_NET_BOUNCE_SIZE: usize = 0x100_000;
/// Combo: bounce sits after the 4 MiB Hal in a 6 MiB driver DMA MR.
#[cfg(all(feature = "unified-dma", feature = "board-x86_64_generic_virtio"))]
pub const VIRTIO_NET_BOUNCE_SIZE: usize = 0x200_000;
#[cfg(not(feature = "unified-dma"))]
#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http",
    feature = "board-x86_64_generic_net"
))]
pub const VIRTIO_NET_CLIENT_DMA_SIZE: usize = 0x200_000;

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_blk"
))]
pub const VIRTIO_BLK_CLIENT_DMA_SIZE: usize = 0x200_000;

pub mod pci {
    use lerux_virtio_hal::BarRegion;
    use virtio_drivers::transport::pci::bus::DeviceFunction;

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk"
    ))]
    pub const BLK_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 3,
        function: 0,
    };

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const NET_DEVICE: DeviceFunction = DeviceFunction {
        bus: 0,
        device: 4,
        function: 0,
    };

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk"
    ))]
    pub const BLK_BAR1_PHYS: u64 = 0xfed0_0000;
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk"
    ))]
    pub const BLK_BAR4_PHYS: u64 = 0xfed1_0000;
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const NET_BAR1_PHYS: u64 = 0xfed2_0000;
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const NET_BAR4_PHYS: u64 = 0xfed3_0000;

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk"
    ))]
    pub const BLK_BAR_PADDRS: &[u64] = &[BLK_BAR1_PHYS, BLK_BAR4_PHYS];
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const NET_BAR_PADDRS: &[u64] = &[NET_BAR1_PHYS, NET_BAR4_PHYS];

    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_blk",
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "PCI BAR addresses below 4 GiB fit in usize"
    )]
    const fn bar_addr(phys: u64) -> usize {
        phys as usize
    }

    #[cfg(feature = "board-x86_64_generic_virtio")]
    pub const BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: bar_addr(BLK_BAR1_PHYS),
            vaddr: bar_addr(BLK_BAR1_PHYS),
            size: 0x1000,
        },
        BarRegion {
            paddr: bar_addr(BLK_BAR4_PHYS),
            vaddr: bar_addr(BLK_BAR4_PHYS),
            size: 0x4000,
        },
        BarRegion {
            paddr: bar_addr(NET_BAR1_PHYS),
            vaddr: bar_addr(NET_BAR1_PHYS),
            size: 0x1000,
        },
        BarRegion {
            paddr: bar_addr(NET_BAR4_PHYS),
            vaddr: bar_addr(NET_BAR4_PHYS),
            size: 0x4000,
        },
    ];

    #[cfg(any(
        feature = "board-x86_64_generic_http",
        feature = "board-x86_64_generic_net"
    ))]
    pub const BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: bar_addr(NET_BAR1_PHYS),
            vaddr: bar_addr(NET_BAR1_PHYS),
            size: 0x1000,
        },
        BarRegion {
            paddr: bar_addr(NET_BAR4_PHYS),
            vaddr: bar_addr(NET_BAR4_PHYS),
            size: 0x4000,
        },
    ];

    #[cfg(feature = "board-x86_64_generic_blk")]
    pub const BAR_REGIONS: &[BarRegion] = &[
        BarRegion {
            paddr: bar_addr(BLK_BAR1_PHYS),
            vaddr: bar_addr(BLK_BAR1_PHYS),
            size: 0x1000,
        },
        BarRegion {
            paddr: bar_addr(BLK_BAR4_PHYS),
            vaddr: bar_addr(BLK_BAR4_PHYS),
            size: 0x4000,
        },
    ];
}
