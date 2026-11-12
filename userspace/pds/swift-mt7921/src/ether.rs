//! Ethernet frames on the standard sDDF network queues.
//!
//! Received 802.11 data is decrypted and written to the receive active queue.
//! A transmit frame is taken from the transmit active queue, then protected.
//! DHCP and ICMP are not built here. The shell does both once it is wired.
//!
//! Queue indexes follow `net_queue`: an acquire load for the index the other
//! side writes, and a release store for the index this side owns.

use lerux_sddf::{
    net_buff_desc_t, net_cancel_signal_active, net_cancel_signal_free, net_connection_resource_t,
    net_dequeue_active, net_dequeue_free, net_driver_config_t, net_enqueue_active,
    net_enqueue_free,
    net_image::{NET_DATA_SIZE, NET_RX_DATA_VADDR, NET_TX_DATA_VADDR},
    net_queue_empty_active, net_queue_empty_free, net_queue_handle_t, net_queue_init,
    net_request_signal_active, net_request_signal_free, net_require_signal_active,
    net_require_signal_free, NET_BUFFER_SIZE,
};
use lerux_swift::wpa::{ccmp_decrypt, ccmp_encrypt, llc_from_ethernet, write_qos_data};

/// Ethernet body that still fits in a radio buffer after the 802.11 and CCMP headers.
const ETHER_LIMIT: usize = 1500;
const FRAME_MAX: usize = 1600;
const SNAP: [u8; 6] = [0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00];

/// Pairwise association. Packet numbers belong to frames this station sends.
pub(crate) struct Link {
    pub bssid: [u8; 6],
    pub sta: [u8; 6],
    pub tk: [u8; 16],
    pub pn: u64,
    pub seq: u16,
    pub quiet: u32,
}

/// What the other protection domain asked to hear about.
pub(crate) struct Note {
    pub rx: bool,
    pub tx: bool,
}

impl Note {
    pub(crate) const fn quiet() -> Self {
        Self {
            rx: false,
            tx: false,
        }
    }
}

/// One transmit dequeue.
pub(crate) enum TxFrame {
    Empty,
    Dropped,
    Wifi(usize),
}

/// Driver-side receive and transmit queues, plus the shared data regions.
pub(crate) struct Net {
    pub rx: net_queue_handle_t,
    pub tx: net_queue_handle_t,
    rx_data: *mut u8,
    tx_data: *mut u8,
}

impl Net {
    /// Store the standard driver queues. This does not load those pages.
    pub(crate) fn from_driver(config: &net_driver_config_t) -> Self {
        Self {
            rx: handle(&config.virt_rx),
            tx: handle(&config.virt_tx),
            rx_data: data_ptr(NET_RX_DATA_VADDR),
            tx_data: data_ptr(NET_TX_DATA_VADDR),
        }
    }

    pub(crate) fn rx_id(config: &net_driver_config_t) -> u8 {
        config.virt_rx.id
    }

    pub(crate) fn tx_id(config: &net_driver_config_t) -> u8 {
        config.virt_tx.id
    }

    /// True when a free receive buffer is available. Asks for one when it is not.
    pub(crate) fn rx_has_room(&self) -> bool {
        if unsafe { net_queue_empty_free(&self.rx) } {
            unsafe { net_request_signal_free(&self.rx) };
            if unsafe { net_queue_empty_free(&self.rx) } {
                return false;
            }
            unsafe { net_cancel_signal_free(&self.rx) };
        }
        true
    }

    /// Decrypt one From-DS frame and enqueue the Ethernet result.
    pub(crate) fn enqueue_received(&self, link: &Link, wifi: &[u8]) -> bool {
        let mut ether = [0u8; FRAME_MAX];
        let Some(n) = wifi_to_ethernet(link, wifi, &mut ether) else {
            return false;
        };
        self.enqueue_ethernet(&ether[..n])
    }

    /// Take one Ethernet frame off the transmit queue and protect it.
    pub(crate) fn dequeue_transmit(&self, link: &mut Link, out: &mut [u8]) -> TxFrame {
        let mut ether = [0u8; FRAME_MAX];
        let n = match self.dequeue_ethernet(&mut ether) {
            TxFrame::Empty => return TxFrame::Empty,
            TxFrame::Dropped => return TxFrame::Dropped,
            TxFrame::Wifi(n) => n,
        };
        match ethernet_to_wifi(link, &ether[..n], out) {
            Some(len) => TxFrame::Wifi(len),
            None => TxFrame::Dropped,
        }
    }

    /// The receive consumer asked to be told about a new frame.
    pub(crate) fn note_rx(&self) -> bool {
        unsafe {
            if net_require_signal_active(&self.rx) {
                net_cancel_signal_active(&self.rx);
                true
            } else {
                false
            }
        }
    }

