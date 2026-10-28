#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpelledAddr {
    text: &'static str,
    value: u64,
}

impl SpelledAddr {
    /// `text` is the system-description attribute, including its underscores.
    /// The integer is parsed from that text. A bad spelling fails const evaluation.
    pub const fn new(text: &'static str) -> Self {
        match parse_hex_spelled(text) {
            Some(value) => Self { text, value },
            None => panic!("invalid spelled address"),
        }
    }

    pub const fn text(self) -> &'static str {
        self.text
    }

    pub const fn value(self) -> u64 {
        self.value
    }
}

const fn parse_hex_spelled(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'0' || bytes[1] != b'x' {
        return None;
    }
    let mut i = 2usize;
    let mut value: u64 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        i += 1;
        if b == b'_' {
            continue;
        }
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u64,
            b'a'..=b'f' => (b - b'a' + 10) as u64,
            b'A'..=b'F' => (b - b'A' + 10) as u64,
            _ => return None,
        };
        let Some(next) = value.checked_mul(16) else {
            return None;
        };
        value = next;
        let Some(next) = value.checked_add(digit) else {
            return None;
        };
        value = next;
    }
    Some(value)
}

pub(crate) const AARCH64_VIRTIO_NET_OFFSET: u64 = 0xe00;
pub(crate) const AARCH64_VIRTIO_BLK_OFFSET: u64 = 0xc00;
pub(crate) const AARCH64_VIRTIO_GPU_OFFSET: u64 = 0xa00;

pub(crate) const RISCV64_VIRTIO_NET_DMA_GUEST: u64 = 0x40_000_000;
pub(crate) const RISCV64_VIRTIO_BLK_DMA_GUEST: u64 = 0x41_000_000;
pub(crate) const RISCV64_VIRTIO_GPU_DMA_GUEST: u64 = 0x42_000_000;

// OVMF on Q35 writes PCIEXBAR `0xe0000001` before the payload starts.
// QEMU's reset default `0xb0000000` is not decoded after that write.
pub const X86_PCI_ECAM_PHYS: SpelledAddr = SpelledAddr::new("0xe0000000");
pub const X86_PCI_ECAM_GUEST: u64 = 0xe000_0000;
const _: () = assert!(X86_PCI_ECAM_PHYS.value() == X86_PCI_ECAM_GUEST);
pub const X86_PCI_ECAM_SIZE: u64 = 0x100_000;
pub const X86_PCI_ECAM_WINDOW: &str = "pci_ecam";
pub const X86_PCI_ECAM_SETVAR: &str = "pci_ecam_vaddr";

pub(crate) const X86_BLK_DEVICE: u8 = 3;
pub(crate) const X86_BLK_FUNCTION: u8 = 0;
pub(crate) const X86_BLK_BAR0: SpelledAddr = SpelledAddr::new("0xfed0_0000");
pub(crate) const X86_BLK_BAR1: SpelledAddr = SpelledAddr::new("0xfed1_0000");

pub(crate) const X86_NET_DEVICE: u8 = 4;
pub(crate) const X86_NET_FUNCTION: u8 = 0;
pub(crate) const X86_NET_BAR0: SpelledAddr = SpelledAddr::new("0xfed2_0000");
pub(crate) const X86_NET_BAR1: SpelledAddr = SpelledAddr::new("0xfed3_0000");

pub(crate) const X86_VIRTIO_NET_DMA_PHYS: SpelledAddr = SpelledAddr::new("0x6000_0000");
pub(crate) const X86_VIRTIO_BLK_DMA_PHYS: SpelledAddr = SpelledAddr::new("0x6200_0000");

pub(crate) const X86_BLK_BAR_VALUES: [u64; 2] = [X86_BLK_BAR0.value(), X86_BLK_BAR1.value()];
pub(crate) const X86_NET_BAR_VALUES: [u64; 2] = [X86_NET_BAR0.value(), X86_NET_BAR1.value()];

/// Physical addresses and interrupt numbers for the system description.
/// Protection domains do not compile this module.
#[cfg(feature = "system-description")]
mod description_only {
    use super::SpelledAddr;

    pub(crate) const AARCH64_UART_PHYS: SpelledAddr = SpelledAddr::new("0x9_000_000");
    pub(crate) const AARCH64_UART_GUEST: u64 = 0x2_000_000;
    pub(crate) const AARCH64_UART_IRQ: u32 = 33;
    pub(crate) const AARCH64_UART_SETVAR: &str = "serial_register_block";

