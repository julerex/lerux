pub mod channels {
    use sel4_microkit::Channel;

    pub const DEVICE: Channel = Channel::new(0);
    pub const CLIENT: Channel = Channel::new(1);
}

#[expect(
    dead_code,
    reason = "DMA region size for future memory_region_symbol use"
)]
pub const GENET_DRIVER_DMA_SIZE: usize = 0x200_000;
pub const GENET_CLIENT_DMA_SIZE: usize = 0x200_000;
