#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_machine::{ahci_map, READY_LINE};
use lerux_sddf::{
    blk_dequeue_req, blk_driver_config_t, blk_enqueue_resp,
    blk_image::{self, BLK_DATA_SIZE, BLK_DATA_VADDR, BLK_DRIVER_IRQ_CHANNEL, BLK_QUEUE_CAPACITY},
    blk_queue_handle_t, blk_queue_init, blk_req_code_t, blk_resp_status_t, blk_storage_info_t,
    blk_storage_set_ready, BLK_TRANSFER_SIZE, SDDF_BLK_MAGIC,
};

mod engine;

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const SECTORS_PER_BLOCK: u64 = 8;

struct HandlerImpl {
    engine: engine::Engine,
    queues: blk_queue_handle_t,
    irq: Channel,
    virt: Channel,
    capacity_blocks: u64,
}

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `blk_driver_config_t`.
    let config = unsafe { blk_image::blk_config_from_bytes::<blk_driver_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_BLK_MAGIC);
    let engine = engine::Engine::open(ahci_map()).expect("ahci engine");
    let capacity_blocks = engine.capacity_blocks();
    // SAFETY: this protection domain maps the storage-info page at the config address.
    unsafe { publish_storage(config.virt.storage_info.vaddr.cast(), capacity_blocks) };
    engine.ack();
    let irq = Channel::new(usize::from(BLK_DRIVER_IRQ_CHANNEL));
    irq.irq_ack().expect("irq ack");
    let mut queues = blk_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps the driver request and response queues here.
    unsafe {
        blk_queue_init(
            &mut queues,
            config.virt.req_queue.vaddr.cast(),
            config.virt.resp_queue.vaddr.cast(),
            u32::from(config.virt.num_buffers),
        );
    }
    log::info!("{}", READY_LINE);
    HandlerImpl {
        engine,
        queues,
        irq,
        virt: Channel::new(usize::from(config.virt.id)),
        capacity_blocks,
    }
}

/// # Safety
///
/// `storage` must point at the mapped `blk_storage_info_t`.
unsafe fn publish_storage(storage: *mut blk_storage_info_t, capacity_blocks: u64) {
    let serial = b"lerux-ahci";
    unsafe {
        core::ptr::write_bytes(storage, 0, 1);
        core::ptr::copy_nonoverlapping(
            serial.as_ptr(),
            (*storage).serial_number.as_mut_ptr(),
            serial.len(),
        );
        (*storage).read_only = false;
        (*storage).sector_size = 512;
        (*storage).block_size = 1;
        (*storage).queue_depth = BLK_QUEUE_CAPACITY;
        (*storage).capacity = capacity_blocks;
        blk_storage_set_ready(storage, true);
    }
}

impl HandlerImpl {
    fn serve(&mut self) {
        loop {
            let mut code = blk_req_code_t::BLK_REQ_READ;
            let mut offset = 0u64;
            let mut block = 0u64;
            let mut count = 0u16;
            let mut id = 0u32;
            // SAFETY: this protection domain is the consumer of the driver request queue.
            if unsafe {
                blk_dequeue_req(
                    &self.queues,
                    &mut code,
                    &mut offset,
                    &mut block,
                    &mut count,
                    &mut id,
                )
            } != 0
            {
                break;
            }
            let (status, success_count) = self.finish(code, offset, block, count);
            // SAFETY: this protection domain is the producer of the driver response queue.
            if unsafe { blk_enqueue_resp(&self.queues, status, success_count, id) } != 0 {
                log::info!("ahci_driver: response queue full");
                break;
            }
            self.virt.notify();
        }
    }

    fn finish(
        &mut self,
        code: blk_req_code_t,
        offset: u64,
        block: u64,
        count: u16,
    ) -> (blk_resp_status_t, u16) {
        match code {
            blk_req_code_t::BLK_REQ_READ => self.transfer(true, offset, block, count),
            blk_req_code_t::BLK_REQ_WRITE => self.transfer(false, offset, block, count),
            blk_req_code_t::BLK_REQ_FLUSH | blk_req_code_t::BLK_REQ_BARRIER => {
                (blk_resp_status_t::BLK_RESP_OK, 0)
            }
        }
    }

    fn transfer(
        &mut self,
        read: bool,
        offset: u64,
        block: u64,
        count: u16,
    ) -> (blk_resp_status_t, u16) {
        let invalid = (blk_resp_status_t::BLK_RESP_ERR_INVALID_PARAM, 0);
        if count == 0 {
            return invalid;
        }
        let Ok(data_len) = usize::try_from(BLK_DATA_SIZE) else {
            return invalid;
        };
        let len = usize::from(count).saturating_mul(BLK_TRANSFER_SIZE as usize);
        let Ok(offset_usize) = usize::try_from(offset) else {
            return invalid;
        };
        if offset_usize
            .checked_add(len)
            .is_none_or(|end| end > data_len)
        {
            return invalid;
        }
        let Some(end_block) = block.checked_add(u64::from(count)) else {
            return invalid;
        };
        if end_block > self.capacity_blocks {
            return invalid;
        }
        let Some(sector) = block.checked_mul(SECTORS_PER_BLOCK) else {
            return invalid;
        };
        let Ok(base) = usize::try_from(BLK_DATA_VADDR) else {
            return invalid;
        };
        let ptr = (base + offset_usize) as *mut u8;
        match self.engine.transfer(read, ptr, len, sector) {
            Ok(()) => (blk_resp_status_t::BLK_RESP_OK, count),
            Err(()) => (blk_resp_status_t::BLK_RESP_ERR_IO, 0),
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.irq) {
            self.engine.ack();
            self.irq.irq_ack().expect("irq ack");
        }
        if channels.contains(self.virt) {
            self.serve();
        }
        Ok(())
    }
}
