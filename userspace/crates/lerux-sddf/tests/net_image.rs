//! Host checks for the one-client network configuration pages.

use core::mem::{offset_of, size_of};

use lerux_sddf::{
    net_client_config_t, net_config_check_magic, net_connection_resource_t, net_copy_config_t,
    net_driver_config_t,
    net_image::{
        client_config, copy_config, driver_config, net_config_from_bytes, net_config_to_bytes,
        virt_rx_config, virt_tx_config, CLIENT_MAC, CLIENT_NET_RX_CHANNEL, CLIENT_NET_TX_CHANNEL,
        COPY_CLIENT_CHANNEL, COPY_VIRT_CHANNEL, DRIVER_RX_CHANNEL, DRIVER_TX_CHANNEL,
        NET_DATA_SIZE, NET_QUEUE_CAPACITY, NET_QUEUE_REGION_SIZE, NET_RX_CLIENT_ACTIVE_VADDR,
        NET_RX_CLIENT_DATA_VADDR, NET_RX_CLIENT_FREE_VADDR, NET_RX_COPY_ACTIVE_VADDR,
        NET_RX_COPY_FREE_VADDR, NET_RX_DATA_VADDR, NET_RX_DRV_ACTIVE_VADDR, NET_RX_DRV_FREE_VADDR,
        NET_RX_META_VADDR, NET_TX_CLIENT_ACTIVE_VADDR, NET_TX_CLIENT_FREE_VADDR, NET_TX_DATA_VADDR,
        NET_TX_DRV_ACTIVE_VADDR, NET_TX_DRV_FREE_VADDR, VIRT_RX_COPY_CHANNEL,
        VIRT_RX_DRIVER_CHANNEL, VIRT_TX_CLIENT_CHANNEL, VIRT_TX_DRIVER_CHANNEL,
    },
    net_virt_rx_config_t, net_virt_tx_config_t, NET_BUFFER_SIZE, SDDF_NET_MAGIC,
};

fn addr(ptr: *mut u8) -> u64 {
    ptr as u64
}

fn bytes<T>(value: &T) -> Vec<u8> {
    let mut stored = vec![0u8; size_of::<T>()];
    net_config_to_bytes(value, &mut stored);
    stored
}

#[test]
fn queue_regions_hold_the_descriptor_array() {
    let used = size_of::<lerux_sddf::net_queue_t>()
        + size_of::<lerux_sddf::net_buff_desc_t>() * usize::from(NET_QUEUE_CAPACITY);
    assert!(used <= NET_QUEUE_REGION_SIZE as usize);
    assert_eq!(
        NET_DATA_SIZE,
        u64::from(NET_QUEUE_CAPACITY) * u64::from(NET_BUFFER_SIZE)
    );
    assert!(size_of::<net_virt_tx_config_t>() > 0x10_000);
}

#[test]
fn regions_do_not_overlap() {
    let mut ranges = [
        (NET_RX_DRV_FREE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_RX_DRV_ACTIVE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_TX_DRV_FREE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_TX_DRV_ACTIVE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_RX_COPY_FREE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_RX_COPY_ACTIVE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_RX_CLIENT_FREE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_RX_CLIENT_ACTIVE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_TX_CLIENT_FREE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_TX_CLIENT_ACTIVE_VADDR, NET_QUEUE_REGION_SIZE),
        (NET_RX_DATA_VADDR, NET_DATA_SIZE),
        (NET_TX_DATA_VADDR, NET_DATA_SIZE),
        (NET_RX_CLIENT_DATA_VADDR, NET_DATA_SIZE),
        (NET_RX_META_VADDR, NET_QUEUE_REGION_SIZE),
    ];
    ranges.sort_by_key(|range| range.0);
    for window in ranges.windows(2) {
        let (start, size) = window[0];
        let (next, _) = window[1];
        assert!(start >= 0x5_000_000);
        assert!(start + size <= next);
    }
}

