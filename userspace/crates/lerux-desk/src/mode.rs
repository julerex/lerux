//! Firmware scanout mode. The Multiboot tag and the mode record are different bytes.

const ADDRESS_ROWS: u32 = 16;
const FRAMEBUFFER_CAP: u64 = 0x80_0000;
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
