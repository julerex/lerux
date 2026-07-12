/// Separate client_dma MR size (legacy / non-unified boards).
#[cfg(not(feature = "unified-dma"))]
pub const VIRTIO_NET_CLIENT_DMA_SIZE: usize = 0x200_000;
/// With `unified-dma`, Hal owns the low half of driver_dma; bounce is the high half.
#[cfg(feature = "unified-dma")]
pub const VIRTIO_NET_HAL_SIZE: usize = 0x100_000;
#[cfg(feature = "unified-dma")]
pub const VIRTIO_NET_BOUNCE_SIZE: usize = 0x100_000;
pub const NET_QUEUE_SIZE: usize = 16;
pub const NET_BUFFER_LEN: usize = 2048;
