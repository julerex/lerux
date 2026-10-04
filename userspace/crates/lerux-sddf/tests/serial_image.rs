//! Host checks for the one-client serial configuration pages.

use core::mem::size_of;

use lerux_sddf::{
    serial_client_config_t, serial_config_check_magic, serial_driver_config_t,
    serial_image::{
        client_config, client_name_bytes, client_name_field_len, driver_config,
        serial_config_from_bytes, serial_config_to_bytes, virt_rx_config, virt_tx_config,
        CLIENT_RX_CHANNEL, CLIENT_TX_CHANNEL, DEFAULT_BAUD, DRIVER_RX_CHANNEL, DRIVER_TX_CHANNEL,
        RX_CLIENT_DATA_VADDR, RX_CLIENT_QUEUE_VADDR, RX_DRIVER_DATA_VADDR, RX_DRIVER_QUEUE_VADDR,
        SWITCH_CHAR, TERMINATE_NUM_CHAR, TX_CLIENT_DATA_VADDR, TX_CLIENT_QUEUE_VADDR,
        TX_DRIVER_DATA_VADDR, TX_DRIVER_QUEUE_VADDR, VIRT_RX_CLIENT_CHANNEL,
        VIRT_RX_DRIVER_CHANNEL, VIRT_TX_CLIENT_CHANNEL, VIRT_TX_DRIVER_CHANNEL,
    },
    serial_virt_rx_config_t, serial_virt_tx_config_t, SDDF_SERIAL_MAGIC,
};

fn addr(ptr: *mut u8) -> u64 {
    ptr as u64
}

#[test]
fn driver_config_points_at_the_driver_queues() {
    let config = driver_config();
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(config.default_baud, DEFAULT_BAUD);
    assert!(config.rx_enabled);
    assert_eq!(config.tx.id, DRIVER_TX_CHANNEL);
    assert_eq!(config.rx.id, DRIVER_RX_CHANNEL);
    assert_eq!(addr(config.tx.queue.vaddr), TX_DRIVER_QUEUE_VADDR);
    assert_eq!(addr(config.tx.data.vaddr), TX_DRIVER_DATA_VADDR);
    assert_eq!(addr(config.rx.queue.vaddr), RX_DRIVER_QUEUE_VADDR);
    assert_eq!(addr(config.rx.data.vaddr), RX_DRIVER_DATA_VADDR);
    assert_eq!(config.tx.data.size, 0x1000);
}

#[test]
fn virt_tx_config_names_one_client_and_leaves_colour_off() {
    let config = virt_tx_config();
    assert!(size_of::<serial_virt_tx_config_t>() < 0x8000);
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(config.num_clients, 1);
    assert!(!config.enable_colour);
    assert!(!config.enable_rx);
    assert_eq!(config.driver.id, VIRT_TX_DRIVER_CHANNEL);
    assert_eq!(addr(config.driver.queue.vaddr), TX_DRIVER_QUEUE_VADDR);
    assert_eq!(addr(config.driver.data.vaddr), TX_DRIVER_DATA_VADDR);
    assert_eq!(config.clients[0].conn.id, VIRT_TX_CLIENT_CHANNEL);
    assert_eq!(
        addr(config.clients[0].conn.queue.vaddr),
        TX_CLIENT_QUEUE_VADDR
    );
    assert_eq!(
        addr(config.clients[0].conn.data.vaddr),
        TX_CLIENT_DATA_VADDR
    );
    assert_eq!(config.clients[0].name.len(), client_name_field_len());
    assert!(config.clients[0].name.starts_with(client_name_bytes()));
    assert_eq!(config.clients[0].name[client_name_bytes().len()], 0);
}