    /// The transmit producer asked for its buffer back.
    pub(crate) fn note_tx(&self) -> bool {
        unsafe {
            if net_require_signal_free(&self.tx) {
                net_cancel_signal_free(&self.tx);
                true
            } else {
                false
            }
        }
    }

    fn enqueue_ethernet(&self, frame: &[u8]) -> bool {
        let cap = usize::try_from(NET_BUFFER_SIZE).expect("buffer size");
        if frame.is_empty() || frame.len() > cap {
            return false;
        }
        let mut desc = net_buff_desc_t::new(0, 0, 0);
        if unsafe { net_dequeue_free(&self.rx, &mut desc) } != 0 {
            return false;
        }
        let len = frame.len();
        let Some(dst) = region_mut(self.rx_data, desc.io_or_offset, len) else {
            restore_free(&self.rx, desc.io_or_offset);
            return false;
        };
        unsafe { core::ptr::copy_nonoverlapping(frame.as_ptr(), dst, len) };
        let out = net_buff_desc_t::new(desc.io_or_offset, frame_len(len), 0);
        if unsafe { net_enqueue_active(&self.rx, out) } != 0 {
            restore_free(&self.rx, desc.io_or_offset);
            return false;
        }
        true
    }

    fn dequeue_ethernet(&self, out: &mut [u8]) -> TxFrame {
        if unsafe { net_queue_empty_active(&self.tx) } {
            unsafe { net_request_signal_active(&self.tx) };
            if unsafe { net_queue_empty_active(&self.tx) } {
                return TxFrame::Empty;
            }
            unsafe { net_cancel_signal_active(&self.tx) };
        }
        let mut desc = net_buff_desc_t::new(0, 0, 0);
        if unsafe { net_dequeue_active(&self.tx, &mut desc) } != 0 {
            return TxFrame::Empty;
        }
        let len = usize::from(desc.len);
        let copied = region_ref(self.tx_data, desc.io_or_offset, len).map(|src| {
            let n = len.min(out.len());
            unsafe { core::ptr::copy_nonoverlapping(src, out.as_mut_ptr(), n) };
            n
        });
        restore_free(&self.tx, desc.io_or_offset);
        match copied {
            Some(n) if n > 0 => TxFrame::Wifi(n),
            _ => TxFrame::Dropped,
        }
    }
}

/// Empty protected `QoS` data to the access point, so the association stays up.
pub(crate) fn write_keepalive(link: &mut Link, out: &mut [u8]) -> Option<usize> {
    let seq = take_seq(link);
    let mut hdr = [0u8; 26];
    write_qos_data(
        &mut hdr,
        &link.bssid,
        &link.sta,
        &link.bssid,
        &[],
        seq,
        true,
    )?;
    let pn = take_pn(link);
    let mut body = [0u8; 16];
    let clen = ccmp_encrypt(&link.tk, &hdr, pn, &[], &mut body)?;
    write_qos_data(
        out,
        &link.bssid,
        &link.sta,
        &link.bssid,
        &body[..clen],
        seq,
        true,
    )
}

fn wifi_to_ethernet(link: &Link, frame: &[u8], out: &mut [u8]) -> Option<usize> {
    if frame.len() < 26 || frame[0] & 0xfc != 0x88 || frame[1] & 0x43 != 0x42 {
        return None;
    }
    let mut plain = [0u8; FRAME_MAX];
    let n = ccmp_decrypt(&link.tk, 26, frame, &mut plain)?;
    let msdu = &plain[..n];
    let payload = msdu.strip_prefix(&SNAP)?;
    if payload.len() >= 2 && payload[0] == 0x88 && payload[1] == 0x8e {
        return None;
    }
    let mut da = [0u8; 6];
    let mut sa = [0u8; 6];
    da.copy_from_slice(frame.get(4..10)?);
    sa.copy_from_slice(frame.get(16..22)?);
    if da != link.sta && da[0] & 0x01 == 0 {
        return None;
    }
    if out.len() < 12 + payload.len() {
        return None;
    }
    out[..6].copy_from_slice(&da);
    out[6..12].copy_from_slice(&sa);
    out[12..12 + payload.len()].copy_from_slice(payload);
    Some(12 + payload.len())
}