#[test]
fn driver_and_virtualisers_share_queues_with_different_channels() {
    let driver = driver_config();
    let tx = virt_tx_config();
    let rx = virt_rx_config();
    assert_eq!(driver.magic, SDDF_NET_MAGIC);
    assert_eq!(tx.magic, SDDF_NET_MAGIC);
    assert_eq!(rx.magic, SDDF_NET_MAGIC);
    assert_eq!(driver.virt_rx.id, DRIVER_RX_CHANNEL);
    assert_eq!(driver.virt_tx.id, DRIVER_TX_CHANNEL);
    assert_eq!(driver.virt_rx.num_buffers, NET_QUEUE_CAPACITY);
    assert_eq!(addr(driver.virt_rx.free_queue.vaddr), NET_RX_DRV_FREE_VADDR);
    assert_eq!(
        addr(driver.virt_rx.active_queue.vaddr),
        NET_RX_DRV_ACTIVE_VADDR
    );
    assert_eq!(addr(driver.virt_tx.free_queue.vaddr), NET_TX_DRV_FREE_VADDR);
    assert_eq!(
        addr(driver.virt_tx.active_queue.vaddr),
        NET_TX_DRV_ACTIVE_VADDR
    );
    assert_eq!(tx.num_clients, 1);
    assert_eq!(tx.driver.id, VIRT_TX_DRIVER_CHANNEL);
    assert_ne!(driver.virt_tx.id, tx.driver.id);
    assert_eq!(
        addr(tx.driver.free_queue.vaddr),
        addr(driver.virt_tx.free_queue.vaddr)
    );
    assert_eq!(
        addr(tx.driver.active_queue.vaddr),
        addr(driver.virt_tx.active_queue.vaddr)
    );
    assert_eq!(tx.clients[0].num_regions, 1);
    assert_eq!(tx.clients[0].conn.id, VIRT_TX_CLIENT_CHANNEL);
    assert_eq!(
        addr(tx.clients[0].conn.free_queue.vaddr),
        NET_TX_CLIENT_FREE_VADDR
    );
    assert_eq!(
        addr(tx.clients[0].regions[0].data.region.vaddr),
        NET_TX_DATA_VADDR
    );
    assert_eq!(tx.clients[0].regions[0].data.io_addr, 0);
    assert_eq!(
        tx.clients[0].regions[0].num_buffers,
        u32::from(NET_QUEUE_CAPACITY)
    );
    assert_eq!(tx.clients[1].num_regions, 0);

    assert_eq!(rx.num_clients, 1);
    assert_eq!(rx.driver.id, VIRT_RX_DRIVER_CHANNEL);
    assert_ne!(driver.virt_rx.id, rx.driver.id);
    assert_eq!(
        addr(rx.driver.free_queue.vaddr),
        addr(driver.virt_rx.free_queue.vaddr)
    );
    assert_eq!(addr(rx.data.region.vaddr), NET_RX_DATA_VADDR);
    assert_eq!(rx.data.io_addr, 0);
    assert_eq!(addr(rx.buffer_metadata.vaddr), NET_RX_META_VADDR);
    assert_eq!(rx.clients[0].num_macs, 1);
    assert_eq!(rx.clients[0].conn.id, VIRT_RX_COPY_CHANNEL);
    assert_eq!(rx.clients[0].mac_addrs[0].addr, CLIENT_MAC.addr);
    assert_eq!(rx.clients[1].num_macs, 0);
}

#[test]
fn copier_and_client_meet_on_the_client_queues() {
    let tx = virt_tx_config();
    let rx = virt_rx_config();
    let copy = copy_config();
    let client = client_config();

    assert_eq!(copy.magic, SDDF_NET_MAGIC);
    assert_eq!(client.magic, SDDF_NET_MAGIC);
    assert_eq!(copy.rx.id, COPY_VIRT_CHANNEL);
    assert_eq!(copy.client.id, COPY_CLIENT_CHANNEL);
    assert_ne!(rx.clients[0].conn.id, copy.rx.id);
    assert_eq!(
        addr(copy.rx.free_queue.vaddr),
        addr(rx.clients[0].conn.free_queue.vaddr)
    );
    assert_eq!(
        addr(copy.rx.active_queue.vaddr),
        addr(rx.clients[0].conn.active_queue.vaddr)
    );
    assert_eq!(addr(copy.rx_data[0].vaddr), NET_RX_DATA_VADDR);
    assert!(copy.rx_data[1].vaddr.is_null());
    assert_eq!(addr(copy.client.free_queue.vaddr), NET_RX_CLIENT_FREE_VADDR);
    assert_eq!(
        addr(copy.client.active_queue.vaddr),
        NET_RX_CLIENT_ACTIVE_VADDR
    );
    assert_eq!(addr(copy.client_data.vaddr), NET_RX_CLIENT_DATA_VADDR);

    assert_eq!(client.rx.id, CLIENT_NET_RX_CHANNEL);
    assert_eq!(client.tx.id, CLIENT_NET_TX_CHANNEL);
    assert_ne!(copy.client.id, client.rx.id);
    assert_eq!(
        addr(client.rx.free_queue.vaddr),
        addr(copy.client.free_queue.vaddr)
    );
    assert_eq!(
        addr(client.rx.active_queue.vaddr),
        addr(copy.client.active_queue.vaddr)
    );
    assert_eq!(addr(client.rx_data.vaddr), NET_RX_CLIENT_DATA_VADDR);
    assert_eq!(
        addr(client.tx.free_queue.vaddr),
        addr(tx.clients[0].conn.free_queue.vaddr)
    );
    assert_eq!(
        addr(client.tx.active_queue.vaddr),
        addr(tx.clients[0].conn.active_queue.vaddr)
    );
    assert_eq!(addr(client.tx_data.vaddr), NET_TX_DATA_VADDR);
    assert_eq!(client.mac_addr.addr, CLIENT_MAC.addr);
}

