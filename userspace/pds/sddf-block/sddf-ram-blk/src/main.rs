//! RAM block driver for the Swift file commands.
//!
//! The volume is ordinary RAM. `sddf-fatfs` formats it when the boot sector is
//! blank. This protection domain does not map the laptop disk, so that format
//! does not write it.

#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_sddf::{
    blk_dequeue_req, blk_driver_config_t, blk_enqueue_resp,
    blk_image::{self, BLK_DATA_SIZE, BLK_DATA_VADDR, BLK_DRIVER_VIRT_CHANNEL, BLK_QUEUE_CAPACITY},
    blk_queue_handle_t, blk_queue_init, blk_req_code_t, blk_resp_status_t, blk_storage_info_t,
    blk_storage_set_ready, BLK_TRANSFER_SIZE, SDDF_BLK_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

/// Matches `ram_disk` in `console-swift.system.template`.
const DISK_VADDR: u64 = 0x4_040_000;
/// 8192 sectors of 512 bytes. One transfer block is 4096 bytes, so the
/// capacity the filesystem server formats is 1024 blocks.
const DISK_BLOCKS: u64 = 1024;
const SECTOR_SIZE: u16 = 512;

/// Block virtualiser. The literal matches `BLK_DRIVER_VIRT_CHANNEL`.
const BLK_VIRT: Channel = Channel::new(1);
const _: () = assert!(BLK_DRIVER_VIRT_CHANNEL == 1);

struct HandlerImpl {
    queues: blk_queue_handle_t,
    virt: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    // SAFETY: the build script wrote a `blk_driver_config_t`.
    let config = unsafe { blk_image::blk_config_from_bytes::<blk_driver_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_BLK_MAGIC);
    assert_eq!(config.virt.id, BLK_DRIVER_VIRT_CHANNEL);
    // SAFETY: this protection domain maps the storage-info page at the config address.
    unsafe { publish_storage(config.virt.storage_info.vaddr.cast()) };
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
    HandlerImpl {
        queues,
        virt: BLK_VIRT,
    }
}

/// # Safety
///
/// `storage` must point at the mapped `blk_storage_info_t`.
unsafe fn publish_storage(storage: *mut blk_storage_info_t) {
    let serial = b"lerux-ram";
    unsafe {
        core::ptr::write_bytes(storage, 0, 1);
        core::ptr::copy_nonoverlapping(
            serial.as_ptr(),
            (*storage).serial_number.as_mut_ptr(),
            serial.len(),
        );
        (*storage).read_only = false;
        (*storage).sector_size = SECTOR_SIZE;
        (*storage).block_size = 1;
        (*storage).queue_depth = BLK_QUEUE_CAPACITY;
        (*storage).capacity = DISK_BLOCKS;
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
            let (status, success_count) = finish(code, offset, block, count);
            // SAFETY: this protection domain is the producer of the driver response queue.
            if unsafe { blk_enqueue_resp(&self.queues, status, success_count, id) } != 0 {
                break;
            }
            self.virt.notify();
        }
    }
}

fn finish(code: blk_req_code_t, offset: u64, block: u64, count: u16) -> (blk_resp_status_t, u16) {
    match code {
        blk_req_code_t::BLK_REQ_READ => transfer(true, offset, block, count),
        blk_req_code_t::BLK_REQ_WRITE => transfer(false, offset, block, count),
        blk_req_code_t::BLK_REQ_FLUSH | blk_req_code_t::BLK_REQ_BARRIER => {
            (blk_resp_status_t::BLK_RESP_OK, 0)
        }
    }
}

fn transfer(read: bool, offset: u64, block: u64, count: u16) -> (blk_resp_status_t, u16) {
    let invalid = (blk_resp_status_t::BLK_RESP_ERR_INVALID_PARAM, 0);
    let Some((data, disk, len)) = span(offset, block, count) else {
        return invalid;
    };
    // SAFETY: `span` keeps both ranges inside the mapped data region and the RAM disk.
    unsafe {
        if read {
            core::ptr::copy_nonoverlapping(disk, data, len);
        } else {
            core::ptr::copy_nonoverlapping(data, disk, len);
        }
    }
    (blk_resp_status_t::BLK_RESP_OK, count)
}

fn span(offset: u64, block: u64, count: u16) -> Option<(*mut u8, *mut u8, usize)> {
    if count == 0 {
        return None;
    }
    let len = usize::from(count).checked_mul(BLK_TRANSFER_SIZE as usize)?;
    let offset_usize = usize::try_from(offset).ok()?;
    let data_len = usize::try_from(BLK_DATA_SIZE).ok()?;
    if offset_usize
        .checked_add(len)
        .is_none_or(|end| end > data_len)
    {
        return None;
    }
    let end_block = block.checked_add(u64::from(count))?;
    if end_block > DISK_BLOCKS {
        return None;
    }
    let disk_off = usize::try_from(block)
        .ok()?
        .checked_mul(BLK_TRANSFER_SIZE as usize)?;
    let data = usize::try_from(BLK_DATA_VADDR)
        .ok()?
        .checked_add(offset_usize)?;
    let disk = usize::try_from(DISK_VADDR).ok()?.checked_add(disk_off)?;
    Some((data as *mut u8, disk as *mut u8, len))
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.virt) {
            self.serve();
        }
        Ok(())
    }
}
