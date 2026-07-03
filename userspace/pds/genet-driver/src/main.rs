#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_driver_interfaces::net::{GetNetDeviceMeta, MacAddress};
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, Channel, ChannelSet, Handler, Infallible,
    MessageInfo,
};
use sel4_microkit_driver_adapters::net::driver::handle_client_request;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};

mod config;
mod genet;

use config::channels;

use config::GENET_CLIENT_DMA_SIZE;

struct GenetDriver {
    genet: genet::Genet,
    mac: [u8; 6],
    client_region: SharedMemoryRef<'static, [u8]>,
    rx_rings: RingBuffers<'static, Use, fn()>,
    tx_rings: RingBuffers<'static, Use, fn()>,
}

impl GetNetDeviceMeta for GenetDriver {
    type Error = core::convert::Infallible;

    fn get_mac_address(&mut self) -> Result<MacAddress, Self::Error> {
        Ok(MacAddress(self.mac))
    }
}

impl GenetDriver {
    fn new(
        genet: genet::Genet,
        mac: [u8; 6],
        client_region: SharedMemoryRef<'static, [u8]>,
        rx_rings: RingBuffers<'static, Use, fn()>,
        tx_rings: RingBuffers<'static, Use, fn()>,
    ) -> Self {
        Self {
            genet,
            mac,
            client_region,
            rx_rings,
            tx_rings,
        }
    }

    fn pump_tx(&mut self) -> bool {
        let mut notify = false;
        while !self.tx_rings.free_mut().is_empty().unwrap() {
            let desc = self.tx_rings.free_mut().dequeue().unwrap().unwrap();
            let start = desc.encoded_addr();
            let len = usize::try_from(desc.len()).unwrap();
            let mut pkt_buf = [0u8; 1518];
            let pkt = &mut pkt_buf[..len];
            self.client_region
                .as_ptr()
                .index(start..start + len)
                .copy_into_slice(pkt);
            let sent = unsafe { self.genet.transmit(pkt) };
            if sent {
                self.tx_rings
                    .used_mut()
                    .enqueue(desc, true)
                    .unwrap()
                    .unwrap();
                notify = true;
            } else {
                break;
            }
        }
        notify
    }

    fn pump_rx(&mut self) -> bool {
        let mut notify = false;
        while !self.rx_rings.free_mut().is_empty().unwrap() {
            let Some(pkt) = (unsafe { self.genet.receive() }) else {
                break;
            };
            let desc = self.rx_rings.free_mut().dequeue().unwrap().unwrap();
            let desc_len = usize::try_from(desc.len()).unwrap();
            if desc_len >= pkt.len() {
                let start = desc.encoded_addr();
                self.client_region
                    .as_mut_ptr()
                    .index(start..start + pkt.len())
                    .copy_from_slice(pkt);
                self.rx_rings
                    .used_mut()
                    .enqueue(desc, true)
                    .unwrap()
                    .unwrap();
                notify = true;
            }
        }
        notify
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
        if !channels.contains(channels::CLIENT) && !channels.contains(channels::DEVICE) {
            unreachable!()
        }

        let notify_tx = self.dev.pump_tx();
        let notify_rx = self.dev.pump_rx();

        if notify_tx {
            self.dev.tx_rings.notify();
        }
        if notify_rx {
            self.dev.rx_rings.notify();
        }

        if channels.contains(channels::DEVICE) {
            unsafe {
                self.dev.genet.ack_interrupts();
                self.dev.genet.check_tx_completions();
            }
            channels::DEVICE.irq_ack().unwrap();
        }

        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        match channel {
            channels::CLIENT => Ok(handle_client_request(&mut self.dev, msg_info)),
            _ => unreachable!(),
        }
    }
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("genet-driver: starting native RPi4 bcm2711-genet-v5 driver");

    let mmio = memory_region_symbol!(genet_mmio_vaddr: *mut ());
    let driver_dma_v = memory_region_symbol!(virtio_net_driver_dma_vaddr: *mut u8);
    let driver_dma_p = *var!(virtio_net_driver_dma_paddr: usize = 0);

    let notify: fn() = || channels::CLIENT.notify();
    let (rx_rings, tx_rings) = create_rings(notify);
    let client_region = unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_client_dma_vaddr: *mut [u8],
            n = GENET_CLIENT_DMA_SIZE
        ))
    };

    let mut g = unsafe { genet::Genet::new(mmio.as_ptr(), driver_dma_v.as_ptr(), driver_dma_p) };

    unsafe {
        g.reset();
        g.phy_init();
        g.set_mac(&[0xb8, 0x27, 0xeb, 0x12, 0x34, 0x56]);
        g.umac_enable();
        g.setup_rings();
        g.enable_irqs();
        g.ack_interrupts();
    }

    let mac = [0xb8, 0x27, 0xeb, 0x12, 0x34, 0x56];
    log::info!(
        "genet: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5]
    );

    let dev = GenetDriver::new(g, mac, client_region, rx_rings, tx_rings);
    channels::DEVICE.irq_ack().unwrap();
    log::info!("genet-driver: ready");

    HandlerImpl { dev }
}
