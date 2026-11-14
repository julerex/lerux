//! One image slot shared by `web_content` and `image_decoder`.
//!
//! `web_content` writes the encoded bytes and waits. `image_decoder` decodes
//! that one image, writes the pixels, notifies, and waits again before it
//! accepts another. Microkit zeroes the control page, and state 0 is idle.

use core::mem::{align_of, size_of};

/// The slot is idle. A zeroed control page starts here.
pub const IMAGE_STATE_IDLE: u32 = 0;
/// `web_content` has written the encoded bytes and is waiting.
pub const IMAGE_STATE_ASK: u32 = 1;
/// The pixel region holds `width * height` packed B, G, R, A pixels.
pub const IMAGE_STATE_OK: u32 = 2;
/// The bytes were not one accepted PNG. `width` and `height` are 0.
pub const IMAGE_STATE_ERR: u32 = 3;

/// Encoded bytes. A longer image is refused.
pub const ENCODED_CAP: usize = 0x1000;

/// Decoded pixels. A larger picture is an error.
pub const PIXEL_CAP: usize = 0x1000;

/// One Microkit page. The struct sits at the start.
pub const IMAGE_REGION_SIZE: usize = 0x1000;

/// System-description name of the control page.
pub const IMAGE_REGION: &str = "ladybird_image";

/// System-description name of the encoded-byte region.
pub const ENCODED_REGION: &str = "ladybird_encoded";

/// System-description name of the pixel region.
pub const PIXEL_REGION: &str = "ladybird_pixels";

/// Virtual address of the control page on `qemu_virt_aarch64_ladybird`.
///
/// The body region ends at this address.
pub const IMAGE_VADDR: u64 = 0x9_031_000;

/// Virtual address of the encoded bytes. It follows the control page.
pub const ENCODED_VADDR: u64 = 0x9_032_000;

/// Virtual address of the decoded pixels. It follows the encoded bytes.
pub const PIXEL_VADDR: u64 = 0x9_033_000;

/// Image notification on `web_content`. Paint is 0 and the request is 1.
pub const WEB_CONTENT_IMAGE_CHANNEL: u8 = 2;

/// Image notification on `image_decoder`.
pub const IMAGE_DECODER_CHANNEL: u8 = 0;

const _: () = assert!(IMAGE_REGION_SIZE == 0x1000);
const _: () = assert!(ENCODED_CAP == 0x1000);
const _: () = assert!(PIXEL_CAP == 0x1000);
const _: () = assert!(ENCODED_VADDR == IMAGE_VADDR + IMAGE_REGION_SIZE as u64);
const _: () = assert!(PIXEL_VADDR == ENCODED_VADDR + ENCODED_CAP as u64);
const _: () = assert!(size_of::<ImagePage>() <= IMAGE_REGION_SIZE);
const _: () = assert!(align_of::<ImagePage>() <= 4);

/// Control page. The encoded bytes and the pixels are different regions.
///
/// `state` is stored by the caller after a release fence, so the lengths are
/// visible before the state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImagePage {
    pub state: u32,
    pub width: u32,
    pub height: u32,
    pub encoded_len: u32,
}

const _: () = assert!(size_of::<ImagePage>() == 16);
