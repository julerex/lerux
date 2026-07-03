#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo,
};
use sel4_microkit_driver_adapters::block::driver::handle_client_request;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};
use sel4_shared_ring_buffer_block_io_types::{
    BlockIORequest, BlockIORequestStatus, BlockIORequestType,
};

mod config;
mod emmc2;

use config::channels;

fn create_client_region() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_blk_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_BLK_CLIENT_DMA_SIZE
        ))
    }
}

fn create_ring_buffers() -> RingBuffers<'static, Use, fn(), BlockIORequest> {
    let notify_client: fn() = || channels::CLIENT.notify();
    RingBuffers::<'_, Use, fn(), BlockIORequest>::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_used: *mut _)) },
        notify_client,
    )
}

fn buf_ptr_for_req(
    client_region: &mut SharedMemoryRef<'static, [u8]>,
    req: &BlockIORequest,
) -> core::ptr::NonNull<[u8]> {
    let start = req.buf().encoded_addr();
    let len = usize::try_from(req.buf().len()).unwrap();
    client_region
        .as_mut_ptr()
        .index(start..start + len)
        .as_raw_ptr()
}

struct HandlerImpl {
    dev: emmc2::Emmc2,
    client_region: SharedMemoryRef<'static, [u8]>,
    ring_buffers: RingBuffers<'static, Use, fn(), BlockIORequest>,
}

impl HandlerImpl {
    fn process_one(&mut self, req: &BlockIORequest) -> Result<(), ()> {
        let lba = req.start_block_idx();
        let mut buf = [0u8; 512];
        match req.ty().map_err(|_| ())? {
            BlockIORequestType::Read => {
                unsafe {
                    self.dev.read_sector(lba, &mut buf).map_err(|_| ())?;
                }
                let mut out = buf_ptr_for_req(&mut self.client_region, req);
                assert_eq!(out.len(), 512);
                unsafe {
                    out.as_mut().copy_from_slice(&buf);
                }
            }
            BlockIORequestType::Write => {
                let src = buf_ptr_for_req(&mut self.client_region, req);
                assert_eq!(src.len(), 512);
                unsafe {
                    buf.copy_from_slice(src.as_ref());
                    self.dev.write_sector(lba, &buf).map_err(|_| ())?;
                }
            }
        }
        Ok(())
    }

    fn drain_free_ring(&mut self) -> bool {
        let mut notify = false;
        while !self.ring_buffers.free_mut().is_empty().unwrap() {
            let client_req = self.ring_buffers.free_mut().dequeue().unwrap().unwrap();
            let status = if self.process_one(&client_req).is_ok() {
                BlockIORequestStatus::Ok
            } else {
                BlockIORequestStatus::IOError
            };
            let mut completed = client_req;
            completed.set_status(status);
            self.ring_buffers
                .used_mut()
                .enqueue_and_commit(completed)
                .unwrap()
                .unwrap();
            notify = true;
        }
        notify
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if !channels.contains(channels::DEVICE) && !channels.contains(channels::CLIENT) {
            unreachable!()
        }
        if self.drain_free_ring() {
            self.ring_buffers.notify();
        }
        self.dev.ack_interrupt();
        channels::DEVICE.irq_ack().unwrap();
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

#[protection_domain(heap_size = 256 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("emmc2-driver: RPi4 bcm2711-emmc2 native driver");

    let mmio = memory_region_symbol!(emmc2_mmio_vaddr: *mut ());
    let mut dev = unsafe { emmc2::Emmc2::new(mmio.as_ptr()) };
    match unsafe { dev.init() } {
        Ok(()) => log::info!("emmc2: SDHCI init ok"),
        Err(e) => log::info!("emmc2: SDHCI init failed {:?} (smoke may still build)", e),
    }

    let client_region = create_client_region();
    let ring_buffers = create_ring_buffers();
    dev.ack_interrupt();
    channels::DEVICE.irq_ack().unwrap();
    log::info!("emmc2-driver: ready");

    HandlerImpl {
        dev,
        client_region,
        ring_buffers,
    }
}
