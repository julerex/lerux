//! Firmware scanout mode. The Multiboot tag and the mode record are different bytes.
// The verifier's expansion names `vstd` and `int`. A normal build erases that
// expansion, so the imports are unused unless `verus` is verifying.
// Protection-domain builds leave the feature off.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use verus_builtin::*;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

const ADDRESS_ROWS: u32 = 16;

/// 16 MiB. The 2026-10-29 Swift X panel is 1920 by 1080 with pitch 7680
/// (`8_294_400` bytes). Pitch 8192 at that size is `8_847_360` bytes and still fits.
const FRAMEBUFFER_CAP: u64 = 0x100_0000;

/// Physical page the trampoline fills with a [`ScanoutMode`] record.
pub const MODE_RECORD_PHYS: u64 = 0x01FE_0000;

/// Mapped panel buffer from the 2026-10-29 Swift X GOP line.
pub const FRAMEBUFFER_PHYS: u64 = 0x00FC_9000_0000;

pub const FRAMEBUFFER_BYTES: u64 = 0x7E_9000;

/// Pitch, width, and height of that line. `pitch * height` is [`FRAMEBUFFER_BYTES`].
pub const FRAMEBUFFER_PITCH: u32 = 7680;

pub const FRAMEBUFFER_WIDTH: u32 = 1920;

pub const FRAMEBUFFER_HEIGHT: u32 = 1080;

/// `fb 0x` followed by the 16 lowercase hex digits of [`FRAMEBUFFER_PHYS`].
pub open spec fn recorded_prefix_byte(index: int) -> u8 {
    if index == 0 {
        b'f'
    } else if index == 1 {
        b'b'
    } else if index == 2 {
        b' '
    } else if index == 3 {
        b'0'
    } else if index == 4 {
        b'x'
    } else if index == 5 {
        b'0'
    } else if index == 6 {
        b'0'
    } else if index == 7 {
        b'0'
    } else if index == 8 {
        b'0'
    } else if index == 9 {
        b'0'
    } else if index == 10 {
        b'0'
    } else if index == 11 {
        b'f'
    } else if index == 12 {
        b'c'
    } else if index == 13 {
        b'9'
    } else if index == 14 {
        b'0'
    } else if index == 15 {
        b'0'
    } else if index == 16 {
        b'0'
    } else if index == 17 {
        b'0'
    } else if index == 18 {
        b'0'
    } else if index == 19 {
        b'0'
    } else {
        b'0'
    }
}

/// Lowercase hex digit `shift` of `address`, `shift == 15` being the most significant.
pub open spec fn spec_hex_byte(address: u64, shift: u64) -> u8 {
    let nibble = ((address >> (shift * 4)) & 0xf) as u8;
    if nibble < 10 {
        (b'0' + nibble) as u8
    } else {
        (b'a' + (nibble - 10)) as u8
    }
}

/// Byte `index` of `fb 0x` and the 16 lowercase hex digits of `address`.
pub open spec fn spec_prefix_byte(address: u64, index: int) -> u8 {
    if index == 0 {
        b'f'
    } else if index == 1 {
        b'b'
    } else if index == 2 {
        b' '
    } else if index == 3 {
        b'0'
    } else if index == 4 {
        b'x'
    } else {
        spec_hex_byte(address, (20 - index) as u64)
    }
}

/// The mapped address is the one this image paints, and it is a page, not the mode record.
pub open spec fn spec_panel_ready(mode_address: u64, mapped_phys: u64) -> bool {
    mode_address != 0 && mode_address != MODE_RECORD_PHYS && mode_address == mapped_phys
        && mode_address % 4096 == 0
}

pub open spec fn recorded_tail(
    pitch: u32,
    width: u32,
    height: u32,
    red: u8,
    green: u8,
    blue: u8,
) -> bool {
    pitch == FRAMEBUFFER_PITCH && width == FRAMEBUFFER_WIDTH && height == FRAMEBUFFER_HEIGHT && red
        == 16 && green == 8 && blue == 0
}

