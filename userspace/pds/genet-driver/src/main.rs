#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};

mod config;

const CLIENT: Channel = Channel::new(1); // matches net-server wiring in template

/// Minimal genet driver state. Real version would hold ring state, MAC, DMA descriptors.
struct GenetDriver {
    mac: [u8; 6],
}

impl GenetDriver {
    fn new() -> Self {
        // Placeholder MAC (real driver reads from GENET_UMAC_MAC0 + GENET_UMAC_MAC1 after reset).
        Self {
            mac: [0xb8, 0x27, 0xeb, 0x00, 0x00, 0x01],
        }
    }

    #[expect(
        dead_code,
        reason = "MAC read for logging / future use in native driver"
    )]
    fn mac(&self) -> [u8; 6] {
        self.mac
    }

    /// Called when net layer posts TX work via the ring buffers.
    /// Real impl: copy buffer to genet TX desc ring, start DMA, return on IRQ completion.
    fn do_tx(&mut self, len: usize) {
        log::info!("genet: TX (stub) {} bytes to wire", len);
        // In full driver: mark the TX buffer used in ring and notify client.
    }
}

fn create_driver_dma() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_driver_dma_vaddr: *mut [u8],
            n = config::GENET_DRIVER_DMA_SIZE
        ))
    }
}

fn create_client_dma() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_client_dma_vaddr: *mut [u8],
            n = config::GENET_CLIENT_DMA_SIZE
        ))
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
            // Net server notified us (new work in rings or IRQ from genet).
            // Stub: assume TX work and complete it.
            self.dev.do_tx(60); // fake size for smoke "TX ok"
                                // In real driver: poll rings, drive HW, complete used buffers, notify back.
        }
        // Also handle our own IRQ channel (id=0 wired to genet IRQ) if present.
        Ok(())
    }
}

#[protection_domain(heap_size = 256 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("genet-driver: starting native RPi4 bcm2711-genet-v5 driver (Phase 37)");

    // Map the device (real code would reset the block, configure MDIO, program MAC, alloc rings).
    let _mmio = memory_region_symbol!(genet_mmio_vaddr: *mut ());
    let _ = _mmio;
    let _ = create_driver_dma();
    let _ = create_client_dma();
    let _notify: fn() = || CLIENT.notify();
    let (_rx_rings, _tx_rings) = create_rings(_notify);

    let dev = GenetDriver::new();
    log::info!(
        "genet: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} (stub - full HW init TODO)",
        dev.mac[0],
        dev.mac[1],
        dev.mac[2],
        dev.mac[3],
        dev.mac[4],
        dev.mac[5]
    );

    // TODO(37): full genet init (reset, rgmii, umac config, desc rings in the DMA region, enable IRQs).
    // For now the stub allows net-server to talk and get completions for TX smoke tests.

    HandlerImpl { dev }
}
