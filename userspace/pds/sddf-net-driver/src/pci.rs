use lerux_machine::{virtio, Arch, VirtioAttach, VirtioRole};
use lerux_virtio_hal::{BarRegion, HalImpl};
use lerux_virtio_pci::create_pci_transport_ioport;
use sel4_microkit::var;
use virtio_drivers::{
    device::net::VirtIONet,
    transport::{
        pci::{bus::DeviceFunction, PciTransport},
        DeviceType, Transport,
    },
};

use lerux_sddf::net_image::NET_DRIVER_DMA_SIZE;

use crate::mmio::{BUFFER_LEN, QUEUE_SIZE};

pub type NetDev = VirtIONet<HalImpl, PciTransport, QUEUE_SIZE>;

const BAR_SIZES: [usize; 2] = [0x1_000, 0x4_000];

const fn net_pci() -> VirtioAttach {
    match virtio(Arch::TARGET, VirtioRole::Net) {
        Some(attach @ VirtioAttach::Pci { .. }) => attach,
        _ => panic!("x86 network image maps virtio-net PCI"),
    }
}

const fn bar_paddrs() -> &'static [u64] {
    let VirtioAttach::Pci { bars, .. } = net_pci() else {
        panic!("x86 network image maps virtio-net PCI");
    };
    bars
}

const fn bar_paddr(index: usize) -> usize {
    bar_paddrs()[index] as usize
}

const fn bar_regions() -> &'static [BarRegion] {
    static REGIONS: [BarRegion; 2] = [
        BarRegion {
            paddr: bar_paddr(0),
            vaddr: bar_paddr(0),
            size: BAR_SIZES[0],
        },
        BarRegion {
            paddr: bar_paddr(1),
            vaddr: bar_paddr(1),
            size: BAR_SIZES[1],
        },
    ];
    &REGIONS
}

fn device_function() -> DeviceFunction {
    let VirtioAttach::Pci {
        device, function, ..
    } = net_pci()
    else {
        panic!("x86 network image maps virtio-net PCI");
    };
    DeviceFunction {
        bus: 0,
        device,
        function,
    }
}

pub fn init_hal() {
    HalImpl::init(
        usize::try_from(NET_DRIVER_DMA_SIZE).expect("dma size fits"),
        *var!(virtio_net_driver_dma_vaddr: usize = 0),
        *var!(virtio_net_driver_dma_paddr: usize = 0),
        bar_regions(),
    );
}

pub fn create_virtio_net() -> NetDev {
    let ioport_id = *var!(pci_net_config_ioport_id: usize = 0) as u32;
    let ioport_addr = *var!(pci_net_config_ioport_addr: usize = 0) as u16;
    let transport =
        create_pci_transport_ioport(ioport_id, ioport_addr, device_function(), bar_paddrs())
            .expect("virtio-net PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Network);
    VirtIONet::<HalImpl, PciTransport, QUEUE_SIZE>::new(transport, BUFFER_LEN)
        .expect("virtio-net device")
}
