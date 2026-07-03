use sel4_microkit::Channel;

#[expect(dead_code, reason = "channel id used in full ring implementation")]
pub const NET_DRIVER: Channel = Channel::new(0); // to net-server

// Sizes match the ones in virtio templates / other net drivers.
#[allow(dead_code)]
pub const GENET_DRIVER_DMA_SIZE: usize = 0x200_000;
#[allow(dead_code)]
pub const GENET_CLIENT_DMA_SIZE: usize = 0x200_000;
