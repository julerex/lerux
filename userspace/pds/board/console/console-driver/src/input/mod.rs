#[cfg(all(
    not(feature = "board-swift_sfx14_41g"),
    not(feature = "board-inspiron_n5010")
))]
pub(crate) mod hid;
pub(crate) mod scancode;
