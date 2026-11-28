//! The debugging dump below the probe lines on the Swift panel.
//!
//! The panel is 1920 by 1080 and a cell is 16 pixels, so the text grid is
//! [`PANEL_COLS`] by [`PANEL_ROWS`]. The console owns rows 0 to 24, the six
//! probe lines rows 25 to 30, the dump rows 31 to 65, and the end marker row
//! 66. The console fill and `Damage::All` cover rows 0 to 24, so every dump
//! row survives a redraw.
//!
//! A hex row is an eight-character label, then twelve `xxxxxxxx` words. A
//! named row is a three-character tag, then seven `name value` pairs. Both
//! stay inside [`PANEL_COLS`]. The protection domains pad each row to the
//! full width so a second paint erases the first.

/// Text columns at 16 pixels per cell on a 1920-pixel panel.
pub const PANEL_COLS: usize = 120;
/// Text rows at 16 pixels per cell on a 1080-pixel panel. Row 66 is the last whole cell.
pub const PANEL_ROWS: usize = 67;
/// 32-bit words on one hex row.
pub const WORDS_PER_ROW: usize = 12;
/// Bytes one hex row covers.
pub const ROW_BYTES: usize = WORDS_PER_ROW * 4;
/// Characters in a hex row label.
pub const LABEL_LEN: usize = 8;
/// `name value` pairs on one named row.
pub const NAMES_PER_ROW: usize = 7;
/// Characters in a register name. Shorter names are padded.
pub const NAME_LEN: usize = 5;
/// Bytes of one PCI configuration space the probe dumps.
pub const CONFIG_BYTES: usize = 256;
/// Hex rows one configuration space needs. The sixth row holds four words.
pub const CONFIG_ROWS: usize = CONFIG_BYTES.div_ceil(ROW_BYTES);

/// Row 31: the Wi-Fi function, its root port, and the `NVMe` root port.
pub const ROW_SUMMARY: usize = 31;
/// Rows 32 to 37: configuration space of `03:00.0`.
pub const ROW_WIFI_CONFIG: usize = 32;
/// Rows 38 to 43: configuration space of the root port above bus 3.
pub const ROW_WIFI_PORT: usize = ROW_WIFI_CONFIG + CONFIG_ROWS;
/// Rows 44 to 49: configuration space of the root port above bus 2.
pub const ROW_NVME_PORT: usize = ROW_WIFI_PORT + CONFIG_ROWS;
/// Row 50: AMD IOMMU registers.
pub const ROW_IOMMU: usize = ROW_NVME_PORT + CONFIG_ROWS;
/// Rows 51 to 53: MT7921 registers read after the firmware step returned.
pub const ROW_REGS_AFTER: usize = ROW_IOMMU + 1;
/// Rows 54 and 55: transmit rings 0, 16, 17, then receive rings 0, 1, 2.
pub const ROW_RINGS: usize = ROW_REGS_AFTER + 3;
/// Rows 56 to 58: the prefetch registers at `0xd4600`, `0xd4630`, `0xd4680`.
pub const ROW_PREFETCH: usize = ROW_RINGS + 2;
/// Row 59: slots 0 to 2 of the MCU transmit ring.
pub const ROW_TX_DESC: usize = ROW_PREFETCH + 3;
/// Rows 60 and 61: the first 68 bytes of the bounce buffer.
pub const ROW_BOUNCE: usize = ROW_TX_DESC + 1;
/// Row 62: slots 0 to 2 of the event receive ring.
pub const ROW_RX_DESC: usize = ROW_BOUNCE + 2;
/// Rows 63 to 65: MT7921 registers read before the first MCU command.
pub const ROW_REGS_BEFORE: usize = ROW_RX_DESC + 1;
/// Row 66, the last whole cell: the end marker. A photograph that shows it
/// shows every row above it.
pub const ROW_END: usize = ROW_REGS_BEFORE + 3;

const _: () = assert!(ROW_END + 1 == PANEL_ROWS);
const _: () = assert!(LABEL_LEN + WORDS_PER_ROW * 9 <= PANEL_COLS);
const _: () = assert!(4 + NAMES_PER_ROW * (NAME_LEN + 10) <= PANEL_COLS);

