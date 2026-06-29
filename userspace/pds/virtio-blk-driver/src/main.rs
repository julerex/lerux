#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::pin::Pin;
use core::ptr::NonNull;

use lerux_logging::{debug, log};
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, Channel, ChannelSet, Handler, Infallible,
    MessageInfo,
};
use sel4_microkit_driver_adapters::block::driver::handle_client_request;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};
use sel4_shared_ring_buffer_block_io_types::{
    BlockIORequest, BlockIORequestStatus, BlockIORequestType,
};
use sel4_virtio_blk::GetBlockDeviceLayoutWrapper;
use sel4_virtio_hal_impl::HalImpl;
use virtio_drivers::{
    device::blk::*,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

mod config;

use config::channels;

// Hard-coded in virtio-drivers.
const QUEUE_SIZE: usize = 4;

fn init_hal() {
    debug::init().unwrap();
    HalImpl::init(
        config::VIRTIO_BLK_DRIVER_DMA_SIZE,
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
    );
}

fn create_virtio_blk() -> VirtIOBlk<HalImpl, MmioTransport<'static>> {
    let header = NonNull::new(
        (*var!(virtio_blk_mmio_vaddr: usize = 0) + config::VIRTIO_BLK_MMIO_OFFSET)
            as *mut VirtIOHeader,
    )
    .unwrap();
    let transport =
        unsafe { MmioTransport::new(header, config::VIRTIO_BLK_MMIO_SIZE) }.unwrap();
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, MmioTransport>::new(transport).unwrap()
}

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

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    init_hal();
    let mut dev = create_virtio_blk();
    let client_region = create_client_region();
    let ring_buffers = create_ring_buffers();
    dev.ack_interrupt();
    channels::DEVICE.irq_ack().unwrap();
    log::info!("virtio-blk driver ready");
    HandlerImpl {
        dev,
        client_region,
        ring_buffers,
        pending: BTreeMap::new(),
    }
}

struct HandlerImpl {
    dev: VirtIOBlk<HalImpl, MmioTransport<'static>>,
    client_region: SharedMemoryRef<'static, [u8]>,
    ring_buffers: RingBuffers<'static, Use, fn(), BlockIORequest>,
    pending: BTreeMap<u16, Pin<Box<PendingEntry>>>,
}

struct PendingEntry {
    client_req: BlockIORequest,
    virtio_req: BlkReq,
    virtio_resp: BlkResp,
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

impl HandlerImpl {

    fn complete_virtio_read(
        &mut self,
        token: u16,
        pending_entry: &mut PendingEntry,
        buf_ptr: &mut [u8],
    ) {
        unsafe {
            self.dev
                .complete_read_blocks(
                    token,
                    &pending_entry.virtio_req,
                    buf_ptr,
                    &mut pending_entry.virtio_resp,
                )
                .unwrap();
        }
    }

    fn enqueue_completed_request(
        &mut self,
        client_req: BlockIORequest,
        virtio_resp: &BlkResp,
    ) {
        let status = match virtio_resp.status() {
            RespStatus::OK => BlockIORequestStatus::Ok,
            _ => panic!(),
        };
        let mut completed_req = client_req;
        completed_req.set_status(status);
        self.ring_buffers
            .used_mut()
            .enqueue_and_commit(completed_req)
            .unwrap()
            .unwrap();
    }

    fn complete_one_used_request(&mut self) {
        let token = self.dev.peek_used().unwrap();
        let mut pending_entry = self.pending.remove(&token).unwrap();
        unsafe {
            let mut buf_ptr =
                buf_ptr_for_req(&mut self.client_region, &pending_entry.client_req);
            let pending_entry = &mut *pending_entry;
            self.complete_virtio_read(token, pending_entry, buf_ptr.as_mut());
        }
        self.enqueue_completed_request(
            pending_entry.client_req,
            &pending_entry.virtio_resp,
        );
    }

    fn complete_used_requests(&mut self) -> bool {
        let mut notify = false;
        while self.dev.peek_used().is_some() {
            self.complete_one_used_request();
            notify = true;
        }
        notify
    }

    fn submit_read_request(
        &mut self,
        pending_entry: &mut PendingEntry,
        buf_ptr: &mut [u8],
    ) -> u16 {
        unsafe {
            self.dev
                .read_blocks_nb(
                    pending_entry
                        .client_req
                        .start_block_idx()
                        .try_into()
                        .unwrap(),
                    &mut pending_entry.virtio_req,
                    buf_ptr,
                    &mut pending_entry.virtio_resp,
                )
                .unwrap()
        }
    }

    fn issue_one_pending_request(&mut self) -> bool {
        let client_req = self.ring_buffers.free_mut().dequeue().unwrap().unwrap();
        assert_eq!(client_req.ty().unwrap(), BlockIORequestType::Read);
        let mut pending_entry = Box::pin(PendingEntry {
            client_req,
            virtio_req: BlkReq::default(),
            virtio_resp: BlkResp::default(),
        });
        let mut buf_ptr =
            buf_ptr_for_req(&mut self.client_region, &pending_entry.client_req);
        assert_eq!(buf_ptr.len(), 512);
        let token = unsafe {
            let pending_entry = &mut *pending_entry;
            self.submit_read_request(pending_entry, buf_ptr.as_mut())
        };
        assert!(self.pending.insert(token, pending_entry).is_none());
        true
    }

    fn issue_pending_requests(&mut self) -> bool {
        let mut notify = false;
        while self.pending.len() < QUEUE_SIZE
            && !self.ring_buffers.free_mut().is_empty().unwrap()
        {
            notify |= self.issue_one_pending_request();
        }
        notify
    }

    fn ack_device_irq(&mut self) {
        self.dev.ack_interrupt();
        channels::DEVICE.irq_ack().unwrap();
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if !channels.contains(channels::DEVICE) && !channels.contains(channels::CLIENT) {
            unreachable!()
        }
        let notify = self.complete_used_requests() | self.issue_pending_requests();
        if notify {
            self.ring_buffers.notify();
        }
        self.ack_device_irq();
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        match channel {
            channels::CLIENT => Ok(handle_client_request(
                &mut GetBlockDeviceLayoutWrapper(&self.dev),
                msg_info,
            )),
            _ => unreachable!(),
        }
    }
}