#[test]
fn virt_rx_and_the_client_share_the_client_queues() {
    let driver = driver_config();
    let tx = virt_tx_config();
    let rx = virt_rx_config();
    let client = client_config();

    assert_eq!(rx.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(client.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(rx.num_clients, 1);
    assert_eq!(rx.switch_char, SWITCH_CHAR);
    assert_eq!(rx.terminate_num_char, TERMINATE_NUM_CHAR);
    assert_eq!(rx.driver.id, VIRT_RX_DRIVER_CHANNEL);
    assert_eq!(rx.clients[0].id, VIRT_RX_CLIENT_CHANNEL);
    assert_eq!(addr(rx.driver.queue.vaddr), addr(driver.rx.queue.vaddr));
    assert_eq!(addr(rx.driver.data.vaddr), addr(driver.rx.data.vaddr));
    assert_eq!(addr(rx.clients[0].queue.vaddr), RX_CLIENT_QUEUE_VADDR);
    assert_eq!(addr(rx.clients[0].data.vaddr), RX_CLIENT_DATA_VADDR);
    assert_eq!(client.tx.id, CLIENT_TX_CHANNEL);
    assert_eq!(client.rx.id, CLIENT_RX_CHANNEL);
    assert_eq!(
        addr(client.tx.queue.vaddr),
        addr(tx.clients[0].conn.queue.vaddr)
    );
    assert_eq!(
        addr(client.tx.data.vaddr),
        addr(tx.clients[0].conn.data.vaddr)
    );
    assert_eq!(addr(client.rx.queue.vaddr), addr(rx.clients[0].queue.vaddr));
    assert_eq!(addr(client.rx.data.vaddr), addr(rx.clients[0].data.vaddr));
    assert_ne!(driver.tx.id, tx.driver.id);
    assert_ne!(driver.rx.id, rx.driver.id);
}

#[test]
fn config_bytes_round_trip_the_magic_and_the_flags() {
    let driver = driver_config();
    let mut driver_bytes = vec![0u8; size_of::<serial_driver_config_t>()];
    serial_config_to_bytes(&driver, &mut driver_bytes);
    assert!(serial_config_check_magic(&driver_bytes));
    // SAFETY: the bytes were copied from a live `serial_driver_config_t`.
    let driver_back = unsafe { serial_config_from_bytes::<serial_driver_config_t>(&driver_bytes) };
    assert_eq!(driver_back.magic, driver.magic);
    assert_eq!(driver_back.default_baud, driver.default_baud);
    assert_eq!(driver_back.rx_enabled, driver.rx_enabled);
    assert_eq!(driver_back.tx.id, driver.tx.id);
    assert_eq!(
        addr(driver_back.tx.queue.vaddr),
        addr(driver.tx.queue.vaddr)
    );

    let tx = virt_tx_config();
    let mut tx_bytes = vec![0u8; size_of::<serial_virt_tx_config_t>()];
    serial_config_to_bytes(&tx, &mut tx_bytes);
    assert!(serial_config_check_magic(&tx_bytes));
    // SAFETY: the bytes were copied from a live `serial_virt_tx_config_t`.
    let tx_back = unsafe { serial_config_from_bytes::<serial_virt_tx_config_t>(&tx_bytes) };
    assert_eq!(tx_back.num_clients, 1);
    assert!(!tx_back.enable_colour);
    assert!(!tx_back.enable_rx);
    assert_eq!(tx_back.clients[0].name, tx.clients[0].name);

    let rx = virt_rx_config();
    let mut rx_bytes = vec![0u8; size_of::<serial_virt_rx_config_t>()];
    serial_config_to_bytes(&rx, &mut rx_bytes);
    // SAFETY: the bytes were copied from a live `serial_virt_rx_config_t`.
    let rx_back = unsafe { serial_config_from_bytes::<serial_virt_rx_config_t>(&rx_bytes) };
    assert_eq!(rx_back.switch_char, SWITCH_CHAR);
    assert_eq!(rx_back.terminate_num_char, TERMINATE_NUM_CHAR);

    let client = client_config();
    let mut client_bytes = vec![0u8; size_of::<serial_client_config_t>()];
    serial_config_to_bytes(&client, &mut client_bytes);
    // SAFETY: the bytes were copied from a live `serial_client_config_t`.
    let client_back = unsafe { serial_config_from_bytes::<serial_client_config_t>(&client_bytes) };
    assert_eq!(client_back.tx.id, CLIENT_TX_CHANNEL);
    assert_eq!(client_back.rx.id, CLIENT_RX_CHANNEL);
}