/// PCI Express link control bits 1:0: ASPM L0s and L1.
pub const LINK_ASPM: u16 = 0x0003;
/// PCI Express link control bit 8: clock power management (CLKREQ).
pub const LINK_CLKPM: u16 = 0x0100;
/// What Linux `mt76_pci_disable_aspm` clears on the endpoint through
/// `pci_disable_link_state(L0S | L1 | CLKPM)`.
pub const ENDPOINT_LINK_CLEAR: u16 = LINK_ASPM | LINK_CLKPM;
/// What that call clears on the parent root port.
pub const PORT_LINK_CLEAR: u16 = LINK_ASPM;

/// The end marker. `END OF OUTPUT`, then dashes to the last column, which is
/// `|`, so the photograph shows the bottom row and the right edge together.
#[must_use]
pub fn write_end_row(out: &mut [u8]) -> usize {
    const TEXT: &[u8] = b"END OF OUTPUT ";
    if out.len() < PANEL_COLS {
        return 0;
    }
    out[..TEXT.len()].copy_from_slice(TEXT);
    out[TEXT.len()..PANEL_COLS - 1].fill(b'-');
    out[PANEL_COLS - 1] = b'|';
    PANEL_COLS
}

/// `+000    ` style label for a configuration row past the first.
#[must_use]
pub fn offset_label(offset: usize) -> [u8; LABEL_LEN] {
    let mut label = [b' '; LABEL_LEN];
    label[0] = b'+';
    label[1] = hex(nibble(offset, 8));
    label[2] = hex(nibble(offset, 4));
    label[3] = hex(nibble(offset, 0));
    label
}

/// `03:00.0 ` style label for the first configuration row.
#[must_use]
pub fn bdf_label(bus: u8, dev: u8, func: u8) -> [u8; LABEL_LEN] {
    let mut label = [b' '; LABEL_LEN];
    label[0] = hex(bus >> 4);
    label[1] = hex(bus);
    label[2] = b':';
    label[3] = hex(dev >> 4);
    label[4] = hex(dev);
    label[5] = b'.';
    label[6] = b'0' + (func & 0xf);
    label
}

/// `000d4600` style label: the register or physical address of the first word.
#[must_use]
pub fn address_label(address: u32) -> [u8; LABEL_LEN] {
    let mut label = [b' '; LABEL_LEN];
    write_hex32(&mut label, address);
    label
}

/// One hex row: the label, then up to twelve words separated by spaces.
/// Returns the characters written, or 0 when `out` is too short.
#[must_use]
pub fn write_hex_row(out: &mut [u8], label: &[u8], words: &[u32]) -> usize {
    let words = &words[..words.len().min(WORDS_PER_ROW)];
    let n = LABEL_LEN + words.len() * 9;
    if out.len() < n {
        return 0;
    }
    out[..LABEL_LEN].fill(b' ');
    let copy = label.len().min(LABEL_LEN);
    out[..copy].copy_from_slice(&label[..copy]);
    for (i, word) in words.iter().enumerate() {
        let at = LABEL_LEN + i * 9;
        out[at] = b' ';
        write_hex32(&mut out[at + 1..at + 9], *word);
    }
    n
}

/// One named row: a three-character tag, then `name value` pairs.
/// `names` and `values` are read in step; the shorter one bounds the row.
#[must_use]
pub fn write_named_row(out: &mut [u8], tag: &[u8], names: &[&[u8]], values: &[u32]) -> usize {
    let count = names.len().min(values.len()).min(NAMES_PER_ROW);
    let n = 3 + count * (NAME_LEN + 10);
    if out.len() < n {
        return 0;
    }
    out[..3].fill(b' ');
    let copy = tag.len().min(3);
    out[..copy].copy_from_slice(&tag[..copy]);
    for (i, (name, value)) in names.iter().zip(values).enumerate().take(count) {
        let at = 3 + i * (NAME_LEN + 10);
        out[at] = b' ';
        out[at + 1..at + 1 + NAME_LEN].fill(b' ');
        let copy = name.len().min(NAME_LEN);
        out[at + 1..at + 1 + copy].copy_from_slice(&name[..copy]);
        out[at + 1 + NAME_LEN] = b' ';
        write_hex32(&mut out[at + 2 + NAME_LEN..at + 10 + NAME_LEN], *value);
    }
    n
}

/// One 64-bit register as `name xxxxxxxxxxxxxxxx `. Returns the width used.
#[must_use]
pub fn write_wide(out: &mut [u8], name: &[u8], value: u64) -> usize {
    let n = name.len() + 1 + 16 + 1;
    if out.len() < n {
        return 0;
    }
    out[..name.len()].copy_from_slice(name);
    out[name.len()] = b' ';
    write_hex32(&mut out[name.len() + 1..name.len() + 9], high32(value));
    write_hex32(&mut out[name.len() + 9..name.len() + 17], low32(value));
    out[n - 1] = b' ';
    n
}

