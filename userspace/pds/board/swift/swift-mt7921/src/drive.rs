//! `Mac` against the MT7921 model. The board build does not include this.

use lerux_swift::{
    mt7921::{read_survey_count, read_survey_mark, read_survey_ssid, Stage},
    mt7921_card::{
        patch_image, ram_image, Air, Card, BAR_LEN, DMA_LEN, DMA_PHYS, NEIGHBOR_SSID, OTHER_SSID,
        STA_MAC,
    },
};

use crate::{
    device::{Images, Mac},
    hook,
};

fn air() -> Air {
    Air::new(
        [0x02, 0x00, 0x00, 0x00, 0x00, 0x11],
        b"lerux",
        6,
        b"password",
    )
    .expect("air")
}

fn firmware() -> Images {
    Images {
        patch: Box::leak(Box::new(patch_image())),
        patch_1a: &[],
        ram: Box::leak(Box::new(ram_image())),
        ram_1a: &[],
    }
}

fn boot(regs: &mut [u8], dma: &mut [u8]) -> (hook::Session, Mac) {
    // Safety: the slices outlive `Mac` and the installed card. Nothing else aliases them.
    let card = unsafe { Card::bind(regs.as_mut_ptr(), dma.as_mut_ptr(), air()) };
    let session = hook::install(card);
    let mac = Mac::boot(regs.as_mut_ptr(), dma.as_mut_ptr(), &firmware());
    (session, mac)
}

#[test]
fn driver_reaches_wait_and_hears_the_survey() {
    assert_eq!(DMA_PHYS, crate::device::DMA_PHYS);
    let mut regs = vec![0u8; BAR_LEN];
    let mut dma = vec![0u8; DMA_LEN];
    let (_session, mut mac) = boot(&mut regs, &mut dma);
    assert_eq!(mac.stage(), Stage::Wait);
    assert_eq!(mac.sta(), STA_MAC);
    assert_eq!(hook::fw_bytes(), 8);
    let mut page = [0u8; 0x1000];
    mac.survey(&mut page);
    assert_eq!(read_survey_count(&page), Some(3));
    assert_eq!(read_survey_ssid(&page, 0), Some(&b"lerux"[..]));
    assert_eq!(read_survey_ssid(&page, 1), Some(NEIGHBOR_SSID));
    assert_eq!(read_survey_ssid(&page, 2), Some(OTHER_SSID));
    let mark = b"rx 1 ev 2 ok 3 done 1 ch 21 bcn 15";
    assert_eq!(read_survey_mark(&page), Some(&mark[..]));
}

#[test]
fn driver_associates_with_the_model_ap() {
    let mut regs = vec![0u8; BAR_LEN];
    let mut dma = vec![0u8; DMA_LEN];
    let (_session, mut mac) = boot(&mut regs, &mut dma);
    assert_eq!(mac.stage(), Stage::Wait);
    let stage = mac.associate(b"lerux", b"password", |_| {});
    assert_eq!(stage, Stage::Wait);
    assert!(mac.link_mark().is_some());
}
