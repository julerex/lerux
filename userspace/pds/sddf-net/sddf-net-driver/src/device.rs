//! Virtio-net device for the board transport (MMIO or PCI).

use virtio_drivers::device::net::RxBuffer;

#[cfg(not(target_arch = "x86_64"))]
use crate::mmio;

#[cfg(target_arch = "x86_64")]
use crate::pci;

#[cfg(not(target_arch = "x86_64"))]
pub enum NetDev {
    Mmio(mmio::NetDev),
}

#[cfg(target_arch = "x86_64")]
pub enum NetDev {
    Pci(pci::NetDev),
}

pub fn init_hal() {
    #[cfg(not(target_arch = "x86_64"))]
    mmio::init_hal();
    #[cfg(target_arch = "x86_64")]
    pci::init_hal();
}

pub fn create_virtio_net() -> NetDev {
    #[cfg(not(target_arch = "x86_64"))]
    {
        NetDev::Mmio(mmio::create_virtio_net())
    }
    #[cfg(target_arch = "x86_64")]
    {
        NetDev::Pci(pci::create_virtio_net())
    }
}

impl NetDev {
    pub fn mac_address(&self) -> [u8; 6] {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.mac_address(),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.mac_address(),
        }
    }

    pub fn ack_interrupt(&mut self) {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => {
                let _ = dev.ack_interrupt();
            }
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => {
                let _ = dev.ack_interrupt();
            }
        }
    }

    pub fn can_send(&self) -> bool {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.can_send(),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.can_send(),
        }
    }

    pub fn can_recv(&self) -> bool {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.can_recv(),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.can_recv(),
        }
    }

    pub fn new_tx_buffer(&mut self, len: usize) -> virtio_drivers::device::net::TxBuffer {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.new_tx_buffer(len),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.new_tx_buffer(len),
        }
    }

    pub fn send(
        &mut self,
        tx: virtio_drivers::device::net::TxBuffer,
    ) -> Result<(), virtio_drivers::Error> {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.send(tx),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.send(tx),
        }
    }

    pub fn receive(&mut self) -> Result<RxBuffer, virtio_drivers::Error> {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.receive(),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.receive(),
        }
    }

    pub fn recycle_rx_buffer(&mut self, rx: RxBuffer) -> Result<(), virtio_drivers::Error> {
        match self {
            #[cfg(not(target_arch = "x86_64"))]
            NetDev::Mmio(dev) => dev.recycle_rx_buffer(rx),
            #[cfg(target_arch = "x86_64")]
            NetDev::Pci(dev) => dev.recycle_rx_buffer(rx),
        }
    }
}