/// Little-endian words of a configuration space, for [`write_hex_row`].
pub fn config_words(config: &[u8], offset: usize, out: &mut [u32]) -> usize {
    let mut n = 0;
    for (i, slot) in out.iter_mut().enumerate() {
        let at = offset + i * 4;
        let Some(bytes) = config.get(at..at + 4) else {
            break;
        };
        *slot = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        n += 1;
    }
    n
}

/// Header type 1 with the multi-function bit masked off.
#[must_use]
pub fn is_bridge(config: &[u8]) -> bool {
    config.len() > 0x0e && config[0x0e] & 0x7f == 0x01 && vendor_present(config)
}

/// Secondary bus number of a type 1 header.
#[must_use]
pub fn secondary_bus(config: &[u8]) -> Option<u8> {
    if !is_bridge(config) || config.len() <= 0x19 {
        return None;
    }
    Some(config[0x19])
}

/// The 16-bit command word at offset 4.
#[must_use]
pub fn command_word(config: &[u8]) -> Option<u16> {
    read_u16(config, 0x04).filter(|_| vendor_present(config))
}

/// Offset of the first capability with this id, walking the list at `0x34`.
#[must_use]
pub fn find_capability(config: &[u8], id: u8) -> Option<usize> {
    if config.len() < 0x40 || config[0x06] & 0x10 == 0 {
        return None;
    }
    let mut ptr = usize::from(config[0x34] & !0x3);
    for _ in 0..48 {
        if ptr < 0x40 || ptr + 2 > config.len() {
            return None;
        }
        if config[ptr] == id {
            return Some(ptr);
        }
        ptr = usize::from(config[ptr + 1] & !0x3);
    }
    None
}

/// Byte offset of the PCI Express link control register. The capability
/// pointer is dword aligned, so this offset is too, and link control is the
/// low half of that dword. Link status is the high half.
#[must_use]
pub fn link_control_offset(config: &[u8]) -> Option<usize> {
    find_capability(config, 0x10).map(|cap| cap + 0x10)
}

/// PCI Express link control: the ASPM field is bits 1:0.
#[must_use]
pub fn link_control(config: &[u8]) -> Option<u16> {
    read_u16(config, link_control_offset(config)?)
}

/// Power management control and status: the power state is bits 1:0.
#[must_use]
pub fn power_control(config: &[u8]) -> Option<u16> {
    let cap = find_capability(config, 0x01)?;
    read_u16(config, cap + 0x04)
}

/// `03:00.0 cmd 0006 lnk 0142>0040 pm 0008` for one function. `link_before`
/// is the link control word read before the probe cleared ASPM on it; `None`
/// leaves the `>` and the second word out.
#[must_use]
pub fn write_function_summary(
    out: &mut [u8],
    bdf: (u8, u8, u8),
    config: &[u8],
    link_before: Option<u16>,
) -> usize {
    let n = 7 + 9 + readback_width(5, link_before) + 8;
    if out.len() < n {
        return 0;
    }
    let (bus, dev, func) = bdf;
    out[..7].copy_from_slice(&bdf_label(bus, dev, func)[..7]);
    write_field(&mut out[7..16], b" cmd ", command_word(config));
    let at = 16 + write_readback(&mut out[16..], b" lnk ", link_before, link_control(config));
    write_field(&mut out[at..at + 8], b" pm ", power_control(config));
    at + 8
}

/// `rp 00:02.2 sec 03 cmd 0407>0407 lnk 0042>0040` for a root port, or
/// `rp miss`. `tag` is `rp` for the Wi-Fi port and `np` for the `NVMe` port.
/// `before` is the command word read before bus master was set on the port,
/// and `link_before` the link control word read before ASPM was cleared.
#[must_use]
pub fn write_port_summary(out: &mut [u8], tag: &[u8; 2], port: Option<PortReadback<'_>>) -> usize {
    let Some(port) = port else {
        let mut miss = *b"rp miss";
        miss[..2].copy_from_slice(tag);
        return write_bytes(out, &miss);
    };
    let n = 10 + 7 + readback_width(5, port.before) + readback_width(5, port.link_before);
    if out.len() < n {
        return 0;
    }
    let (bus, dev, func) = port.bdf;
    out[..2].copy_from_slice(tag);
    out[2] = b' ';
    out[3..10].copy_from_slice(&bdf_label(bus, dev, func)[..7]);
    out[10..15].copy_from_slice(b" sec ");
    out[15] = hex(secondary_bus(port.config).unwrap_or(0xff) >> 4);
    out[16] = hex(secondary_bus(port.config).unwrap_or(0xff));
    let mut at = 17;
    at += write_readback(
        &mut out[at..],
        b" cmd ",
        port.before,
        command_word(port.config),
    );
    at += write_readback(
        &mut out[at..],
        b" lnk ",
        port.link_before,
        link_control(port.config),
    );
    at
}

