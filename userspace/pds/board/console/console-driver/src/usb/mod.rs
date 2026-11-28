#[cfg(all(
    not(feature = "board-swift_sfx14_41g"),
    not(feature = "board-inspiron_n5010")
))]
pub(crate) mod usb;
#[cfg(all(
    not(test),
    feature = "hardware",
    any(feature = "board-swift_sfx14_41g", feature = "board-inspiron_n5010")
))]
pub(crate) mod usb_off;
#[cfg(all(
    not(test),
    feature = "hardware",
    not(feature = "board-swift_sfx14_41g"),
    not(feature = "board-inspiron_n5010")
))]
pub(crate) mod xhci;
