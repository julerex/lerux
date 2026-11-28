//! Virtio-blk device for the board transport (MMIO or PCI).

#[cfg(not(target_arch = "x86_64"))]
use crate::mmio;

#[cfg(target_arch = "x86_64")]
use crate::pci;

#[cfg(not(target_arch = "x86_64"))]
pub enum BlkDev {
    Mmio(mmio::BlkDev),
}

#[cfg(target_arch = "x86_64")]
pub enum BlkDev {
    Pci(pci::BlkDev),
}

pub fn init_hal() {
    #[cfg(not(target_arch = "x86_64"))]
    mmio::init_hal();
    #[cfg(target_arch = "x86_64")]
    pci::init_hal();
}

pub fn create_virtio_blk() -> BlkDev {
    #[cfg(not(target_arch = "x86_64"))]
    {
        BlkDev::Mmio(mmio::create_virtio_blk())
    }
    #[cfg(target_arch = "x86_64")]
    {
        BlkDev::Pci(pci::create_virtio_blk())
    }
}

impl BlkDev {
    pub fn capacity(&self) -> u64 {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            BlkDev::Mmio(dev) => dev.capacity(),
            #[cfg(target_arch = "x86_64")]
            BlkDev::Pci(dev) => dev.capacity(),
        }
    }

    pub fn ack_interrupt(&mut self) {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            BlkDev::Mmio(dev) => {
                let _ = dev.ack_interrupt();
            }
            #[cfg(target_arch = "x86_64")]
            BlkDev::Pci(dev) => {
                let _ = dev.ack_interrupt();
            }
        }
    }

    pub fn read_blocks(
        &mut self,
        sector: usize,
        buf: &mut [u8],
    ) -> Result<(), virtio_drivers::Error> {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            BlkDev::Mmio(dev) => dev.read_blocks(sector, buf),
            #[cfg(target_arch = "x86_64")]
            BlkDev::Pci(dev) => dev.read_blocks(sector, buf),
        }
    }

    pub fn write_blocks(&mut self, sector: usize, buf: &[u8]) -> Result<(), virtio_drivers::Error> {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            BlkDev::Mmio(dev) => dev.write_blocks(sector, buf),
            #[cfg(target_arch = "x86_64")]
            BlkDev::Pci(dev) => dev.write_blocks(sector, buf),
        }
    }
}
