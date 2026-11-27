//! Milestone 14 command names. A missing device prints the name and `err`.

use lerux_rescue::{owns, step, Flags, Role, Step, View};

#[cfg(not(any(
    feature = "board-pc_z97_d3h_i217",
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
use lerux_posix::Open;

const ROLE_SLOTS: usize = 16;
/// Swift template `disk_result`. The shell map is read-only and has no setvar.
#[cfg(feature = "board-swift_sfx14_41g")]
const DISK_RESULT: usize = 0x3ffe_0000;

#[cfg(feature = "board-swift_sfx14_41g")]
const SWIFT_ROLES: &[Role<'static>] = &[
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

const SWIFT_PCI: &[&[u8]] = &[
    b"01:00.0 10de:1f9d",
    b"02:00.0 1c5c:174a",
    b"03:00.0 14c3:7961",
    b"04:00.0 1002:164c",
    b"04:00.3 1022:1639",
    b"04:00.4 1022:1638",
];
const Z97_PCI: &[&[u8]] = &[b"00:19.0 8086:153b"];
const NATIVE_PCI: &[&[u8]] = &[b"00:1f.2 8086:2922", b"00:04.0 8086:10d3"];
const AHCI_PCI: &[&[u8]] = &[b"00:1f.2 8086:2922"];

pub(crate) fn command(cmd: &[u8], rest: &[u8]) -> bool {
    if !owns(cmd) {
        return false;
    }
    let mut roles = [Role {
        name: b"",
        priority: 0,
        budget_us: None,
    }; ROLE_SLOTS];
    let count = fill_roles(&mut roles);
    let view = make_view(&roles[..count]);
    let mut out = [0u8; 512];
    apply(step(cmd, rest, &view, &mut out), cmd, &out)
}

fn apply(found: Step<'_>, cmd: &[u8], out: &[u8]) -> bool {
    match found {
        Step::Unknown => false,
        Step::Absent => {
            miss(cmd);
            true
        }
        Step::Show(n) => {
            super::say(&out[..n]);
            true
        }
        Step::Probe(addr, port) => {
            probe(addr, port);
            true
        }
        Step::Clone(src, dst) => {
            clone_files(src, dst);
            true
        }
        Step::Fuser(name) => {
            fuser_name(name);
            true
        }
        Step::Journal(filter) => {
            journal_emit(filter);
            true
        }
        Step::Link(mac) => {
            show_link(mac);
            true
        }
    }
}

fn miss(cmd: &[u8]) {
    let mut out = [0u8; 64];
    let n = cmd.len().min(48);
    out[..n].copy_from_slice(&cmd[..n]);
    let tail = b" err\n";
    out[n..n + tail.len()].copy_from_slice(tail);
    super::say(&out[..n + tail.len()]);
}

fn make_view<'a>(roles: &'a [Role<'a>]) -> View<'a> {
    View {
        arch: arch_name(),
        cores: 1,
        block: block_name(),
        roles,
        pci: pci_lines(),
        stack: u32::try_from(super::STACK_SIZE).expect("stack fits"),
        faults: 0,
        boot_ns: super::boot_ns(),
        nvme_model: nvme_model(),
        smart: smart_bytes(),
        mac: mac_addr(),
        flags: view_flags(),
    }
}

fn view_flags() -> Flags {
    Flags::new()
        .mounted(mounted_fat())
        .net(cfg!(feature = "net-client"))
        .log(has_log())
        .daily(true)
        .second(false)
}

const fn has_log() -> bool {
    !cfg!(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))
}

fn fill_roles(out: &mut [Role<'static>]) -> usize {
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf"
    ))]
    {
        fill_schedule(out)
    }
    #[cfg(feature = "board-swift_sfx14_41g")]
    {
        copy_roles(SWIFT_ROLES, out)
    }
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_fs_sddf",
        feature = "board-qemu_virt_riscv64_fs_sddf",
        feature = "board-x86_64_generic_fs_sddf",
        feature = "board-swift_sfx14_41g"
    )))]
    {
        let _ = out;
        0
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn copy_roles(src: &'static [Role<'static>], out: &mut [Role<'static>]) -> usize {
    let n = src.len().min(out.len());
    out[..n].copy_from_slice(&src[..n]);
    n
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn fill_schedule(out: &mut [Role<'static>]) -> usize {
    let page = super::schedule();
    let n = usize::from(page.count).min(out.len());
    for (slot, entry) in out.iter_mut().zip(page.entry.iter()).take(n) {
        *slot = role_of(entry);
    }
    n
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fs_sddf",
    feature = "board-qemu_virt_riscv64_fs_sddf",
    feature = "board-x86_64_generic_fs_sddf"
))]
fn role_of(entry: &'static lerux_sddf::sched_entry_t) -> Role<'static> {
    let len = entry
        .name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(entry.name.len());
    Role {
        name: &entry.name[..len],
        priority: entry.priority,
        budget_us: Some(entry.budget_us),
    }
}

fn arch_name() -> &'static [u8] {
    #[cfg(target_arch = "aarch64")]
    {
        b"aarch64"
    }
    #[cfg(target_arch = "x86_64")]
    {
        b"x86_64"
    }
    #[cfg(target_arch = "riscv64")]
    {
        b"riscv64"
    }
    #[cfg(not(any(
        target_arch = "aarch64",
        target_arch = "x86_64",
        target_arch = "riscv64"
    )))]
    {
        b"unknown"
    }
}

