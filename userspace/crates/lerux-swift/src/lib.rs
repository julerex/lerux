//! Swift X SFX14-41G device logic that does not touch seL4.
//!
//! The protection domains paint with [`paint_row`] and call the parsers here.
//! A missing firmware blob or a controller that does not answer stays an error.
//! Nothing in this crate formats a disk or sends an NVMe write.

#![cfg_attr(not(test), no_std)]

mod bt;
mod crypto;
pub mod mt7921;
mod nvme;
mod panel;
mod pci;
mod wifi;
pub mod wpa;

pub use bt::{
    acknowledge_reset, bring_up as bt_bring_up, fail_line, find_bluetooth_interface,
    find_hci_interrupt, first_interface_class, format_report as bt_line, frame_usb_event, further,
    hci_reset, is_bluetooth_interface, usb_hci_reset, version_line, xfer_line, BtReport, HciIntr,
    PortStop, XferFail,
};
pub use mt7921::{read_key, write_key, FirmwareKind, Stage};
pub use nvme::{
    io_allowed, io_rejected, load_named, report_bytes, write_result, DiskReport, NAMED_FILE,
};
pub use panel::paint_row;
pub use pci::{
    bar_line, class_code, config_address, describe, load_header, memory_bar, store_header,
    vendor_device, HEADER_BYTES, LOCATIONS, MT7921_BAR, MT7921_BDF, MT7921_IDS, NVME_BDF,
    NVME_CLASS, XHCI_BDFS, XHCI_CLASS,
};
pub use wifi::{
    bring_up, credentials, dhcp_router, dhcp_yiaddr, enqueue_frame, format_report, icmp_reply,
    write_discover, write_echo, write_request, Nic, WifiReport, CLIENT_MAC,
};
pub use wpa::key_ok;
