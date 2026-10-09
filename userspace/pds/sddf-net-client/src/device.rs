//! smoltcp device over the client transmit and receive queues.
//!
//! `receive` leaves the transmit free queue alone. smoltcp drops a transmit
//! token it does not use, and this client is not the producer of that free
//! queue after init. The token dequeues a free buffer only inside `consume`,
//! after the closure has filled a scratch frame, and notifies the transmit
//! virtualiser only after those bytes are in the shared buffer.

use core::marker::PhantomData;

use sel4_microkit::Channel;

use smoltcp::{
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    time::Instant,
};

use lerux_sddf::{
    net_buff_desc_t, net_cancel_signal_active, net_cancel_signal_free, net_dequeue_active,
    net_dequeue_free, net_enqueue_active, net_enqueue_free, net_image::NET_DATA_SIZE,
    net_queue_empty_active, net_queue_empty_free, net_queue_handle_t, net_require_signal_active,
    net_require_signal_free, NET_BUFFER_SIZE,
};

const FRAME_SIZE: usize = {
    assert!(NET_BUFFER_SIZE == 2048);
    2048
};

pub(crate) struct NetDevice {
    rx: net_queue_handle_t,
    tx: net_queue_handle_t,
    rx_data: *mut u8,
    tx_data: *mut u8,
    rx_ch: Channel,
    tx_ch: Channel,
    frame: [u8; FRAME_SIZE],
    frame_len: usize,
}

impl NetDevice {
    pub(crate) fn new(
        rx: net_queue_handle_t,
        tx: net_queue_handle_t,
        rx_data: *mut u8,
        tx_data: *mut u8,
        rx_ch: Channel,
        tx_ch: Channel,
    ) -> Self {
        Self {
            rx,
            tx,
            rx_data,
            tx_data,
            rx_ch,
            tx_ch,
            frame: [0; FRAME_SIZE],
            frame_len: 0,
        }
    }

    fn tx_token(&self) -> NetTxToken<'_> {
        NetTxToken {
            tx: duplicate(&self.tx),
            tx_data: self.tx_data,
            tx_ch: self.tx_ch,
            _lifetime: PhantomData,
        }
    }
}

fn duplicate(queue: &net_queue_handle_t) -> net_queue_handle_t {
    net_queue_handle_t {
        free: queue.free,
        active: queue.active,
        capacity: queue.capacity,
    }
}

fn offset_ok(offset: u64) -> bool {
    let buf = u64::from(NET_BUFFER_SIZE);
    offset.is_multiple_of(buf)
        && offset
            .checked_add(buf)
            .is_some_and(|end| end <= NET_DATA_SIZE)
}

pub(crate) struct NetRxToken<'a> {
    frame: *const u8,
    len: usize,
    _lifetime: PhantomData<&'a [u8]>,
}

pub(crate) struct NetTxToken<'a> {
    tx: net_queue_handle_t,
    tx_data: *mut u8,
    tx_ch: Channel,
    _lifetime: PhantomData<&'a mut NetDevice>,
}

impl RxToken for NetRxToken<'_> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        // SAFETY: `receive` copied `len` bytes into the device frame, and this
        // token's lifetime ends before the device is used again.
        let frame = unsafe { core::slice::from_raw_parts(self.frame, self.len) };
        f(frame)
    }
}

impl TxToken for NetTxToken<'_> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        assert!(len > 0 && len <= FRAME_SIZE, "offset fits");
        let mut scratch = [0u8; FRAME_SIZE];
        let result = f(&mut scratch[..len]);
        let mut buffer = net_buff_desc_t::new(0, 0, 0);
        // SAFETY: `receive` or `transmit` saw a free buffer, and only this
        // consume dequeues it. The client does not enqueue free.
        let err = unsafe { net_dequeue_free(&self.tx, &mut buffer) };
        assert_eq!(err, 0, "transmit free buffer is present");
        assert!(offset_ok(buffer.io_or_offset), "offset fits");
        let dst = usize::try_from(buffer.io_or_offset).expect("offset fits");
        // SAFETY: `dst` starts a buffer inside the mapped transmit data region,
        // and `len` fits in that buffer.
        unsafe {
            core::ptr::copy_nonoverlapping(scratch.as_ptr(), self.tx_data.add(dst), len);
        }
        buffer.len = u16::try_from(len).expect("packet length fits");
        buffer.set_oid(0);
        // SAFETY: this client produces the transmit active queue.
        let err = unsafe { net_enqueue_active(&self.tx, buffer) };
        assert_eq!(err, 0, "transmit active queue accepts the buffer");
        if unsafe { net_require_signal_active(&self.tx) } {
            unsafe { net_cancel_signal_active(&self.tx) };
            self.tx_ch.notify();
        }
        result
    }
}

impl Device for NetDevice {
    type RxToken<'a>
        = NetRxToken<'a>
    where
        Self: 'a;
    type TxToken<'a>
        = NetTxToken<'a>
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        // A missing transmit buffer would make smoltcp drop the ingress frame.
        // Leave the free descriptor queued until `TxToken::consume`.
        if unsafe { net_queue_empty_free(&self.tx) || net_queue_empty_active(&self.rx) } {
            return None;
        }
        let mut buffer = net_buff_desc_t::new(0, 0, 0);
        // SAFETY: the empty check is the dequeue condition. This client consumes the active queue.
        let err = unsafe { net_dequeue_active(&self.rx, &mut buffer) };
        assert_eq!(err, 0, "receive buffer is present");
        let len = usize::from(buffer.len);
        assert!(
            len > 0 && len <= FRAME_SIZE && offset_ok(buffer.io_or_offset),
            "offset fits"
        );
        // SAFETY: the offset check keeps `len` bytes inside the mapped receive region.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.rx_data
                    .add(usize::try_from(buffer.io_or_offset).expect("offset fits")),
                self.frame.as_mut_ptr(),
                len,
            );
        }
        self.frame_len = len;
        buffer.len = 0;
        // SAFETY: this client produces the receive free queue. The copier filled it.
        let err = unsafe { net_enqueue_free(&self.rx, buffer) };
        assert_eq!(err, 0, "receive free queue accepts the buffer");
        if unsafe { net_require_signal_free(&self.rx) } {
            unsafe { net_cancel_signal_free(&self.rx) };
            self.rx_ch.notify();
        }
        let token = NetRxToken {
            frame: self.frame.as_ptr(),
            len: self.frame_len,
            _lifetime: PhantomData,
        };
        Some((token, self.tx_token()))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        if unsafe { net_queue_empty_free(&self.tx) } {
            None
        } else {
            Some(self.tx_token())
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1514;
        caps
    }
}