/// A root port the probe found: its location, its configuration space after
/// the probe's writes, the command word before bus master was set, and the
/// link control word before ASPM was cleared.
#[derive(Clone, Copy)]
pub struct PortReadback<'a> {
    pub bdf: (u8, u8, u8),
    pub config: &'a [u8],
    pub before: Option<u16>,
    pub link_before: Option<u16>,
}

/// Width of [`write_readback`] for a name of `name_len` characters.
fn readback_width(name_len: usize, before: Option<u16>) -> usize {
    name_len + if before.is_some() { 9 } else { 4 }
}

/// ` name bbbb>aaaa` when `before` is known, ` name aaaa` otherwise.
fn write_readback(out: &mut [u8], name: &[u8], before: Option<u16>, after: Option<u16>) -> usize {
    let Some(before) = before else {
        write_field(&mut out[..name.len() + 4], name, after);
        return name.len() + 4;
    };
    write_field(&mut out[..name.len() + 4], name, Some(before));
    out[name.len() + 4] = b'>';
    write_hex16_opt(&mut out[name.len() + 5..name.len() + 9], after);
    name.len() + 9
}

fn write_field(out: &mut [u8], name: &[u8], value: Option<u16>) {
    out[..name.len()].copy_from_slice(name);
    write_hex16_opt(&mut out[name.len()..name.len() + 4], value);
}

fn write_hex16_opt(out: &mut [u8], value: Option<u16>) {
    match value {
        Some(value) => {
            for (i, byte) in out.iter_mut().enumerate().take(4) {
                *byte = hex(nibble(usize::from(value), 12 - 4 * i));
            }
        }
        None => out[..4].copy_from_slice(b"----"),
    }
}

fn write_bytes(out: &mut [u8], text: &[u8]) -> usize {
    if out.len() < text.len() {
        return 0;
    }
    out[..text.len()].copy_from_slice(text);
    text.len()
}

fn vendor_present(config: &[u8]) -> bool {
    read_u16(config, 0).is_some_and(|vendor| vendor != 0xffff && vendor != 0)
}