fn ethernet_to_wifi(link: &mut Link, ether: &[u8], out: &mut [u8]) -> Option<usize> {
    if ether.len() < 14 || ether.len() > ETHER_LIMIT {
        return None;
    }
    let mut llc = [0u8; FRAME_MAX];
    let n = llc_from_ethernet(ether, &mut llc)?;
    let mut da = [0u8; 6];
    da.copy_from_slice(&ether[..6]);
    let seq = take_seq(link);
    let mut hdr = [0u8; 26];
    write_qos_data(&mut hdr, &link.bssid, &link.sta, &da, &[], seq, true)?;
    let pn = take_pn(link);
    let mut body = [0u8; FRAME_MAX];
    let clen = ccmp_encrypt(&link.tk, &hdr, pn, &llc[..n], &mut body)?;
    write_qos_data(out, &link.bssid, &link.sta, &da, &body[..clen], seq, true)
}

fn take_seq(link: &mut Link) -> u16 {
    let seq = link.seq;
    link.seq = link.seq.wrapping_add(1);
    seq
}

fn take_pn(link: &mut Link) -> u64 {
    let pn = link.pn;
    link.pn = link.pn.wrapping_add(1);
    pn
}

fn handle(conn: &net_connection_resource_t) -> net_queue_handle_t {
    let mut queue = net_queue_handle_t {
        free: core::ptr::null_mut(),
        active: core::ptr::null_mut(),
        capacity: 0,
    };
    unsafe {
        net_queue_init(
            &mut queue,
            conn.free_queue.vaddr.cast(),
            conn.active_queue.vaddr.cast(),
            u32::from(conn.num_buffers),
        );
    }
    queue
}

fn data_ptr(vaddr: u64) -> *mut u8 {
    let addr = usize::try_from(vaddr).expect("region address");
    addr as *mut u8
}

fn frame_len(len: usize) -> u16 {
    u16::try_from(len).expect("frame length fits")
}

fn restore_free(queue: &net_queue_handle_t, offset: u64) {
    let back = net_buff_desc_t::new(offset, 0, 0);
    let _ = unsafe { net_enqueue_free(queue, back) };
}

fn region_mut(base: *mut u8, offset: u64, len: usize) -> Option<*mut u8> {
    let off = region_off(offset, len)?;
    Some(base.wrapping_add(off))
}

fn region_ref(base: *mut u8, offset: u64, len: usize) -> Option<*const u8> {
    region_mut(base, offset, len).map(<*mut u8>::cast_const)
}