    pub(crate) const RISCV64_UART_PHYS: SpelledAddr = SpelledAddr::new("0x1_000_0000");
    pub(crate) const RISCV64_UART_GUEST: u64 = 0x2_000_000;
    pub(crate) const RISCV64_UART_IRQ: u32 = 10;
    pub(crate) const RISCV64_UART_SETVAR: &str = "serial_register_block";

    pub(crate) const X86_UART_PORT: SpelledAddr = SpelledAddr::new("0x3f8");
    pub(crate) const X86_UART_PORT_SIZE: u16 = 8;
    pub(crate) const X86_UART_ID_SYMBOL: &str = "com1_ioport_id";
    pub(crate) const X86_UART_ADDR_SYMBOL: &str = "com1_ioport_addr";
    pub(crate) const X86_UART_PIN: u8 = 4;
    pub(crate) const X86_UART_VECTOR: u8 = 48;

    pub(crate) const AARCH64_VIRTIO_WINDOW_PHYS: SpelledAddr = SpelledAddr::new("0xa003000");
    pub(crate) const AARCH64_VIRTIO_WINDOW_GUEST: u64 = 0x6_000_000_000;
    pub(crate) const AARCH64_VIRTIO_WINDOW_SIZE: u64 = 0x1_000;
    pub(crate) const AARCH64_VIRTIO_NET_IRQ: u32 = 79;
    pub(crate) const AARCH64_VIRTIO_BLK_IRQ: u32 = 78;
    pub(crate) const AARCH64_VIRTIO_GPU_IRQ: u32 = 77;

    pub(crate) const RISCV64_BLK_PHYS: SpelledAddr = SpelledAddr::new("0x10_001_000");
    pub(crate) const RISCV64_BLK_IRQ: u32 = 1;
    pub(crate) const RISCV64_BLK_BUS: u8 = 0;
    pub(crate) const RISCV64_NET_PHYS: SpelledAddr = SpelledAddr::new("0x10_002_000");
    pub(crate) const RISCV64_NET_IRQ: u32 = 2;
    pub(crate) const RISCV64_NET_BUS: u8 = 1;
    pub(crate) const RISCV64_VIRTIO_MMIO_OFFSET: u64 = 0;
    pub(crate) const RISCV64_GPU_PHYS: SpelledAddr = SpelledAddr::new("0x10_003_000");
    pub(crate) const RISCV64_GPU_IRQ: u32 = 3;
    pub(crate) const RISCV64_GPU_BUS: u8 = 2;

    pub(crate) const X86_PCI_CONFIG_PORT: SpelledAddr = SpelledAddr::new("0xcf8");
    pub(crate) const X86_PCI_CONFIG_SIZE: u16 = 8;
    pub(crate) const X86_PCI_CONFIG_ID_SYMBOL: &str = "pci_config_ioport_id";
    pub(crate) const X86_PCI_CONFIG_ADDR_SYMBOL: &str = "pci_config_ioport_addr";
    pub(crate) const X86_PCI_NET_CONFIG_ID_SYMBOL: &str = "pci_net_config_ioport_id";
    pub(crate) const X86_PCI_NET_CONFIG_ADDR_SYMBOL: &str = "pci_net_config_ioport_addr";

    pub(crate) const X86_BLK_PIN: u8 = 10;
    pub(crate) const X86_BLK_VECTOR: u8 = 49;
    pub(crate) const X86_NET_PIN: u8 = 11;
    pub(crate) const X86_NET_VECTOR: u8 = 50;

    pub(crate) const SERIAL_MMIO_WINDOW_NAME: &str = "serial_mmio";
    pub(crate) const SERIAL_MMIO_SIZE: u64 = 0x1_000;
    pub(crate) const AARCH64_VIRTIO_WINDOW_NAME: &str = "virtio_mmio";
}

#[cfg(feature = "system-description")]
pub(crate) use description_only::*;

#[cfg(test)]
mod tests {
    use super::SpelledAddr;

    #[test]
    fn spelled_addr_round_trip() {
        const ADDR: SpelledAddr = SpelledAddr::new("0x9_000_000");
        assert_eq!(ADDR.text(), "0x9_000_000");
        assert_eq!(ADDR.value(), 0x9_000_000);
    }
}
