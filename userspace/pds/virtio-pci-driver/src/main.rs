#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::pin::Pin;

use lerux_logging::{debug, log};
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
    MessageInfo,
};
use sel4_microkit_driver_adapters::{
    block::driver::handle_client_request,
    net::driver::HandlerImpl as NetHandlerImpl,
};
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};
use sel4_shared_ring_buffer_block_io_types::{
    BlockIORequest, BlockIORequestStatus, BlockIORequestType,
};
use sel4_virtio_blk::GetBlockDeviceLayoutWrapper;
use sel4_virtio_net::DeviceWrapper;
use virtio_drivers::{
    device::blk::*,
    transport::pci::PciTransport,
};

mod config;
mod pci;

use config::channels;
use lerux_virtio_hal::HalImpl;

const BLK_QUEUE_SIZE: usize = 4;

#[protection_domain(heap_size = 768 * 1024)]
fn init() -> ComboHandler {
    debug::init().unwrap();
    pci::init_hal();

    let (ioport_id, ioport_addr) = pci::ioport_config();
    let mut blk_dev = pci::create_virtio_blk(ioport_id, ioport_addr);
    let mut net_dev = pci::create_virtio_net(ioport_id, ioport_addr);

    let blk_client_region = unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_blk_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_BLK_CLIENT_DMA_SIZE
        ))
    };
    let notify_blk_client: fn() = || channels::BLK_CLIENT.notify();
    let blk_ring_buffers =
        RingBuffers::<'_, Use, fn(), BlockIORequest>::from_ptrs_using_default_initialization_strategy_for_role(
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_free: *mut _)) },
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_blk_used: *mut _)) },
            notify_blk_client,
        );

    let net_client_region = unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_NET_CLIENT_DMA_SIZE
        ))
    };
    let notify_net_client: fn() = || channels::NET_CLIENT.notify();
    let net_rx_ring_buffers =
        RingBuffers::<'_, Use, fn()>::from_ptrs_using_default_initialization_strategy_for_role(
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_free: *mut _)) },
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_used: *mut _)) },
            notify_net_client,
        );
    let net_tx_ring_buffers =
        RingBuffers::<'_, Use, fn()>::from_ptrs_using_default_initialization_strategy_for_role(
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_free: *mut _)) },
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_used: *mut _)) },
            notify_net_client,
        );

    blk_dev.ack_interrupt();
    channels::BLK_DEVICE.irq_ack().unwrap();
    log::info!("virtio-blk driver ready");

    net_dev.ack_interrupt();
    channels::NET_DEVICE.irq_ack().unwrap();
    log::info!("virtio-net driver ready");

    ComboHandler {
        blk: BlkHandler {
            dev: blk_dev,
            client_region: blk_client_region,
            ring_buffers: blk_ring_buffers,
            pending: BTreeMap::new(),
        },
        net: NetHandlerImpl::new(
            DeviceWrapper::new(net_dev),
            net_client_region,
            net_rx_ring_buffers,
            net_tx_ring_buffers,
            channels::NET_DEVICE,
            channels::NET_CLIENT,
        ),
    }
}

struct ComboHandler {
    blk: BlkHandler,
    net: NetHandlerImpl<DeviceWrapper<HalImpl, PciTransport>>,
}

struct BlkHandler {
    dev: VirtIOBlk<HalImpl, PciTransport>,
    client_region: SharedMemoryRef<'static, [u8]>,
    ring_buffers: RingBuffers<'static, Use, fn(), BlockIORequest>,
    pending: BTreeMap<u16, Pin<Box<BlkPendingEntry>>>,
}

struct BlkPendingEntry {
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

impl BlkHandler {
    fn complete_virtio_read(
        &mut self,
        token: u16,
        pending_entry: &mut BlkPendingEntry,
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
        pending_entry: &mut BlkPendingEntry,
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
        let mut pending_entry = Box::pin(BlkPendingEntry {
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
        while self.pending.len() < BLK_QUEUE_SIZE
            && !self.ring_buffers.free_mut().is_empty().unwrap()
        {
            notify |= self.issue_one_pending_request();
        }
        notify
    }

    fn ack_device_irq(&mut self) {
        self.dev.ack_interrupt();
        channels::BLK_DEVICE.irq_ack().unwrap();
    }

    fn notified(&mut self, channels: ChannelSet) -> bool {
        if !channels.contains(channels::BLK_DEVICE) && !channels.contains(channels::BLK_CLIENT) {
            return false;
        }
        let notify = self.complete_used_requests() | self.issue_pending_requests();
        if notify {
            self.ring_buffers.notify();
        }
        if channels.contains(channels::BLK_DEVICE) {
            self.ack_device_irq();
        }
        true
    }

    fn protected(&mut self, channel: Channel, msg_info: MessageInfo) -> MessageInfo {
        assert_eq!(channel, channels::BLK_CLIENT);
        handle_client_request(
            &mut GetBlockDeviceLayoutWrapper(&self.dev),
            msg_info,
        )
    }
}

impl Handler for ComboHandler {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(channels::BLK_DEVICE) || channels.contains(channels::BLK_CLIENT) {
            self.blk.notified(channels);
        }
        if channels.contains(channels::NET_DEVICE) || channels.contains(channels::NET_CLIENT) {
            self.net.notified(channels)?;
        }
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel == channels::BLK_CLIENT {
            Ok(self.blk.protected(channel, msg_info))
        } else if channel == channels::NET_CLIENT {
            self.net.protected(channel, msg_info)
        } else {
            unreachable!()
        }
    }
}