fn block_name() -> Option<&'static [u8]> {
    if cfg!(feature = "board-swift_sfx14_41g") {
        Some(b"ram")
    } else if cfg!(feature = "board-x86_64_generic_fetch_native")
        || cfg!(feature = "board-x86_64_generic_fs_ahci")
    {
        Some(b"ahci")
    } else if virtio_block() {
        Some(b"virtio-blk")
    } else {
        None
    }
}

const fn virtio_block() -> bool {
    !cfg!(feature = "board-swift_sfx14_41g")
        && !cfg!(feature = "board-x86_64_generic_fetch_native")
        && !cfg!(feature = "board-x86_64_generic_fs_ahci")
        && (cfg!(feature = "board-qemu_virt_aarch64_fs_sddf")
            || cfg!(feature = "board-qemu_virt_riscv64_fs_sddf")
            || cfg!(feature = "board-x86_64_generic_fs_sddf")
            || cfg!(feature = "board-qemu_virt_aarch64_fetch_sddf")
            || cfg!(feature = "board-qemu_virt_riscv64_fetch_sddf")
            || cfg!(feature = "board-x86_64_generic_fetch_sddf")
            || cfg!(feature = "board-qemu_virt_aarch64_web_sddf")
            || cfg!(feature = "board-qemu_virt_riscv64_web_sddf")
            || cfg!(feature = "board-x86_64_generic_web_sddf")
            || cfg!(feature = "board-qemu_virt_aarch64_browser_sddf")
            || cfg!(feature = "board-qemu_virt_riscv64_browser_sddf"))
}

fn pci_lines() -> &'static [&'static [u8]] {
    if cfg!(feature = "board-swift_sfx14_41g") {
        SWIFT_PCI
    } else if cfg!(feature = "board-pc_z97_d3h_i217") {
        Z97_PCI
    } else if cfg!(feature = "board-x86_64_generic_fetch_native") {
        NATIVE_PCI
    } else if cfg!(feature = "board-x86_64_generic_fs_ahci") {
        AHCI_PCI
    } else {
        &[]
    }
}

fn mounted_fat() -> bool {
    #[cfg(any(
        feature = "board-pc_z97_d3h_i217",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))]
    {
        false
    }
    #[cfg(not(any(
        feature = "board-pc_z97_d3h_i217",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    {
        let Ok(root) = super::files().stat(b"/") else {
            return false;
        };
        super::is_dir(&root) && root.blksize == super::VOLUME_SECTOR
    }
}

#[cfg_attr(
    all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    ),
    expect(
        clippy::unnecessary_wraps,
        reason = "a board without a network client returns None"
    )
)]
fn mac_addr() -> Option<[u8; 6]> {
    #[cfg(all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    ))]
    {
        Some(super::net::hardware_address())
    }
    #[cfg(not(all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    )))]
    {
        None
    }
}

fn nvme_model() -> Option<&'static [u8]> {
    #[cfg(feature = "board-swift_sfx14_41g")]
    {
        disk_slot(512)
    }
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    {
        None
    }
}

