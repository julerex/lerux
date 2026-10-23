use lerux_gate::{serial_pair, Mmio, QueueError, Region, Shared};

#[repr(align(8))]
struct QueueBuf([u8; 64]);

fn leak_queue() -> Region {
    let buf = Box::leak(Box::new(QueueBuf([0; 64])));
    Region::from_mut(&mut buf.0)
}

fn leak_data(len: usize) -> Region {
    Region::from_mut(Box::leak(vec![0u8; len].into_boxed_slice()))
}

#[test]
fn enqueue_then_dequeue_one_byte() {
    let (tx, rx) = serial_pair(leak_queue(), leak_data(4), 4).expect("ring");
    tx.enqueue(b'q').expect("space");
    assert_eq!(rx.dequeue().expect("byte"), b'q');
    assert_eq!(rx.dequeue(), Err(QueueError::Empty));
}

#[test]
fn a_full_ring_rejects_the_next_byte() {
    let (tx, _rx) = serial_pair(leak_queue(), leak_data(2), 2).expect("ring");
    tx.enqueue(b'a').expect("space");
    tx.enqueue(b'b').expect("space");
    assert_eq!(tx.enqueue(b'c'), Err(QueueError::Full));
}

#[test]
fn mmio_round_trip_on_a_byte_window() {
    let window = Mmio::from_mut(Box::leak(vec![0u8; 4].into_boxed_slice()));
    window.write_u8(1, 0x2a);
    assert_eq!(window.read_u8(1), 0x2a);
}

#[test]
#[should_panic(expected = "Shared::with re-entered")]
fn shared_with_does_not_reenter() {
    let cell = Shared::new(1u32);
    cell.with(|value| {
        cell.with(|inner| *inner += 1);
        *value += 1;
    });
}
