//! Host exercise of one block request and one block response.

use lerux_sddf::{
    blk_dequeue_req, blk_dequeue_resp, blk_enqueue_req, blk_enqueue_resp, blk_queue_handle_t,
    blk_queue_init, blk_req_code_t, blk_resp_status_t, blk_storage_info_t, blk_storage_is_ready,
    blk_storage_set_ready,
};

#[repr(C, align(8))]
struct Page([u8; 4096]);

#[test]
fn enqueues_and_dequeues_a_request_and_a_response() {
    let mut requests = Page([0; 4096]);
    let mut responses = Page([0; 4096]);
    let mut handle = blk_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    unsafe {
        blk_queue_init(
            &mut handle,
            requests.0.as_mut_ptr().cast(),
            responses.0.as_mut_ptr().cast(),
            2,
        );
        (*handle.req_queue).plugged = true;

        let mut code = blk_req_code_t::BLK_REQ_FLUSH;
        let mut offset = 0u64;
        let mut block = 0u64;
        let mut count = 0u16;
        let mut id = 0u32;
        assert_eq!(
            blk_dequeue_req(
                &handle,
                &mut code,
                &mut offset,
                &mut block,
                &mut count,
                &mut id
            ),
            -1
        );
        assert_eq!(
            blk_enqueue_req(&handle, blk_req_code_t::BLK_REQ_READ, 0x1000, 7, 1, 9),
            0
        );
        assert_eq!(
            blk_dequeue_req(
                &handle,
                &mut code,
                &mut offset,
                &mut block,
                &mut count,
                &mut id
            ),
            0
        );
        assert_eq!(code, blk_req_code_t::BLK_REQ_READ);
        assert_eq!(offset, 0x1000);
        assert_eq!(block, 7);
        assert_eq!(count, 1);
        assert_eq!(id, 9);
        assert_eq!(
            blk_dequeue_req(
                &handle,
                &mut code,
                &mut offset,
                &mut block,
                &mut count,
                &mut id
            ),
            -1
        );

        assert_eq!(
            blk_enqueue_req(&handle, blk_req_code_t::BLK_REQ_WRITE, 0, 1, 1, 1),
            0
        );
        assert_eq!(
            blk_enqueue_req(&handle, blk_req_code_t::BLK_REQ_WRITE, 0, 2, 1, 2),
            0
        );
        assert_eq!(
            blk_enqueue_req(&handle, blk_req_code_t::BLK_REQ_WRITE, 0, 3, 1, 3),
            -1
        );
        assert_eq!(
            blk_dequeue_req(
                &handle,
                &mut code,
                &mut offset,
                &mut block,
                &mut count,
                &mut id
            ),
            0
        );
        assert_eq!(id, 1);
        assert_eq!(
            blk_dequeue_req(
                &handle,
                &mut code,
                &mut offset,
                &mut block,
                &mut count,
                &mut id
            ),
            0
        );
        assert_eq!(id, 2);
        assert_eq!(
            blk_enqueue_req(&handle, blk_req_code_t::BLK_REQ_READ, 8, 4, 2, 4),
            0
        );
        assert_eq!(
            blk_dequeue_req(
                &handle,
                &mut code,
                &mut offset,
                &mut block,
                &mut count,
                &mut id
            ),
            0
        );
        assert_eq!(id, 4);
        assert_eq!(offset, 8);
        assert_eq!(block, 4);
        assert_eq!(count, 2);

        let mut status = blk_resp_status_t::BLK_RESP_ERR_UNSPEC;
        let mut success = 1u16;
        let mut resp_id = 1u32;
        assert_eq!(
            blk_dequeue_resp(&handle, &mut status, &mut success, &mut resp_id),
            -1
        );
        assert_eq!(
            blk_enqueue_resp(&handle, blk_resp_status_t::BLK_RESP_OK, 1, 9),
            0
        );
        assert_eq!(
            blk_dequeue_resp(&handle, &mut status, &mut success, &mut resp_id),
            0
        );
        assert_eq!(status, blk_resp_status_t::BLK_RESP_OK);
        assert_eq!(success, 1);
        assert_eq!(resp_id, 9);
        assert_eq!(
            blk_dequeue_resp(&handle, &mut status, &mut success, &mut resp_id),
            -1
        );
        assert_eq!(
            blk_enqueue_resp(&handle, blk_resp_status_t::BLK_RESP_OK, 1, 11),
            0
        );
        assert_eq!(
            blk_enqueue_resp(&handle, blk_resp_status_t::BLK_RESP_ERR_IO, 0, 12),
            0
        );
        assert_eq!(
            blk_enqueue_resp(&handle, blk_resp_status_t::BLK_RESP_ERR_IO, 0, 13),
            -1
        );
        assert_eq!(
            blk_dequeue_resp(&handle, &mut status, &mut success, &mut resp_id),
            0
        );
        assert_eq!(resp_id, 11);
        assert_eq!(
            blk_dequeue_resp(&handle, &mut status, &mut success, &mut resp_id),
            0
        );
        assert_eq!(resp_id, 12);
        assert_eq!(status, blk_resp_status_t::BLK_RESP_ERR_IO);
        assert_eq!(
            blk_enqueue_resp(
                &handle,
                blk_resp_status_t::BLK_RESP_ERR_INVALID_PARAM,
                0,
                14
            ),
            0
        );
        assert_eq!(
            blk_dequeue_resp(&handle, &mut status, &mut success, &mut resp_id),
            0
        );
        assert_eq!(resp_id, 14);
        assert_eq!(status, blk_resp_status_t::BLK_RESP_ERR_INVALID_PARAM);
    }
}

#[test]
fn ready_flag_uses_acquire_and_release() {
    let mut info = core::mem::MaybeUninit::<blk_storage_info_t>::zeroed();
    unsafe {
        let info = info.as_mut_ptr();
        assert!(!blk_storage_is_ready(info));
        blk_storage_set_ready(info, true);
        assert!(blk_storage_is_ready(info));
        blk_storage_set_ready(info, false);
        assert!(!blk_storage_is_ready(info));
    }
}
