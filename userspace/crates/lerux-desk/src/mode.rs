//! Firmware scanout mode. The Multiboot tag and the mode record are different bytes.

const ADDRESS_ROWS: u32 = 16;
/// 16 MiB. The 2026-10-29 Swift X panel is 1920 by 1080 with pitch 7680
/// (8_294_400 bytes). Pitch 8192 at that size is 8_847_360 bytes and still fits.
const FRAMEBUFFER_CAP: u64 = 0x100_0000;

/// Physical page the trampoline fills with a [`ScanoutMode`] record.
pub const MODE_RECORD_PHYS: u64 = 0x01FE_0000;
/// Mapped panel buffer from the 2026-10-29 Swift X GOP line.
pub const FRAMEBUFFER_PHYS: u64 = 0x00FC_9000_0000;
pub const FRAMEBUFFER_BYTES: u64 = 0x7E_9000;

/// The recorded address is the buffer this image maps, and it is not the mode page.
pub fn panel_ready(mode_address: u64, mapped_phys: u64) -> bool {
    mode_address != 0
        && mode_address != MODE_RECORD_PHYS
        && mode_address == mapped_phys
        && mode_address.is_multiple_of(4096)
}

/// Same spelling as `fb_mode_line` in `support/mb2-stub/fb_paint.c`.
pub fn mode_line(
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    red: u8,
    green: u8,
    blue: u8,
) -> Option<[u8; 80]> {
    let mut out = [0u8; 80];
    let mut len = 0usize;
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
    if !push(&mut out, &mut len, b" pitch ")
        || !push_u32(&mut out, &mut len, pitch)
        || !push(&mut out, &mut len, b" ")
        || !push_u32(&mut out, &mut len, width)
        || !push(&mut out, &mut len, b"x")
        || !push_u32(&mut out, &mut len, height)
        || !push(&mut out, &mut len, b" r")
        || !push_u32(&mut out, &mut len, u32::from(red))
        || !push(&mut out, &mut len, b" g")
        || !push_u32(&mut out, &mut len, u32::from(green))
        || !push(&mut out, &mut len, b" b")
        || !push_u32(&mut out, &mut len, u32::from(blue))
    {
        return None;
    }
    Some(out)
}
const RECORD_LEN: usize = 40;
const MODE_MAGIC: u64 = 0x4D4F_4445_4C52_5558;

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

impl ScanoutMode {
    /// Accept a 40-byte mode record written by [`Self::write_record`].
    pub fn parse(record: &[u8]) -> Option<Self> {
        let record = record.get(..RECORD_LEN)?;
        if read_u64(record, 0)? != MODE_MAGIC {
            return None;
        }
        validated(Parts {
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

    pub fn address(self) -> u64 {
        self.address
    }

    pub fn pitch(self) -> u32 {
        self.pitch
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }

    pub fn bits_per_pixel(self) -> u8 {
        self.bpp
    }

    pub fn red_shift(self) -> u8 {
        self.red.shift
    }

    pub fn green_shift(self) -> u8 {
        self.green.shift
    }

    pub fn blue_shift(self) -> u8 {
        self.blue.shift
    }

    pub fn red_width(self) -> u8 {
        self.red.width
    }

    pub fn green_width(self) -> u8 {
        self.green.width
    }

    pub fn blue_width(self) -> u8 {
        self.blue.width
    }
}

/// Multiboot 2 framebuffer tag, type 8, including its 8-byte header.
///
/// Framebuffer type 1 is the linear direct-color mode. A shorter slice or any
/// other framebuffer type is rejected.
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
    validated(Parts {
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

fn validated(parts: Parts) -> Option<ScanoutMode> {
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
