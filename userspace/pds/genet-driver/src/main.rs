#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, Channel, ChannelSet, Handler, Infallible,
};
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};

mod config;
mod genet;

const CLIENT: Channel = Channel::new(1);

struct GenetDriver {
    genet: genet::Genet,
    #[allow(dead_code)]
    mac: [u8; 6],
    #[allow(dead_code)]
    rx_rings: RingBuffers<'static, Use, fn()>,
    #[allow(dead_code)]
    tx_rings: RingBuffers<'static, Use, fn()>,
}

impl GenetDriver {
    fn new(
        genet: genet::Genet,
        mac: [u8; 6],
        rx_rings: RingBuffers<'static, Use, fn()>,
        tx_rings: RingBuffers<'static, Use, fn()>,
    ) -> Self {
        Self {
            genet,
            mac,
            rx_rings,
            tx_rings,
        }
    }

    fn process_tx(&mut self) {
        // Simple stub: exercise the transmit path when net-server notifies us.
        // A real implementation would drain self.tx_rings.take() / .used etc.
        let dummy_pkt = [0u8; 64];
        unsafe {
            let _ = self.genet.transmit(&dummy_pkt);
        }
        log::info!("genet: processed TX notification (stub ring pump)");
    }

    unsafe fn handle_hw_irq(&mut self) {
        unsafe {
            self.genet.ack_interrupts();
            self.genet.check_tx_completions();
        }

        // Placeholder for RX: real code would walk RX descriptors here,
        // deliver packets into rx_rings, then re-arm.
        log::info!("genet: HW IRQ serviced");
    }
}

#[allow(clippy::type_complexity)]
fn create_rings(
    notify: fn(),
) -> (
    RingBuffers<'static, Use, fn()>,
    RingBuffers<'static, Use, fn()>,
) {
    let rx = RingBuffers::<'_, Use, fn()>::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_used: *mut _)) },
        notify,
    );
    let tx = RingBuffers::<'_, Use, fn()>::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_used: *mut _)) },
        notify,
    );
    (rx, tx)
}

struct HandlerImpl {
    dev: GenetDriver,
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(CLIENT) {
            self.dev.process_tx();
        }

        // The GENET IRQ is wired as id=0 in the .system template.
        if !channels.contains(CLIENT) {
            unsafe {
                self.dev.handle_hw_irq();
            }
        }
        Ok(())
    }
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("genet-driver: starting native RPi4 bcm2711-genet-v5 driver (Phase 37)");

    // Obtain regions mapped by Microkit / the system template.
    let mmio = memory_region_symbol!(genet_mmio_vaddr: *mut ());
    let driver_dma_v = memory_region_symbol!(virtio_net_driver_dma_vaddr: *mut u8);
    let driver_dma_p = *var!(virtio_net_driver_dma_paddr: usize = 0);

    let _notify: fn() = || CLIENT.notify();
    let (rx_rings, tx_rings) = create_rings(_notify);

    // Construct the GENET driver.
    let mut g = unsafe { genet::Genet::new(mmio.as_ptr(), driver_dma_v.as_ptr(), driver_dma_p) };

    // === The requested full initialization ===
    unsafe {
        g.reset();
        g.phy_init();          // MDIO + RGMII PHY bringup
        g.set_mac(&[0xb8, 0x27, 0xeb, 0x12, 0x34, 0x56]);
        g.umac_enable();
        g.setup_rings();       // TX/RX descriptor rings inside the driver DMA region
        g.enable_irqs();
    }

    let mac = [0xb8, 0x27, 0xeb, 0x12, 0x34, 0x56];
    log::info!(
        "genet: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} (full native init)",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );

    let mut dev = GenetDriver::new(g, mac, rx_rings, tx_rings);

    unsafe {
        dev.genet.ack_interrupts();
    }

    HandlerImpl { dev }
}