/// `FRAMEBUFFER_PITCH * FRAMEBUFFER_HEIGHT` is the mapped byte count.
pub proof fn lemma_framebuffer_bytes()
    ensures
        (FRAMEBUFFER_PITCH as u64) * (FRAMEBUFFER_HEIGHT as u64) == FRAMEBUFFER_BYTES,
{
    assert((FRAMEBUFFER_PITCH as u64) * (FRAMEBUFFER_HEIGHT as u64) == FRAMEBUFFER_BYTES)
        by (compute);
}

/// The recorded address is page-aligned, nonzero, and not the mode-record page.
pub proof fn lemma_recorded_address_ready()
    ensures
        spec_panel_ready(FRAMEBUFFER_PHYS, FRAMEBUFFER_PHYS),
{
    assert(FRAMEBUFFER_PHYS != 0);
    assert(FRAMEBUFFER_PHYS != MODE_RECORD_PHYS);
    assert(FRAMEBUFFER_PHYS % 4096 == 0) by (compute);
}

/// The recorded prefix is `fb 0x` plus the lowercase hex digits of [`FRAMEBUFFER_PHYS`].
pub proof fn lemma_recorded_prefix_is_hex()
    ensures
        forall|i: int|
            #![auto]
            0 <= i < 21 ==> spec_prefix_byte(FRAMEBUFFER_PHYS, i) == recorded_prefix_byte(i),
{
    assert(spec_prefix_byte(FRAMEBUFFER_PHYS, 0) == recorded_prefix_byte(0) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        1,
    ) == recorded_prefix_byte(1) && spec_prefix_byte(FRAMEBUFFER_PHYS, 2) == recorded_prefix_byte(2)
        && spec_prefix_byte(FRAMEBUFFER_PHYS, 3) == recorded_prefix_byte(3) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        4,
    ) == recorded_prefix_byte(4) && spec_prefix_byte(FRAMEBUFFER_PHYS, 5) == recorded_prefix_byte(5)
        && spec_prefix_byte(FRAMEBUFFER_PHYS, 6) == recorded_prefix_byte(6) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        7,
    ) == recorded_prefix_byte(7) && spec_prefix_byte(FRAMEBUFFER_PHYS, 8) == recorded_prefix_byte(8)
        && spec_prefix_byte(FRAMEBUFFER_PHYS, 9) == recorded_prefix_byte(9) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        10,
    ) == recorded_prefix_byte(10) && spec_prefix_byte(FRAMEBUFFER_PHYS, 11) == recorded_prefix_byte(
        11,
    ) && spec_prefix_byte(FRAMEBUFFER_PHYS, 12) == recorded_prefix_byte(12) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        13,
    ) == recorded_prefix_byte(13) && spec_prefix_byte(FRAMEBUFFER_PHYS, 14) == recorded_prefix_byte(
        14,
    ) && spec_prefix_byte(FRAMEBUFFER_PHYS, 15) == recorded_prefix_byte(15) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        16,
    ) == recorded_prefix_byte(16) && spec_prefix_byte(FRAMEBUFFER_PHYS, 17) == recorded_prefix_byte(
        17,
    ) && spec_prefix_byte(FRAMEBUFFER_PHYS, 18) == recorded_prefix_byte(18) && spec_prefix_byte(
        FRAMEBUFFER_PHYS,
        19,
    ) == recorded_prefix_byte(19) && spec_prefix_byte(FRAMEBUFFER_PHYS, 20) == recorded_prefix_byte(
        20,
    )) by (compute);
    assert forall|i: int| #![auto] 0 <= i < 21 implies spec_prefix_byte(FRAMEBUFFER_PHYS, i)
        == recorded_prefix_byte(i) by {
        if i == 0 {
        } else if i == 1 {
        } else if i == 2 {
        } else if i == 3 {
        } else if i == 4 {
        } else if i == 5 {
        } else if i == 6 {
        } else if i == 7 {
        } else if i == 8 {
        } else if i == 9 {
        } else if i == 10 {
        } else if i == 11 {
        } else if i == 12 {
        } else if i == 13 {
        } else if i == 14 {
        } else if i == 15 {
        } else if i == 16 {
        } else if i == 17 {
        } else if i == 18 {
        } else if i == 19 {
        } else {
            assert(i == 20);
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mask {
    shift: u8,
    width: u8,
}

/// Linear 32-bit framebuffer described by firmware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanoutMode {
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    red: Mask,
    green: Mask,
    blue: Mask,
}

pub closed spec fn spec_address(mode: ScanoutMode) -> u64 {
    mode.address
}

pub closed spec fn spec_pitch(mode: ScanoutMode) -> u32 {
    mode.pitch
}

pub closed spec fn spec_width(mode: ScanoutMode) -> u32 {
    mode.width
}

pub closed spec fn spec_height(mode: ScanoutMode) -> u32 {
    mode.height
}

pub closed spec fn spec_bpp(mode: ScanoutMode) -> u8 {
    mode.bpp
}

pub closed spec fn spec_red_shift(mode: ScanoutMode) -> u8 {
    mode.red.shift
}

pub closed spec fn spec_red_width(mode: ScanoutMode) -> u8 {
    mode.red.width
}

pub closed spec fn spec_green_shift(mode: ScanoutMode) -> u8 {
    mode.green.shift
}

pub closed spec fn spec_green_width(mode: ScanoutMode) -> u8 {
    mode.green.width
}

pub closed spec fn spec_blue_shift(mode: ScanoutMode) -> u8 {
    mode.blue.shift
}

pub closed spec fn spec_blue_width(mode: ScanoutMode) -> u8 {
    mode.blue.width
}

/// The recorded address is the buffer this image maps, and it is not the mode page.
#[must_use]
pub fn panel_ready(mode_address: u64, mapped_phys: u64) -> (ready: bool)
    ensures
        ready == spec_panel_ready(mode_address, mapped_phys),
        ready ==> mode_address == mapped_phys && mode_address != 0 && mode_address
            != MODE_RECORD_PHYS && mode_address % 4096 == 0,
{
    let aligned = mode_address.is_multiple_of(4096);
    assert(aligned == (mode_address % 4096 == 0));
    mode_address != 0 && mode_address != MODE_RECORD_PHYS && mode_address == mapped_phys && aligned
}

/// Byte 0 of the Swift `browser_hold` page while the shell's browser owns the
/// panel. Microkit zeroes that page, so the console owns the panel at boot.
pub const BROWSER_HOLD: u8 = 1;

/// Geometry this image maps. seL4 clears the RAM frame at [`MODE_RECORD_PHYS`]
/// on retype, so a missing record still describes the panel.
#[must_use]
pub fn swift_recorded_mode() -> (mode: ScanoutMode)
    ensures
        spec_address(mode) == FRAMEBUFFER_PHYS,
        spec_pitch(mode) == FRAMEBUFFER_PITCH,
        spec_width(mode) == FRAMEBUFFER_WIDTH,
        spec_height(mode) == FRAMEBUFFER_HEIGHT,
        spec_bpp(mode) == 32,
        spec_red_shift(mode) == 16,
        spec_red_width(mode) == 8,
        spec_green_shift(mode) == 8,
        spec_green_width(mode) == 8,
        spec_blue_shift(mode) == 0,
        spec_blue_width(mode) == 8,
        spec_panel_ready(spec_address(mode), FRAMEBUFFER_PHYS),
        (spec_pitch(mode) as u64) * (spec_height(mode) as u64) == FRAMEBUFFER_BYTES,
{
    let mode = ScanoutMode {
        address: FRAMEBUFFER_PHYS,
        pitch: FRAMEBUFFER_PITCH,
        width: FRAMEBUFFER_WIDTH,
        height: FRAMEBUFFER_HEIGHT,
        bpp: 32,
        red: Mask { shift: 16, width: 8 },
        green: Mask { shift: 8, width: 8 },
        blue: Mask { shift: 0, width: 8 },
    };
    proof {
        lemma_framebuffer_bytes();
        lemma_recorded_address_ready();
    }
    mode
}

fn is_recorded_tail(pitch: u32, width: u32, height: u32, red: u8, green: u8, blue: u8) -> (yes:
    bool)
    ensures
        yes == recorded_tail(pitch, width, height, red, green, blue),
{
    pitch == FRAMEBUFFER_PITCH && width == FRAMEBUFFER_WIDTH && height == FRAMEBUFFER_HEIGHT && red
        == 16 && green == 8 && blue == 0
}

proof fn lemma_prefix_header(address: u64)
    ensures
        spec_prefix_byte(address, 0) == b'f',
        spec_prefix_byte(address, 1) == b'b',
        spec_prefix_byte(address, 2) == b' ',
        spec_prefix_byte(address, 3) == b'0',
        spec_prefix_byte(address, 4) == b'x',
{
}

proof fn lemma_prefix_slot(address: u64, index: int, shift: u64, byte: u8)
    requires
        5 <= index < 21,
        index + shift == 20,
        byte == spec_hex_byte(address, shift),
    ensures
        spec_prefix_byte(address, index) == byte,
{
    assert(shift == (20 - index) as u64);
}

fn hex_digit(address: u64, shift: u64) -> (byte: u8)
    requires
        shift <= 15,
    ensures
        byte == spec_hex_byte(address, shift),
{
    let masked = (address >> (shift * 4)) & 0xf;
    assert(masked < 16) by (bit_vector)
        requires
            shift <= 15,
            masked == (address >> (shift * 4)) & 0xf,
    ;
    let nibble = masked as u8;
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

fn write_address_prefix(out: &mut [u8; 80], address: u64)
    ensures
        forall|i: int| #![auto] 0 <= i < 21 ==> final(out)@[i] == spec_prefix_byte(address, i),
{
    out[0] = b'f';
    out[1] = b'b';
    out[2] = b' ';
    out[3] = b'0';
    out[4] = b'x';
    let mut shift: u64 = 16;
    let mut index: usize = 5;
    proof {
        lemma_prefix_header(address);
    }
    while shift > 0
        invariant
            5 <= index <= 21,
            shift <= 16,
            index as int + shift as int == 21,
            forall|j: int|
                #![auto]
                0 <= j < index as int ==> out@[j] == spec_prefix_byte(address, j),
        decreases shift,
    {
        shift -= 1;
        let byte = hex_digit(address, shift);
        proof {
            lemma_prefix_slot(address, index as int, shift, byte);
        }
        out[index] = byte;
        index += 1;
    }
}

/// The recorded tail is the spelling after the address. It does not touch the prefix.
fn write_recorded_tail(out: &mut [u8; 80], address: u64)
    requires
        forall|j: int| #![auto] 0 <= j < 21 ==> out@[j] == spec_prefix_byte(address, j),
    ensures
        forall|j: int| #![auto] 0 <= j < 21 ==> final(out)@[j] == spec_prefix_byte(address, j),
{
    // The address is named by the erased prefix invariant.
    let _ = address;
    let tail: &[u8; 31] = b" pitch 7680 1920x1080 r16 g8 b0";
    let mut n: usize = 0;
    while n < 31
        invariant
            n <= 31,
            forall|j: int| #![auto] 0 <= j < 21 ==> out@[j] == spec_prefix_byte(address, j),
        decreases 31 - n,
    {
        let idx = 21 + n;
        out[idx] = tail[n];
        n += 1;
    }
}

/// Formatter for every mode except the recorded Swift geometry.
///
/// The body is the existing formatter. [`mode_line`]'s postcondition does not
/// mention this return: it only constrains the recorded geometry.
#[verifier::external_body]
fn general_mode_line(
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    red: u8,
    green: u8,
    blue: u8,
) -> (out: Option<[u8; 80]>) {
    let mut out = [0u8;80];
    let mut len = 0usize;
    if !push(&mut out, &mut len, b"fb 0x") {
        return None;
    }
    for shift in (0..16).rev() {
        let nibble = ((address >> (shift * 4)) & 0xf) as u8;
        let byte = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
        if !push(&mut out, &mut len, &[byte]) {
            return None;
        }
    }
    if !push(&mut out, &mut len, b" pitch ") || !push_u32(&mut out, &mut len, pitch) || !push(
        &mut out,
        &mut len,
        b" ",
    ) || !push_u32(&mut out, &mut len, width) || !push(&mut out, &mut len, b"x") || !push_u32(
        &mut out,
        &mut len,
        height,
    ) || !push(&mut out, &mut len, b" r") || !push_u32(&mut out, &mut len, u32::from(red)) || !push(
        &mut out,
        &mut len,
        b" g",
    ) || !push_u32(&mut out, &mut len, u32::from(green)) || !push(&mut out, &mut len, b" b")
        || !push_u32(&mut out, &mut len, u32::from(blue)) {
        return None;
    }
    Some(out)
}

/// Same spelling as `fb_mode_line` in `support/mb2-stub/fb_paint.c`.
///
/// The recorded geometry takes the proved prefix path. Every other mode uses
/// [`general_mode_line`], which is the same formatter.
#[must_use]
pub fn mode_line(
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    red: u8,
    green: u8,
    blue: u8,
) -> (out: Option<[u8; 80]>)
    ensures
        recorded_tail(pitch, width, height, red, green, blue) ==> out is Some && forall|i: int|
            #![auto]
            0 <= i < 21 ==> out->Some_0@[i] == spec_prefix_byte(address, i),
{
    if is_recorded_tail(pitch, width, height, red, green, blue) {
        let mut line = [0u8;80];
        write_address_prefix(&mut line, address);
        write_recorded_tail(&mut line, address);
        Some(line)
    } else {
        general_mode_line(address, pitch, width, height, red, green, blue)
    }
}

/// The recorded mode's line starts with `fb 0x` and the lowercase hex address.
///
/// The host test is the exec caller. The lib build keeps the function so the
/// verifier still checks this spelling.
#[must_use]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn recorded_mode_line() -> (line: [u8; 80])
    ensures
        forall|i: int| #![auto] 0 <= i < 21 ==> line@[i] == spec_prefix_byte(FRAMEBUFFER_PHYS, i),
        forall|i: int| #![auto] 0 <= i < 21 ==> line@[i] == recorded_prefix_byte(i),
{
    let mode = swift_recorded_mode();
    let out = mode_line(
        mode.address,
        mode.pitch,
        mode.width,
        mode.height,
        mode.red.shift,
        mode.green.shift,
        mode.blue.shift,
    );
    proof {
        lemma_recorded_prefix_is_hex();
    }
    if let Some(line) = out {
        line
    } else {
        assert(false);
        [0u8;80]
    }
}

} // verus!
const _: () = assert!(FRAMEBUFFER_PITCH as u64 * FRAMEBUFFER_HEIGHT as u64 == FRAMEBUFFER_BYTES);

/// Mode to paint. A record that names another address stays off. A cleared
/// record uses [`swift_recorded_mode`].
#[must_use]
pub fn panel_mode(record: &[u8]) -> Option<ScanoutMode> {
    if let Some(mode) = ScanoutMode::parse(record) {
        let bytes = u64::from(mode.pitch()).saturating_mul(u64::from(mode.height()));
        return (panel_ready(mode.address(), FRAMEBUFFER_PHYS) && bytes <= FRAMEBUFFER_BYTES)
            .then_some(mode);
    }
    Some(swift_recorded_mode())
}

fn push(out: &mut [u8; 80], len: &mut usize, bytes: &[u8]) -> bool {
    let Some(next) = len.checked_add(bytes.len()) else {
        return false;
    };
    if next >= out.len() {
        return false;
    }
    out[*len..next].copy_from_slice(bytes);
    *len = next;
    true
}

fn push_u32(out: &mut [u8; 80], len: &mut usize, value: u32) -> bool {
    let mut tmp = [0u8; 10];
    let mut n = 0usize;
    let mut rest = value;
    if rest == 0 {
        return push(out, len, b"0");
    }
    while rest > 0 {
        tmp[n] = b'0' + (rest % 10) as u8;
        n += 1;
        rest /= 10;
    }
    while n > 0 {
        n -= 1;
        if !push(out, len, &tmp[n..=n]) {
            return false;
        }
    }
    true
}

const RECORD_LEN: usize = 40;
const MODE_MAGIC: u64 = 0x4D4F_4445_4C52_5558;

impl ScanoutMode {
    /// Accept a 40-byte mode record written by [`Self::write_record`].
    #[must_use]
    pub fn parse(record: &[u8]) -> Option<Self> {
        let record = record.get(..RECORD_LEN)?;
        if read_u64(record, 0)? != MODE_MAGIC {
            return None;
        }
        validated(&Parts {
            address: read_u64(record, 8)?,
            pitch: read_u32(record, 16)?,
            width: read_u32(record, 20)?,
            height: read_u32(record, 24)?,
            bpp: read_u8(record, 28)?,
            red_shift: read_u8(record, 29)?,
            red_width: read_u8(record, 30)?,
            green_shift: read_u8(record, 31)?,
            green_width: read_u8(record, 32)?,
            blue_shift: read_u8(record, 33)?,
            blue_width: read_u8(record, 34)?,
        })
    }

    /// Write the 40-byte record [`Self::parse`] accepts.
    ///
    /// `None` means `out` is shorter than that record.
    pub fn write_record(self, out: &mut [u8]) -> Option<()> {
        let out = out.get_mut(..RECORD_LEN)?;
        out.fill(0);
        write_u64(out, 0, MODE_MAGIC);
        write_u64(out, 8, self.address);
        write_u32(out, 16, self.pitch);
        write_u32(out, 20, self.width);
        write_u32(out, 24, self.height);
        out[28] = self.bpp;
        out[29] = self.red.shift;
        out[30] = self.red.width;
        out[31] = self.green.shift;
        out[32] = self.green.width;
        out[33] = self.blue.shift;
        out[34] = self.blue.width;
        Some(())
    }

    #[must_use]
    pub fn address(self) -> u64 {
        self.address
    }

    #[must_use]
    pub fn pitch(self) -> u32 {
        self.pitch
    }

    #[must_use]
    pub fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn bits_per_pixel(self) -> u8 {
        self.bpp
    }

    #[must_use]
    pub fn red_shift(self) -> u8 {
        self.red.shift
    }

    #[must_use]
    pub fn green_shift(self) -> u8 {
        self.green.shift
    }

    #[must_use]
    pub fn blue_shift(self) -> u8 {
        self.blue.shift
    }

    #[must_use]
    pub fn red_width(self) -> u8 {
        self.red.width
    }

    #[must_use]
    pub fn green_width(self) -> u8 {
        self.green.width
    }

    #[must_use]
    pub fn blue_width(self) -> u8 {
        self.blue.width
    }
}

/// Multiboot 2 framebuffer tag, type 8, including its 8-byte header.
///
/// Framebuffer type 1 is the linear direct-color mode. A shorter slice or any
/// other framebuffer type is rejected.
#[must_use]
pub fn mode_from_mb2_tag(tag: &[u8]) -> Option<ScanoutMode> {
    let kind = read_u32(tag, 0)?;
    let size = usize::try_from(read_u32(tag, 4)?).ok()?;
    if kind != 8 || size < RECORD_LEN || tag.len() < size {
        return None;
    }
    // 0 is indexed and 2 is text. Only type 1 is linear direct color.
    if read_u8(tag, 29)? != 1 {
        return None;
    }
    validated(&Parts {
        address: read_u64(tag, 8)?,
        pitch: read_u32(tag, 16)?,
        width: read_u32(tag, 20)?,
        height: read_u32(tag, 24)?,
        bpp: read_u8(tag, 28)?,
        red_shift: read_u8(tag, 32)?,
        red_width: read_u8(tag, 33)?,
        green_shift: read_u8(tag, 34)?,
        green_width: read_u8(tag, 35)?,
        blue_shift: read_u8(tag, 36)?,
        blue_width: read_u8(tag, 37)?,
    })
}

struct Parts {
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    red_shift: u8,
    red_width: u8,
    green_shift: u8,
    green_width: u8,
    blue_shift: u8,
    blue_width: u8,
}

fn validated(parts: &Parts) -> Option<ScanoutMode> {
    if parts.height < ADDRESS_ROWS || parts.bpp != 32 {
        return None;
    }
    let min_pitch = parts.width.checked_mul(4)?;
    if parts.pitch < min_pitch {
        return None;
    }
    let bytes = u64::from(parts.pitch).checked_mul(u64::from(parts.height))?;
    if bytes > FRAMEBUFFER_CAP || !parts.address.is_multiple_of(4096) {
        return None;
    }
    Some(ScanoutMode {
        address: parts.address,
        pitch: parts.pitch,
        width: parts.width,
        height: parts.height,
        bpp: parts.bpp,
        red: Mask {
            shift: parts.red_shift,
            width: parts.red_width,
        },
        green: Mask {
            shift: parts.green_shift,
            width: parts.green_width,
        },
        blue: Mask {
            shift: parts.blue_shift,
            width: parts.blue_width,
        },
    })
}

fn read_u8(bytes: &[u8], at: usize) -> Option<u8> {
    bytes.get(at).copied()
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    let buf: [u8; 4] = slice.try_into().ok()?;
    Some(u32::from_le_bytes(buf))
}

fn read_u64(bytes: &[u8], at: usize) -> Option<u64> {
    let slice = bytes.get(at..at + 8)?;
    let buf: [u8; 8] = slice.try_into().ok()?;
    Some(u64::from_le_bytes(buf))
}

fn write_u32(out: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = out.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn write_u64(out: &mut [u8], at: usize, value: u64) {
    if let Some(slot) = out.get_mut(at..at + 8) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG: &[u8] = &[
        0x08, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd0, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x20, 0x01,
        0x00, 0x00, 0x10, 0x08, 0x08, 0x08, 0x00, 0x08, 0x00, 0x00,
    ];

    fn assert_recorded(mode: ScanoutMode) {
        assert_eq!(mode.address(), 0xd000_0000);
        assert_eq!(mode.pitch(), 4096);
        assert_eq!(mode.width(), 1024);
        assert_eq!(mode.height(), 768);
        assert_eq!(mode.bits_per_pixel(), 32);
        assert_eq!(mode.red_shift(), 16);
        assert_eq!(mode.green_shift(), 8);
        assert_eq!(mode.blue_shift(), 0);
        assert_eq!(mode.red_width(), 8);
        assert_eq!(mode.green_width(), 8);
        assert_eq!(mode.blue_width(), 8);
    }

    #[test]
    fn recorded_tag_parses() {
        let mode = mode_from_mb2_tag(TAG).expect("the recorded tag is a linear mode");
        assert_recorded(mode);
    }

    #[test]
    fn truncated_or_nonlinear_tag_fails() {
        assert!(
            mode_from_mb2_tag(&TAG[..TAG.len() - 1]).is_none(),
            "a truncated tag fails"
        );
        let mut indexed = [0u8; TAG.len()];
        indexed.copy_from_slice(TAG);
        indexed[29] = 0;
        assert!(
            mode_from_mb2_tag(&indexed).is_none(),
            "a non-linear type fails"
        );
        indexed.copy_from_slice(TAG);
        indexed[29] = 2;
        assert!(
            mode_from_mb2_tag(&indexed).is_none(),
            "a text framebuffer type fails"
        );
    }

    #[test]
    fn swift_pitch_fits() {
        let mut tag = [0u8; TAG.len()];
        tag.copy_from_slice(TAG);
        tag[8..16].copy_from_slice(&0x8000_0000u64.to_le_bytes());
        tag[16..20].copy_from_slice(&8192u32.to_le_bytes());
        tag[20..24].copy_from_slice(&1920u32.to_le_bytes());
        tag[24..28].copy_from_slice(&1080u32.to_le_bytes());
        let mode = mode_from_mb2_tag(&tag).expect("1920x1080 pitch 8192 still fits");
        assert_eq!(mode.width(), 1920);
        assert_eq!(mode.height(), 1080);
        assert_eq!(mode.pitch(), 8192);
        let bytes = u64::from(mode.pitch()) * u64::from(mode.height());
        assert!(bytes <= FRAMEBUFFER_CAP);
        assert!(bytes > FRAMEBUFFER_BYTES);
    }

    #[test]
    fn swift_recorded_mode_fits() {
        let mut tag = [0u8; TAG.len()];
        tag.copy_from_slice(TAG);
        tag[8..16].copy_from_slice(&FRAMEBUFFER_PHYS.to_le_bytes());
        tag[16..20].copy_from_slice(&7680u32.to_le_bytes());
        tag[20..24].copy_from_slice(&1920u32.to_le_bytes());
        tag[24..28].copy_from_slice(&1080u32.to_le_bytes());
        let mode = mode_from_mb2_tag(&tag).expect("the 2026-10-29 Swift mode fits");
        assert_eq!(mode.address(), FRAMEBUFFER_PHYS);
        assert_eq!(mode.pitch(), 7680);
        assert_eq!(mode.width(), 1920);
        assert_eq!(mode.height(), 1080);
        assert_eq!(mode.red_shift(), 16);
        assert_eq!(mode.green_shift(), 8);
        assert_eq!(mode.blue_shift(), 0);
        let bytes = u64::from(mode.pitch()) * u64::from(mode.height());
        assert_eq!(bytes, 8_294_400);
        assert_eq!(bytes, FRAMEBUFFER_BYTES);
        assert!(panel_ready(mode.address(), FRAMEBUFFER_PHYS));
        let line = mode_line(
            mode.address(),
            mode.pitch(),
            mode.width(),
            mode.height(),
            mode.red_shift(),
            mode.green_shift(),
            mode.blue_shift(),
        )
        .expect("line fits");
        assert_eq!(recorded_mode_line(), line);
        let text = line.split(|byte| *byte == 0).next().expect("text");
        assert_eq!(
            std::str::from_utf8(text).expect("utf8"),
            "fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0"
        );
    }

    #[test]
    fn mode_line_matches_the_stub() {
        let line = mode_line(0xd000_0000, 4096, 1024, 768, 16, 8, 0).expect("line fits");
        let rust = line.split(|byte| *byte == 0).next().expect("text");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let out_dir = root.join("build/host");
        std::fs::create_dir_all(&out_dir).expect("host build dir");
        let bin = out_dir.join("fb-paint-test");
        let status = std::process::Command::new("clang")
            .current_dir(&root)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "support/mb2-stub/fb_paint.c",
                "support/mb2-stub/fb_paint_test.c",
                "-o",
            ])
            .arg(&bin)
            .status();
        let status = status.expect("clang");
        assert!(status.success(), "fb paint host build");
        let printed = std::process::Command::new(&bin)
            .output()
            .expect("run fb paint test");
        assert!(printed.status.success(), "fb paint test failed");
        let text = std::str::from_utf8(&printed.stdout).expect("utf8");
        assert_eq!(
            text.trim_end(),
            std::str::from_utf8(rust).expect("line utf8")
        );
    }

    #[test]
    fn unrecorded_framebuffer_does_not_paint() {
        assert!(!panel_ready(MODE_RECORD_PHYS, FRAMEBUFFER_PHYS));
        assert!(!panel_ready(0, 0));
        assert!(panel_ready(0x8000_0000, 0x8000_0000));
    }

    #[test]
    fn cleared_mode_record_paints_the_mapped_panel() {
        let mode = panel_mode(&[0; RECORD_LEN]).expect("a cleared record still paints");
        assert_eq!(mode, swift_recorded_mode());
        assert_eq!(mode.address(), FRAMEBUFFER_PHYS);
        assert_eq!(mode.pitch(), FRAMEBUFFER_PITCH);
        let mut record = [0u8; RECORD_LEN];
        mode.write_record(&mut record)
            .expect("the mode record is 40 bytes");
        assert_eq!(panel_mode(&record), Some(mode));
        record[8..16].copy_from_slice(&0x8000_0000u64.to_le_bytes());
        assert!(panel_mode(&record).is_none());
    }

    #[test]
    fn mode_record_round_trips() {
        let mode = mode_from_mb2_tag(TAG).expect("the recorded tag is a linear mode");
        assert!(
            ScanoutMode::parse(TAG).is_none(),
            "the multiboot tag is not the mode record"
        );
        let mut record = [0u8; RECORD_LEN];
        mode.write_record(&mut record)
            .expect("the mode record is 40 bytes");
        let again = ScanoutMode::parse(&record).expect("a written mode record parses");
        assert_recorded(again);
        assert_eq!(again, mode);
    }
}
