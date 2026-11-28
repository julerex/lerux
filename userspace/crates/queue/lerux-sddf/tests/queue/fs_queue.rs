//! Host checks for the filesystem command ring.
//!
//! The ring lives on the heap. `fs_queue_t` is 32 KiB, which is the header size
//! at capacity 511.

use core::mem::size_of;

use lerux_sddf::{
    fs_buffer_t, fs_cmd_params_file_write_t, fs_cmd_params_t, fs_cmd_t, fs_command_enqueue,
    fs_message_dequeue, fs_msg_t, fs_queue_t, FS_CMD_FILE_WRITE, FS_QUEUE_CAPACITY,
};

fn empty_queue() -> Box<fs_queue_t> {
    let queue = Box::<fs_queue_t>::new_zeroed();
    // SAFETY: every field is an integer, an atomic, or a byte array. The
    // all-zero pattern is an empty ring.
    unsafe { queue.assume_init() }
}

fn write_command(id: u64, tail_byte: u8) -> fs_cmd_t {
    let mut params = fs_cmd_params_t {
        min_size: [tail_byte; 48],
    };
    params.file_write = fs_cmd_params_file_write_t {
        fd: 7,
        offset: 0x1000,
        buf: fs_buffer_t {
            offset: 0x20,
            size: 4,
        },
    };
    fs_cmd_t {
        id,
        r#type: FS_CMD_FILE_WRITE,
        params,
    }
}

fn msg_bytes(msg: &fs_msg_t) -> [u8; 64] {
    assert_eq!(size_of::<fs_msg_t>(), 64);
    let mut bytes = [0u8; 64];
    // SAFETY: `fs_msg_t` is 64 bytes and `msg` is fully initialised.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(msg).cast::<u8>(),
            bytes.as_mut_ptr(),
            64,
        );
    }
    bytes
}

#[test]
fn command_and_message_are_64_bytes() {
    assert_eq!(FS_QUEUE_CAPACITY, 511);
    assert_eq!(size_of::<fs_cmd_t>(), 64);
    assert_eq!(size_of::<fs_msg_t>(), 64);
    assert_eq!(size_of::<fs_queue_t>(), 64 + 64 * FS_QUEUE_CAPACITY);
}

#[test]
fn enqueued_command_keeps_its_bytes() {
    let mut queue = empty_queue();
    let cmd = write_command(0x0123_4567_89ab_cdef, 0xa5);
    let sent = msg_bytes(&fs_msg_t { cmd });

    assert_eq!(unsafe { fs_command_enqueue(queue.as_mut(), cmd) }, 0);

    let mut got = fs_msg_t {
        cmd: write_command(0, 0),
    };
    assert_eq!(unsafe { fs_message_dequeue(queue.as_mut(), &mut got) }, 0);
    assert_eq!(msg_bytes(&got), sent);

    let got_cmd = unsafe { got.cmd };
    assert_eq!(got_cmd.id, 0x0123_4567_89ab_cdef);
    assert_eq!(got_cmd.r#type, FS_CMD_FILE_WRITE);
    let got_write = unsafe { got_cmd.params.file_write };
    assert_eq!(got_write.fd, 7);
    assert_eq!(got_write.offset, 0x1000);
    assert_eq!(got_write.buf.size, 4);
    assert_eq!(
        unsafe { got_cmd.params.min_size }[47],
        0xa5,
        "bytes past the active command parameters were rearranged"
    );

    let mut extra = fs_msg_t {
        cmd: write_command(0, 0),
    };
    assert_eq!(
        unsafe { fs_message_dequeue(queue.as_mut(), &mut extra) },
        -1
    );
}

#[test]
fn ring_fills_at_capacity_and_wraps() {
    let mut queue = empty_queue();
    for id in 0..FS_QUEUE_CAPACITY {
        let cmd = write_command(id as u64, 0);
        assert_eq!(
            unsafe { fs_command_enqueue(queue.as_mut(), cmd) },
            0,
            "{id}"
        );
    }
    assert_eq!(
        unsafe { fs_command_enqueue(queue.as_mut(), write_command(999, 0)) },
        -1
    );

    let mut first = fs_msg_t {
        cmd: write_command(0, 0),
    };
    assert_eq!(unsafe { fs_message_dequeue(queue.as_mut(), &mut first) }, 0);
    assert_eq!(unsafe { first.cmd.id }, 0);

    let wrapped = write_command(1_000, 0x5a);
    let wrapped_bytes = msg_bytes(&fs_msg_t { cmd: wrapped });
    assert_eq!(unsafe { fs_command_enqueue(queue.as_mut(), wrapped) }, 0);

    for id in 1..FS_QUEUE_CAPACITY {
        let mut msg = fs_msg_t {
            cmd: write_command(0, 0),
        };
        assert_eq!(unsafe { fs_message_dequeue(queue.as_mut(), &mut msg) }, 0);
        assert_eq!(unsafe { msg.cmd.id }, id as u64);
    }

    let mut last = fs_msg_t {
        cmd: write_command(0, 0),
    };
    assert_eq!(unsafe { fs_message_dequeue(queue.as_mut(), &mut last) }, 0);
    assert_eq!(msg_bytes(&last), wrapped_bytes);
    assert_eq!(unsafe { last.cmd.id }, 1_000);
    assert_eq!(unsafe { fs_message_dequeue(queue.as_mut(), &mut last) }, -1);
}
