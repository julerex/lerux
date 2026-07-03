// These match the ring/DMA region sizes used in the blk-emmc-rpi system template.
// They are referenced via memory_region_symbol! in a full driver impl.
#[expect(
    dead_code,
    reason = "sizes for memory_region_symbol in full driver implementation"
)]
pub const EMMC2_DRIVER_DMA_SIZE: usize = 0x200_000;
#[expect(
    dead_code,
    reason = "sizes for memory_region_symbol in full driver implementation"
)]
pub const EMMC2_CLIENT_DMA_SIZE: usize = 0x200_000;