fn region_off(offset: u64, len: usize) -> Option<usize> {
    if len == 0 || len > usize::try_from(NET_BUFFER_SIZE).expect("buffer size") {
        return None;
    }
    if !offset.is_multiple_of(u64::from(NET_BUFFER_SIZE)) {
        return None;
    }
    let len_u64 = u64::try_from(len).expect("length fits");
    let end = offset.checked_add(len_u64)?;
    if end > NET_DATA_SIZE {
        return None;
    }
    usize::try_from(offset).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lerux_sddf::{net_buffers_init, net_dequeue_active, net_enqueue_active, net_queue_length};

    const CAP: u32 = 16;

    struct Fix {
        net: Net,
        rx_free: Box<Region>,
        rx_active: Box<Region>,
        tx_free: Box<Region>,
        tx_active: Box<Region>,
        rx_data: Box<[u8]>,
        tx_data: Box<[u8]>,
    }

    #[repr(C, align(8))]
    struct Region([u8; 4096]);

    const DATA_LEN: usize = 0x8000;

    impl Fix {
        fn new() -> Self {
            let mut fix = Self {
                net: Net {
                    rx: net_queue_handle_t {
                        free: core::ptr::null_mut(),
                        active: core::ptr::null_mut(),
                        capacity: 0,
                    },
                    tx: net_queue_handle_t {
                        free: core::ptr::null_mut(),
                        active: core::ptr::null_mut(),
                        capacity: 0,
                    },
                    rx_data: core::ptr::null_mut(),
                    tx_data: core::ptr::null_mut(),
                },
                rx_free: Box::new(Region([0; 4096])),
                rx_active: Box::new(Region([0; 4096])),
                tx_free: Box::new(Region([0; 4096])),
                tx_active: Box::new(Region([0; 4096])),
                rx_data: vec![0u8; DATA_LEN].into_boxed_slice(),
                tx_data: vec![0u8; DATA_LEN].into_boxed_slice(),
            };
            fix.install();
            fix
        }

        fn install(&mut self) {
            self.net.rx_data = self.rx_data.as_mut_ptr();
            self.net.tx_data = self.tx_data.as_mut_ptr();
            unsafe {
                net_queue_init(
                    &mut self.net.rx,
                    self.rx_free.0.as_mut_ptr().cast(),
                    self.rx_active.0.as_mut_ptr().cast(),
                    CAP,
                );
                net_queue_init(
                    &mut self.net.tx,
                    self.tx_free.0.as_mut_ptr().cast(),
                    self.tx_active.0.as_mut_ptr().cast(),
                    CAP,
                );
                net_buffers_init(&self.net.rx, 0);
            }
        }
    }

    fn link() -> Link {
        Link {
            bssid: [0x02, 0, 0, 0, 0, 1],
            sta: [0x02, 0x14, 0xc3, 0x79, 0x21, 0x01],
            tk: [0x11; 16],
            pn: 1,
            seq: 1,
            quiet: 0,
        }
    }

    fn ethernet(dst: [u8; 6], src: [u8; 6], body: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(14 + body.len());
        frame.extend_from_slice(&dst);
        frame.extend_from_slice(&src);
        frame.extend_from_slice(&0x0800u16.to_be_bytes());
        frame.extend_from_slice(body);
        frame
    }

    fn protect_from_ds(link: &Link, ether: &[u8]) -> Vec<u8> {
        let mut llc = [0u8; FRAME_MAX];
        let n = llc_from_ethernet(ether, &mut llc).expect("llc");
        let mut hdr = [0u8; 26];
        hdr[0] = 0x88;
        hdr[1] = 0x42;
        hdr[4..10].copy_from_slice(&ether[..6]);
        hdr[10..16].copy_from_slice(&link.bssid);
        hdr[16..22].copy_from_slice(&ether[6..12]);
        let mut body = [0u8; FRAME_MAX];
        let clen = ccmp_encrypt(&link.tk, &hdr, 7, &llc[..n], &mut body).expect("ccmp");
        let mut frame = vec![0u8; 26 + clen];
        frame[..26].copy_from_slice(&hdr);
        frame[26..].copy_from_slice(&body[..clen]);
        frame
    }

    fn read_rx(fix: &mut Fix) -> Vec<u8> {
        let mut desc = net_buff_desc_t::new(0, 0, 0);
        assert_eq!(unsafe { net_dequeue_active(&fix.net.rx, &mut desc) }, 0);
        let off = usize::try_from(desc.io_or_offset).expect("offset");
        let len = usize::from(desc.len);
        fix.rx_data[off..off + len].to_vec()
    }

    fn write_tx(fix: &mut Fix, ether: &[u8]) {
        let len = ether.len();
        fix.tx_data[..len].copy_from_slice(ether);
        let desc = net_buff_desc_t::new(0, frame_len(len), 0);
        assert_eq!(unsafe { net_enqueue_active(&fix.net.tx, desc) }, 0);
    }

    fn ethernet_from_tx(link: &Link, frame: &[u8]) -> Vec<u8> {
        assert_eq!(frame[0], 0x88);
        assert_eq!(frame[1] & 0x41, 0x41);
        let mut plain = [0u8; FRAME_MAX];
        let n = ccmp_decrypt(&link.tk, 26, frame, &mut plain).expect("decrypt");
        let payload = plain[..n].strip_prefix(&SNAP).expect("snap");
        let mut ether = Vec::with_capacity(12 + payload.len());
        ether.extend_from_slice(&frame[16..22]);
        ether.extend_from_slice(&frame[10..16]);
        ether.extend_from_slice(payload);
        ether
    }

    #[test]
    fn client_receive_queue_and_transmit_queue() {
        let mut link = link();
        let mut fix = Fix::new();
        let down = ethernet(link.sta, [0x02, 0, 0, 0, 0, 9], b"radio-frame");
        let wifi = protect_from_ds(&link, &down);
        assert!(fix.net.enqueue_received(&link, &wifi));
        assert_eq!(read_rx(&mut fix), down);

        let up = ethernet([0x02, 0, 0, 0, 0, 9], link.sta, b"uplink-body");
        write_tx(&mut fix, &up);
        assert_eq!(unsafe { net_queue_length(fix.net.tx.active) }, 1);
        let mut out = [0u8; FRAME_MAX];
        let n = match fix.net.dequeue_transmit(&mut link, &mut out) {
            TxFrame::Wifi(n) => n,
            TxFrame::Empty | TxFrame::Dropped => panic!("transmit frame missing"),
        };
        assert_eq!(unsafe { net_queue_length(fix.net.tx.active) }, 0);
        assert_eq!(ethernet_from_tx(&link, &out[..n]), up);
    }

    #[test]
    fn keepalive_is_empty_protected_data() {
        let mut link = link();
        let mut frame = [0u8; 64];
        let n = write_keepalive(&mut link, &mut frame).expect("keepalive");
        assert_eq!(frame[0], 0x88);
        assert_eq!(frame[1] & 0x41, 0x41);
        assert_eq!(&frame[4..10], &link.bssid);
        let mut plain = [0u8; 16];
        let got = ccmp_decrypt(&link.tk, 26, &frame[..n], &mut plain).expect("decrypt");
        assert_eq!(got, 0);
    }
}
