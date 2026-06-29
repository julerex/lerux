pub mod channels {
    use sel4_microkit::Channel;

    pub const DEVICE: Channel = Channel::new(0);
    pub const CLIENT: Channel = Channel::new(1);
}

#[cfg(feature = "board-qemu_virt_riscv64_virtio")]
pub const VIRTIO_NET_MMIO_OFFSET: usize = 0;

#[cfg(not(feature = "board-qemu_virt_riscv64_virtio"))]
pub const VIRTIO_NET_MMIO_OFFSET: usize = 0xe00;

pub const VIRTIO_NET_MMIO_SIZE: usize = 0x200;
pub const VIRTIO_NET_DRIVER_DMA_SIZE: usize = 0x200_000;
pub const VIRTIO_NET_CLIENT_DMA_SIZE: usize = 0x200_000;