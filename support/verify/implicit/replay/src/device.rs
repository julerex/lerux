use crate::scan;

pub fn flags() -> Result<Vec<(&'static str, bool)>, String> {
    Ok(vec![("CopyChecks", sites()?.into_iter().all(|(_, ok)| ok))])
}

pub fn behavior() -> Result<(), String> {
    for (name, ok) in sites()? {
        if !ok {
            return Err(name.to_string());
        }
    }
    Ok(())
}

fn sites() -> Result<Vec<(&'static str, bool)>, String> {
    let e1000 = scan::read("userspace/pds/sddf-e1000e-driver/src/engine.rs")?;
    let genet = scan::read("userspace/pds/genet-driver/src/genet.rs")?;
    let nvme = scan::read("userspace/pds/swift-nvme/src/main.rs")?;
    let ahci = scan::read("userspace/pds/sddf-ahci-driver/src/engine.rs")?;
    let copy_rx = scan::one(&e1000, "copy_rx")?;
    let reclaim = scan::one(&e1000, "reclaim_tx")?;
    let transmit = scan::one(&genet, "transmit")?;
    let receive = scan::one(&genet, "receive")?;
    let poll = scan::one(&nvme, "poll")?;
    let dma = scan::one(&ahci, "run_dma")?;
    Ok(vec![
        (
            "e1000e copy_rx no longer waits for RX_DD",
            copy_rx.contains("header.status & RX_DD == 0"),
        ),
        (
            "e1000e reclaim_tx no longer waits for TX_DD",
            reclaim.contains("header.status & TX_DD == 0"),
        ),
        (
            "GENET transmit no longer waits while DMA_OWN is set",
            transmit.contains("stat & DMA_OWN != 0"),
        ),
        (
            "GENET receive no longer waits while DMA_OWN is set",
            receive.contains("stat & DMA_OWN != 0"),
        ),
        (
            "NVMe poll no longer compares the completion phase",
            poll.contains("(dw3 >> 16) & 1 == phase"),
        ),
        (
            "AHCI run_dma no longer waits for poll_pxci",
            dma.contains("if !poll_pxci"),
        ),
    ])
}