#[test]
fn config_bytes_round_trip_and_keep_padding_zero() {
    let driver = driver_config();
    let driver_bytes = bytes(&driver);
    assert!(net_config_check_magic(&driver_bytes));
    assert!(!net_config_check_magic(&driver_bytes[..4]));
    let id_at =
        offset_of!(net_driver_config_t, virt_rx) + offset_of!(net_connection_resource_t, id);
    assert_eq!(driver_bytes[id_at], DRIVER_RX_CHANNEL);
    let tx_at = offset_of!(net_driver_config_t, virt_tx);
    assert!(driver_bytes[id_at + 1..tx_at].iter().all(|byte| *byte == 0));
    assert_eq!(bytes(&driver_config()), driver_bytes);
    // SAFETY: the bytes were copied from a live `net_driver_config_t`.
    let driver_back = unsafe { net_config_from_bytes::<net_driver_config_t>(&driver_bytes) };
    assert_eq!(driver_back.magic, driver.magic);
    assert_eq!(driver_back.virt_rx.id, driver.virt_rx.id);
    assert_eq!(
        addr(driver_back.virt_rx.free_queue.vaddr),
        addr(driver.virt_rx.free_queue.vaddr)
    );

    let tx = virt_tx_config();
    let tx_bytes = bytes(&tx);
    assert!(net_config_check_magic(&tx_bytes));
    // SAFETY: the bytes were copied from a live `net_virt_tx_config_t`.
    let tx_back = unsafe { net_config_from_bytes::<net_virt_tx_config_t>(&tx_bytes) };
    assert_eq!(tx_back.num_clients, 1);
    assert_eq!(tx_back.clients[0].num_regions, 1);
    assert_eq!(tx_back.clients[0].conn.id, VIRT_TX_CLIENT_CHANNEL);

    let rx = virt_rx_config();
    let rx_bytes = bytes(&rx);
    // SAFETY: the bytes were copied from a live `net_virt_rx_config_t`.
    let rx_back = unsafe { net_config_from_bytes::<net_virt_rx_config_t>(&rx_bytes) };
    assert_eq!(rx_back.clients[0].mac_addrs[0].addr, CLIENT_MAC.addr);
    assert_eq!(rx_back.buffer_metadata.size, NET_QUEUE_REGION_SIZE);

    let copy_bytes = bytes(&copy_config());
    // SAFETY: the bytes were copied from a live `net_copy_config_t`.
    let copy_back = unsafe { net_config_from_bytes::<net_copy_config_t>(&copy_bytes) };
    assert_eq!(copy_back.client.id, COPY_CLIENT_CHANNEL);
    assert!(copy_back.rx_data[1].vaddr.is_null());

    let client_bytes = bytes(&client_config());
    // SAFETY: the bytes were copied from a live `net_client_config_t`.
    let client_back = unsafe { net_config_from_bytes::<net_client_config_t>(&client_bytes) };
    assert_eq!(client_back.rx.id, CLIENT_NET_RX_CHANNEL);
    assert_eq!(client_back.tx.id, CLIENT_NET_TX_CHANNEL);
    assert_eq!(client_back.mac_addr.addr, CLIENT_MAC.addr);
}
