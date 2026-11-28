pub mod channels {
    use sel4_microkit::Channel;

    pub const DEVICE: Channel = Channel::new(0);
    pub const CLIENT: Channel = Channel::new(1);
}

pub const VIRTIO_BLK_CLIENT_DMA_SIZE: usize = 0x200_000;
