//! The same rescue names, on a console that has no disk and no network.

use lerux_rescue::{owns, step, Flags, Role, Step, View};
use lerux_signon::PROMPT;

#[cfg(feature = "board-swift_sfx14_41g")]
const ROLES: &[Role<'static>] = &[
    Role {
        name: b"console_driver",
        priority: 6,
        budget_us: None,
    },
    Role {
        name: b"swift_pci_probe",
        priority: 5,
        budget_us: None,
    },
    Role {
        name: b"swift_mt7921",
        priority: 4,
        budget_us: None,
    },
    Role {
        name: b"swift_nvme",
        priority: 3,
        budget_us: None,
    },
    Role {
        name: b"swift_bt",
        priority: 2,
        budget_us: None,
    },
    Role {
        name: b"net_virt_tx",
        priority: 3,
        budget_us: None,
    },
    Role {
        name: b"net_virt_rx",
        priority: 3,
        budget_us: None,
    },
    Role {
        name: b"net_copy",
        priority: 2,
        budget_us: None,
    },
    Role {
        name: b"ram_blk",
        priority: 4,
        budget_us: None,
    },
    Role {
        name: b"blk_virt",
        priority: 3,
        budget_us: None,
    },
    Role {
        name: b"fatfs",
        priority: 2,
        budget_us: None,
    },
    Role {
        name: b"shell",
        priority: 1,
        budget_us: None,
    },
];

#[cfg(feature = "board-inspiron_n5010")]
const ROLES: &[Role<'static>] = &[
    Role {
        name: b"console_driver",
        priority: 2,
        budget_us: None,
    },
    Role {
        name: b"shell",
        priority: 1,
        budget_us: None,
    },
];

#[cfg(not(any(feature = "board-swift_sfx14_41g", feature = "board-inspiron_n5010")))]
const ROLES: &[Role<'static>] = &[
    Role {
        name: b"serial_driver",
        priority: 10,
        budget_us: None,
    },
    Role {
        name: b"console_driver",
        priority: 2,
        budget_us: None,
    },
    Role {
        name: b"shell",
        priority: 1,
        budget_us: None,
    },
];

#[cfg(feature = "board-swift_sfx14_41g")]
const PCI: &[&[u8]] = &[
    b"01:00.0 10de:1f9d",
    b"02:00.0 1c5c:174a",
    b"03:00.0 14c3:7961",
    b"04:00.0 1002:164c",
    b"04:00.3 1022:1639",
    b"04:00.4 1022:1638",
];

pub(crate) fn offer(shell: &mut super::HandlerImpl, line: &[u8]) -> bool {
    let Some((cmd, rest)) = split(line) else {
        return false;
    };
    if !owns(cmd) {
        return false;
    }
    let mut out = [0u8; 256];
    match step(cmd, rest, &view(), &mut out) {
        Step::Unknown => false,
        Step::Show(n) => {
            shell.emit(&out[..n]);
            shell.emit(PROMPT);
            true
        }
        _ => {
            miss(shell, cmd);
            true
        }
    }
}

fn view() -> View<'static> {
    View {
        arch: arch_name(),
        cores: 1,
        block: None,
        roles: ROLES,
        pci: pci_lines(),
        stack: 0x10_000,
        faults: 0,
        boot_ns: None,
        nvme_model: None,
        smart: None,
        mac: None,
        flags: Flags::new(),
    }
}

fn pci_lines() -> &'static [&'static [u8]] {
    #[cfg(feature = "board-swift_sfx14_41g")]
    {
        PCI
    }
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    {
        &[]
    }
}

fn arch_name() -> &'static [u8] {
    #[cfg(target_arch = "x86_64")]
    {
        b"x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        b"aarch64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        b"unknown"
    }
}

fn miss(shell: &mut super::HandlerImpl, cmd: &[u8]) {
    shell.emit(cmd);
    shell.emit(b" err\n");
    shell.emit(PROMPT);
}

fn split(line: &[u8]) -> Option<(&[u8], &[u8])> {
    if line.is_empty() {
        return None;
    }
    let Some(space) = line.iter().position(|byte| *byte == b' ') else {
        return Some((line, b""));
    };
    Some((&line[..space], trim(&line[space + 1..])))
}

fn trim(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|byte| *byte != b' ')
        .unwrap_or(line.len());
    &line[start..]
}