fn read_u16(config: &[u8], at: usize) -> Option<u16> {
    let bytes = config.get(at..at + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn write_hex32(slot: &mut [u8], value: u32) {
    for (i, byte) in slot.iter_mut().enumerate().take(8) {
        *byte = hex(nibble(value as usize, 28 - 4 * i));
    }
}

fn nibble(value: usize, shift: usize) -> u8 {
    u8::try_from((value >> shift) & 0xf).expect("nibble")
}

fn high32(value: u64) -> u32 {
    u32::try_from(value >> 32).expect("high word")
}

fn low32(value: u64) -> u32 {
    u32::try_from(value & 0xffff_ffff).expect("low word")
}

fn hex(nibble: u8) -> u8 {
    let nibble = nibble & 0xf;
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wifi_config() -> [u8; CONFIG_BYTES] {
        let mut config = [0u8; CONFIG_BYTES];
        config[0..2].copy_from_slice(&0x14c3u16.to_le_bytes());
        config[2..4].copy_from_slice(&0x7961u16.to_le_bytes());
        config[4..6].copy_from_slice(&0x0006u16.to_le_bytes());
        config[6] = 0x10;
        config[0x0e] = 0x00;
        config[0x34] = 0x80;
        config[0x80] = 0x01;
        config[0x81] = 0x90;
        config[0x84..0x86].copy_from_slice(&0x0008u16.to_le_bytes());
        config[0x90] = 0x10;
        config[0x91] = 0x00;
        config[0xa0..0xa2].copy_from_slice(&0x0042u16.to_le_bytes());
        config
    }

    fn port_config() -> [u8; CONFIG_BYTES] {
        let mut config = [0u8; CONFIG_BYTES];
        config[0..2].copy_from_slice(&0x1022u16.to_le_bytes());
        config[2..4].copy_from_slice(&0x1634u16.to_le_bytes());
        config[4..6].copy_from_slice(&0x0407u16.to_le_bytes());
        config[6] = 0x10;
        config[0x0e] = 0x81;
        config[0x18] = 0;
        config[0x19] = 3;
        config[0x1a] = 3;
        config[0x34] = 0x58;
        config[0x58] = 0x10;
        config[0x59] = 0x00;
        config[0x68..0x6a].copy_from_slice(&0x0043u16.to_le_bytes());
        config
    }

    #[test]
    fn rows_fill_the_panel_below_the_probe_lines() {
        assert_eq!(ROW_SUMMARY, 31);
        assert_eq!(ROW_WIFI_CONFIG, 32);
        assert_eq!(ROW_WIFI_PORT, 38);
        assert_eq!(ROW_NVME_PORT, 44);
        assert_eq!(ROW_IOMMU, 50);
        assert_eq!(ROW_REGS_AFTER, 51);
        assert_eq!(ROW_RINGS, 54);
        assert_eq!(ROW_PREFETCH, 56);
        assert_eq!(ROW_TX_DESC, 59);
        assert_eq!(ROW_BOUNCE, 60);
        assert_eq!(ROW_RX_DESC, 62);
        assert_eq!(ROW_REGS_BEFORE, 63);
        assert_eq!(ROW_END, 66);
        assert_eq!(CONFIG_ROWS, 6);
    }

    #[test]
    fn end_row_fills_the_last_row_to_the_right_edge() {
        let mut out = [0u8; PANEL_COLS];
        let n = write_end_row(&mut out);
        assert_eq!(n, PANEL_COLS);
        assert!(out.starts_with(b"END OF OUTPUT ---"));
        assert!(out.ends_with(b"---|"));
        assert!(out[..PANEL_COLS - 1].iter().all(|byte| *byte != b'|'));
        let mut short = [0u8; PANEL_COLS - 1];
        assert_eq!(write_end_row(&mut short), 0);
    }

    #[test]
    fn hex_row_is_a_label_and_twelve_words() {
        let mut out = [0u8; PANEL_COLS];
        let words = [0x01f6_2000u32, 0x4004_0000, 0, 0];
        let n = write_hex_row(&mut out, &address_label(0x01f6_2000), &words);
        assert_eq!(&out[..n], b"01f62000 01f62000 40040000 00000000 00000000");
        let words = [0xffff_ffffu32; 13];
        let n = write_hex_row(&mut out, b"+030", &words);
        assert_eq!(n, LABEL_LEN + 12 * 9);
        assert!(n <= PANEL_COLS);
        assert_eq!(&out[..9], b"+030     ");
        assert_eq!(&out[n - 8..n], b"ffffffff");
    }

    #[test]
    fn hex_row_refuses_a_short_buffer() {
        let mut out = [0u8; 16];
        assert_eq!(write_hex_row(&mut out, b"x", &[1, 2]), 0);
    }

    #[test]
    fn config_labels_name_the_function_and_the_offset() {
        assert_eq!(&bdf_label(3, 0, 0), b"03:00.0 ");
        assert_eq!(&bdf_label(0, 2, 2), b"00:02.2 ");
        assert_eq!(&offset_label(0x30), b"+030    ");
        assert_eq!(&offset_label(0xf0), b"+0f0    ");
        assert_eq!(&address_label(0xd4600), b"000d4600");
    }

    #[test]
    fn named_row_pads_names_to_five() {
        let mut out = [0u8; PANEL_COLS];
        let names: [&[u8]; 3] = [b"lpctl", b"glo", b"ext0"];
        let n = write_named_row(&mut out, b"aft", &names, &[0x4, 0x7000_9075, 0]);
        assert_eq!(
            &out[..n],
            b"aft lpctl 00000004 glo   70009075 ext0  00000000"
        );
        let names: [&[u8]; 8] = [b"a"; 8];
        let n = write_named_row(&mut out, b"pre", &names, &[0; 8]);
        assert_eq!(n, 3 + NAMES_PER_ROW * (NAME_LEN + 10));
        assert!(n <= PANEL_COLS);
    }

    #[test]
    fn wide_register_prints_sixteen_digits() {
        let mut out = [0u8; 32];
        let n = write_wide(&mut out, b"ctrl", 0x0000_0001_0000_0400);
        assert_eq!(&out[..n], b"ctrl 0000000100000400 ");
    }

    #[test]
    fn config_words_stop_at_the_end() {
        let mut config = [0u8; 8];
        config[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        config[4..8].copy_from_slice(&0x9abc_def0u32.to_le_bytes());
        let mut words = [0u32; 4];
        assert_eq!(config_words(&config, 0, &mut words), 2);
        assert_eq!(&words[..2], &[0x1234_5678, 0x9abc_def0]);
        assert_eq!(config_words(&config, 4, &mut words), 1);
    }

    #[test]
    fn bridge_fields_come_from_a_type_one_header() {
        let port = port_config();
        assert!(is_bridge(&port));
        assert_eq!(secondary_bus(&port), Some(3));
        assert_eq!(command_word(&port), Some(0x0407));
        assert_eq!(link_control(&port), Some(0x0043));
        let wifi = wifi_config();
        assert!(!is_bridge(&wifi));
        assert_eq!(secondary_bus(&wifi), None);
        assert_eq!(find_capability(&wifi, 0x10), Some(0x90));
        assert_eq!(link_control_offset(&wifi), Some(0xa0));
        assert_eq!(link_control(&wifi), Some(0x0042));
        assert_eq!(link_control_offset(&port), Some(0x68));
        assert_eq!(power_control(&wifi), Some(0x0008));
        assert_eq!(command_word(&[0xff; 64]), None);
        assert_eq!(link_control_offset(&[0u8; CONFIG_BYTES]), None);
    }

    #[test]
    fn link_clear_masks_match_linux() {
        assert_eq!(ENDPOINT_LINK_CLEAR, 0x0103);
        assert_eq!(PORT_LINK_CLEAR, 0x0003);
        assert_eq!(0x0142 & !ENDPOINT_LINK_CLEAR, 0x0040);
        assert_eq!(0x0042 & !PORT_LINK_CLEAR, 0x0040);
    }

    #[test]
    fn summaries_fit_one_row_together() {
        let mut out = [b' '; PANEL_COLS];
        let wifi = wifi_config();
        let port = port_config();
        let n = write_function_summary(&mut out, (3, 0, 0), &wifi, Some(0x0142));
        assert_eq!(&out[..n], b"03:00.0 cmd 0006 lnk 0142>0042 pm 0008");
        out[n] = b'|';
        let readback = PortReadback {
            bdf: (0, 2, 2),
            config: &port,
            before: Some(0x0403),
            link_before: Some(0x0043),
        };
        let m = write_port_summary(&mut out[n + 1..], b"rp", Some(readback));
        assert_eq!(
            &out[n + 1..n + 1 + m],
            b"rp 00:02.2 sec 03 cmd 0403>0407 lnk 0043>0043"
        );
        out[n + 1 + m] = b'|';
        let untouched = PortReadback {
            bdf: (0, 2, 1),
            config: &port,
            before: None,
            link_before: None,
        };
        let k = write_port_summary(&mut out[n + 2 + m..], b"np", Some(untouched));
        assert_eq!(
            &out[n + 2 + m..n + 2 + m + k],
            b"np 00:02.1 sec 03 cmd 0407 lnk 0043"
        );
        assert_eq!(n + 2 + m + k, PANEL_COLS);
        let k = write_port_summary(&mut out, b"np", None);
        assert_eq!(&out[..k], b"np miss");
    }

    #[test]
    fn summary_without_a_link_readback_is_the_old_shape() {
        let mut out = [b' '; PANEL_COLS];
        let wifi = wifi_config();
        let n = write_function_summary(&mut out, (3, 0, 0), &wifi, None);
        assert_eq!(&out[..n], b"03:00.0 cmd 0006 lnk 0042 pm 0008");
    }

    #[test]
    fn missing_capability_prints_dashes() {
        let mut out = [0u8; 40];
        let bare = [0x22u8, 0x10, 0x34, 0x16, 0x07, 0x04, 0, 0];
        let n = write_function_summary(&mut out, (0, 2, 1), &bare, None);
        assert_eq!(&out[..n], b"00:02.1 cmd 0407 lnk ---- pm ----");
        let n = write_function_summary(&mut out, (0, 2, 1), &bare, Some(0x0042));
        assert_eq!(&out[..n], b"00:02.1 cmd 0407 lnk 0042>---- pm ----");
    }
}
