#[cfg(all(not(test), feature = "hardware"))]
pub(crate) mod device;
#[cfg(all(not(test), feature = "hardware"))]
pub(crate) mod handler;
#[cfg(all(not(test), feature = "board-pc_z97_d3h_i217"))]
pub(crate) mod i217;
pub(crate) mod screen;
