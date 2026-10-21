use lerux_machine::{virtio, Arch, VirtioAttach, VirtioRole};
use lerux_virtio_hal::{BarRegion, HalImpl};
use lerux_virtio_pci::{create_pci_transport_ecam, create_pci_transport_ioport};
use sel4_microkit::var;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        pci::{bus::DeviceFunction, PciTransport},
        DeviceType, Transport,
    },
};

use lerux_sddf::blk_image::BLK_VIRTIO_DMA_SIZE;

pub type BlkDev = VirtIOBlk<HalImpl, PciTransport>;

const BAR_SIZES: [usize; 2] = [0x1_000, 0x4_000];

const fn blk_pci() -> VirtioAttach {
    match virtio(Arch::TARGET, VirtioRole::Blk) {
        Some(attach @ VirtioAttach::Pci { .. }) => attach,
        _ => panic!("x86 filesystem image maps virtio-blk PCI"),
    }
}

const fn bar_paddrs() -> &'static [u64] {
    let VirtioAttach::Pci { bars, .. } = blk_pci() else {
        panic!("x86 filesystem image maps virtio-blk PCI");
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
    } = blk_pci()
    else {
        panic!("x86 filesystem image maps virtio-blk PCI");
    };
    DeviceFunction {
        bus: 0,
        device,
        function,
    }
}

pub fn init_hal() {
    HalImpl::init(
        usize::try_from(BLK_VIRTIO_DMA_SIZE).expect("dma size fits"),
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
        bar_regions(),
    );
}

#[cfg(any(
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "board-x86_64_generic_web_sddf",
    feature = "board-x86_64_generic_agent_sddf",
    feature = "board-x86_64_generic_program_sddf"
))]
pub fn create_virtio_blk() -> BlkDev {
    let transport = create_pci_transport_ecam(
        *var!(pci_ecam_vaddr: usize = 0),
        device_function(),
        bar_paddrs(),
    )
    .expect("virtio-blk PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, PciTransport>::new(transport).expect("virtio-blk device")
}

#[cfg(not(any(
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "board-x86_64_generic_web_sddf",
    feature = "board-x86_64_generic_agent_sddf",
    feature = "board-x86_64_generic_program_sddf"
)))]
pub fn create_virtio_blk() -> BlkDev {
    let ioport_id = *var!(pci_config_ioport_id: usize = 0) as u32;
    let ioport_addr = *var!(pci_config_ioport_addr: usize = 0) as u16;
    let transport =
        create_pci_transport_ioport(ioport_id, ioport_addr, device_function(), bar_paddrs())
            .expect("virtio-blk PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, PciTransport>::new(transport).expect("virtio-blk device")
}
