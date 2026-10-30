//! Swift X SFX14-41G device logic that does not touch seL4.
//!
//! The protection domains paint with [`paint_row`] and call the parsers here.
//! A missing firmware blob or a controller that does not answer stays an error.
//! Nothing in this crate formats a disk or sends an NVMe write.

#![cfg_attr(not(test), no_std)]

mod bt;
mod crypto;
mod nvme;
mod panel;
mod pci;
mod wifi;

pub use bt::{
    acknowledge_reset, bring_up as bt_bring_up, find_bluetooth_interface, find_hci_interrupt,
    format_report as bt_line, frame_usb_event, further, hci_reset, is_bluetooth_interface,
    usb_hci_reset, version_line, xfer_line, BtReport, HciIntr, XferFail,
};
pub use nvme::{
    io_allowed, io_rejected, load_named, report_bytes, write_result, DiskReport, NAMED_FILE,
};
pub use panel::paint_row;
pub use pci::{
    class_code, config_address, describe, load_header, memory_bar, store_header, vendor_device,
    HEADER_BYTES, LOCATIONS, MT7921_BAR, MT7921_BDF, MT7921_IDS, NVME_BDF, NVME_CLASS, XHCI_BDFS,
    XHCI_CLASS,
};
pub use wifi::{bring_up, credentials, enqueue_frame, format_report, Nic, WifiReport};