fn smart_bytes() -> Option<&'static [u8]> {
    #[cfg(feature = "board-swift_sfx14_41g")]
    {
        disk_slot(768)
    }
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    {
        None
    }
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn disk_slot(at: usize) -> Option<&'static [u8]> {
    let page = disk_page();
    let len = usize::from(u16::from_le_bytes([*page.get(at)?, *page.get(at + 1)?]));
    let end = at.checked_add(2)?.checked_add(len)?;
    if len == 0 || len > 512 || end > page.len() {
        return None;
    }
    page.get(at + 2..end)
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn disk_page() -> &'static [u8] {
    // SAFETY: the Swift template maps `disk_result` read-only at this address
    // for the life of the shell. This worker only reads it.
    unsafe { core::slice::from_raw_parts(DISK_RESULT as *const u8, 0x1000) }
}

fn probe(addr: [u8; 4], port: u16) {
    #[cfg(all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    ))]
    {
        match super::net::probe_port(addr, port) {
            Ok(true) => super::say(b"nmap open\n"),
            Ok(false) => super::say(b"nmap closed\n"),
            Err(()) => miss(b"nmap"),
        }
    }
    #[cfg(not(all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    )))]
    {
        let _ = (addr, port);
        miss(b"nmap");
    }
}

fn show_link(mac: [u8; 6]) {
    #[cfg(all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    ))]
    {
        let up = super::net::local_address().is_ok();
        let mut line = [0u8; 40];
        let n = lerux_rescue::ethtool_line(up, mac, &mut line);
        if n == 0 {
            miss(b"ethtool");
        } else {
            super::say(&line[..n]);
        }
    }
    #[cfg(not(all(
        feature = "net-client",
        not(feature = "board-qemu_virt_aarch64_browser_sddf"),
        not(feature = "board-qemu_virt_riscv64_browser_sddf")
    )))]
    {
        let _ = mac;
        miss(b"ethtool");
    }
}

fn clone_files(src: &[u8], dst: &[u8]) {
    #[cfg(not(any(
        feature = "board-pc_z97_d3h_i217",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    {
        clone_volume(src, dst);
    }
    #[cfg(any(
        feature = "board-pc_z97_d3h_i217",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))]
    {
        let _ = (src, dst);
        miss(b"partclone");
    }
}

#[cfg(not(any(
    feature = "board-pc_z97_d3h_i217",
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn clone_volume(src: &[u8], dst: &[u8]) {
    let Ok(fd) = super::files().open(src, Open::Read) else {
        miss(b"partclone");
        return;
    };
    let mut buf = [0u8; 4096];
    let read = super::files().read(fd, &mut buf);
    let _ = super::files().close(fd);
    let Ok(n) = read else {
        miss(b"partclone");
        return;
    };
    if n == 0 || n == buf.len() {
        miss(b"partclone");
        return;
    }
    write_clone(dst, &buf[..n]);
}

#[cfg(not(any(
    feature = "board-pc_z97_d3h_i217",
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn write_clone(dst: &[u8], bytes: &[u8]) {
    let Ok(fd) = super::files().open(dst, Open::CreateWrite) else {
        miss(b"partclone");
        return;
    };
    let wrote = super::files().write(fd, bytes);
    let _ = super::files().close(fd);
    if matches!(wrote, Ok(n) if n == bytes.len()) {
        super::say(b"partclone ok\n");
    } else {
        miss(b"partclone");
    }
}

fn fuser_name(name: &[u8]) {
    #[cfg(not(any(
        feature = "board-pc_z97_d3h_i217",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    {
        if super::files().stat(name).is_ok() {
            super::say(b"fuser none\n");
            return;
        }
    }
    #[cfg(any(
        feature = "board-pc_z97_d3h_i217",
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))]
    {
        let _ = name;
    }
    miss(b"fuser");
}

fn journal_emit(filter: Option<&[u8]>) {
    #[cfg(not(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )))]
    {
        // SAFETY: the worker is the only user of this log. `emit` does not append.
        let log = unsafe { &*super::LOG.0.get() };
        let bytes = &log.bytes[..log.len];
        match filter {
            None => super::emit(bytes),
            Some(needle) => emit_hits(bytes, needle),
        }
    }
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    ))]
    {
        let _ = filter;
        miss(b"journalctl");
    }
}

#[cfg(not(any(
    feature = "board-qemu_virt_aarch64_browser_sddf",
    feature = "board-qemu_virt_riscv64_browser_sddf"
)))]
fn emit_hits(bytes: &[u8], needle: &[u8]) {
    if needle.is_empty() {
        return;
    }
    for line in bytes.split(|byte| *byte == b'\n') {
        if line.windows(needle.len()).any(|window| window == needle) {
            super::emit(line);
            super::emit(b"\n");
        }
    }
}
