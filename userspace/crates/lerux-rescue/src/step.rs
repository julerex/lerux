//! One command, one line. Side effects stay in the shell.

use crate::{
    parse::{ata_smart_line, endpoint, fat_boot_ok, identify, nvme_celsius, qemu_fat_boot, FsKind},
    text::{show, Buf},
};

/// One protection domain the supervisor already scheduled.
#[derive(Clone, Copy, Debug)]
pub struct Role<'a> {
    /// Protection-domain name.
    pub name: &'a [u8],
    /// Scheduling priority.
    pub priority: u8,
    /// Budget in microseconds, when the schedule page has one.
    pub budget_us: Option<u32>,
}

/// What the shell can see without opening a device itself.
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    /// `aarch64`, `x86_64`, or `riscv64`.
    pub arch: &'a [u8],
    /// Cores this image starts. These images start one.
    pub cores: u32,
    /// Block driver that booted, when this image has one.
    pub block: Option<&'a [u8]>,
    /// Roles in template order.
    pub roles: &'a [Role<'a>],
    /// Peripheral Component Interconnect configuration lines already recorded.
    pub pci: &'a [&'a [u8]],
    /// Stack the worker runs on, in bytes.
    pub stack: u32,
    /// Faults this domain has recorded.
    pub faults: u32,
    /// Boot timestamp in nanoseconds, when a timer was read.
    pub boot_ns: Option<u64>,
    /// Non-Volatile Memory Express identify model, when the controller stored one.
    pub nvme_model: Option<&'a [u8]>,
    /// Self-Monitoring page or Non-Volatile Memory Express health log.
    pub smart: Option<&'a [u8]>,
    /// Station or client hardware address.
    pub mac: Option<[u8; 6]>,
    /// Board facts that are either present or absent.
    pub flags: Flags,
}

/// Present-or-absent facts for [`View`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Flags(u8);

const MOUNTED: u8 = 1;
const NET: u8 = 2;
const LOG: u8 = 4;
const DAILY: u8 = 8;
const SECOND: u8 = 16;

impl Flags {
    /// No volume, no network, no log, no second disk.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// The shell's File Allocation Table volume answered `stat /`.
    #[must_use]
    pub const fn mounted(self, on: bool) -> Self {
        self.bit(MOUNTED, on)
    }

    /// This image has a network client.
    #[must_use]
    pub const fn net(self, on: bool) -> Self {
        self.bit(NET, on)
    }

    /// The shell keeps its own log.
    #[must_use]
    pub const fn log(self, on: bool) -> Self {
        self.bit(LOG, on)
    }

    /// `ip`, `ping`, `date`, and `dmesg` already have handlers.
    #[must_use]
    pub const fn daily(self, on: bool) -> Self {
        self.bit(DAILY, on)
    }

    /// A second disk is attached.
    #[must_use]
    pub const fn second(self, on: bool) -> Self {
        self.bit(SECOND, on)
    }

    #[must_use]
    pub const fn is_mounted(self) -> bool {
        self.0 & MOUNTED != 0
    }

    #[must_use]
    pub const fn has_net(self) -> bool {
        self.0 & NET != 0
    }

    #[must_use]
    pub const fn has_log(self) -> bool {
        self.0 & LOG != 0
    }

    #[must_use]
    pub const fn delegates_daily(self) -> bool {
        self.0 & DAILY != 0
    }

    #[must_use]
    pub const fn has_second(self) -> bool {
        self.0 & SECOND != 0
    }

    const fn bit(self, mask: u8, on: bool) -> Self {
        if on {
            Self(self.0 | mask)
        } else {
            Self(self.0 & !mask)
        }
    }
}

/// What the shell should do with a command it recognized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step<'a> {
    /// Not one of these names.
    Unknown,
    /// The device or the argument is missing. Print `name err`.
    Absent,
    /// `out[..n]` is the line, including the newline.
    Show(usize),
    /// Probe this one host and port.
    Probe([u8; 4], u16),
    /// Copy a file on the dedicated volume.
    Clone(&'a [u8], &'a [u8]),
    /// Say whether any domain holds this name open.
    Fuser(&'a [u8]),
    /// Print the shell log, or the lines that contain the filter.
    Journal(Option<&'a [u8]>),
    /// Print link state for this hardware address. Do not invent a speed.
    Link([u8; 6]),
}

/// `true` when `cmd` is a rescue name, including names an earlier table owns.
#[must_use]
pub fn owns(cmd: &[u8]) -> bool {
    everyday_name(cmd)
        || disk_name(cmd)
        || hardware_name(cmd)
        || copy_name(cmd)
        || volume_name(cmd)
        || trace_name(cmd)
}

/// Fill `out` or return the side effect the shell still has to perform.
#[must_use]
pub fn step<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Step<'a> {
    if !owns(cmd) {
        return Step::Unknown;
    }
    if view.flags.delegates_daily() && daily_name(cmd) {
        return Step::Unknown;
    }
    if let Some(found) = everyday(cmd, rest, view, out) {
        return found;
    }
    if let Some(found) = disk(cmd, rest, view, out) {
        return found;
    }
    if let Some(found) = hardware(cmd, rest, view) {
        return found;
    }
    if let Some(found) = copies(cmd, rest, view, out) {
        return found;
    }
    if let Some(found) = volumes(cmd, rest, view, out) {
        return found;
    }
    traces(cmd, rest, view, out).unwrap_or(Step::Absent)
}

/// `ethtool up 52:54:00:12:34:56`. Speed is omitted when this image has none.
#[must_use]
pub fn ethtool_line(up: bool, mac: [u8; 6], out: &mut [u8]) -> usize {
    let mut buf = Buf::new(out);
    let state: &[u8] = if up { b"up " } else { b"down " };
    if buf.push(b"ethtool ") && buf.push(state) && push_mac(&mut buf, mac) && buf.push(b"\n") {
        buf.len()
    } else {
        0
    }
}

fn everyday<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Option<Step<'a>> {
    Some(match cmd {
        b"ps" => roles(b"ps ok\n", rest, view, out, false),
        b"top" => roles(b"top ok\n", rest, view, out, false),
        b"pstree" => roles(b"pstree ok\n", rest, view, out, true),
        b"free" => bare(rest, literal(b"free stack ", view.stack, out)),
        b"vmstat" => bare(rest, literal(b"vmstat ", view.faults, out)),
        b"lscpu" => bare(rest, lscpu(view, out)),
        b"lsblk" => bare(rest, lsblk(view, out)),
        b"findmnt" => bare(rest, findmnt(view, out)),
        b"lspci" => bare(rest, pci(view, out)),
        b"ss" => bare(rest, ss(view, out)),
        b"systemd-analyze" => bare(rest, analyze(view, out)),
        b"systemctl" => systemctl(rest, view, out),
        b"fuser" => fuser(rest, view),
        b"journalctl" => journal(rest, view),
        _ => return None,
    })
}

fn disk<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Option<Step<'a>> {
    Some(match cmd {
        b"smartctl" => bare(rest, smart(view, out)),
        b"nvme" => nvme(rest, view, out),
        b"hdparm" | b"sdparm" | b"lsscsi" => Step::Absent,
        _ => return None,
    })
}

fn hardware<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>) -> Option<Step<'a>> {
    Some(match cmd {
        b"ethtool" => ethtool(rest, view),
        b"dmidecode" | b"lsusb" => Step::Absent,
        _ => return None,
    })
}

fn copies<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Option<Step<'a>> {
    Some(match cmd {
        b"fdisk" | b"parted" | b"gdisk" | b"gpart" | b"testdisk" => {
            bare(rest, table(cmd, view, out))
        }
        b"partclone" => partclone(rest, view),
        b"ddrescue" => ddrescue(rest, view),
        _ => return None,
    })
}

fn volumes<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Option<Step<'a>> {
    Some(match cmd {
        b"fsck" => bare(rest, fsck(view, out)),
        b"cryptsetup" => crypt(rest, view, out),
        b"e2fsprogs" | b"btrfs-progs" | b"xfsprogs" | b"ntfsprogs" | b"lvm2" | b"mdadm"
        | b"memtest" => Step::Absent,
        _ => return None,
    })
}

fn traces<'a>(cmd: &[u8], rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Option<Step<'a>> {
    Some(match cmd {
        b"dig" | b"ip" | b"ping" | b"dmesg" | b"date" => Step::Absent,
        b"tcpdump" => bare(rest, tcpdump(view, out)),
        b"nmap" => nmap(rest, view),
        _ => return None,
    })
}

fn everyday_name(cmd: &[u8]) -> bool {
    matches!(
        cmd,
        b"ps"
            | b"top"
            | b"pstree"
            | b"free"
            | b"vmstat"
            | b"lscpu"
            | b"lsblk"
            | b"findmnt"
            | b"lspci"
            | b"ss"
            | b"systemd-analyze"
            | b"systemctl"
            | b"fuser"
            | b"journalctl"
    )
}

fn disk_name(cmd: &[u8]) -> bool {
    matches!(
        cmd,
        b"smartctl" | b"nvme" | b"hdparm" | b"sdparm" | b"lsscsi"
    )
}

fn hardware_name(cmd: &[u8]) -> bool {
    matches!(cmd, b"ethtool" | b"dmidecode" | b"lsusb")
}

fn copy_name(cmd: &[u8]) -> bool {
    matches!(
        cmd,
        b"fdisk" | b"parted" | b"gdisk" | b"gpart" | b"testdisk" | b"partclone" | b"ddrescue"
    )
}

fn volume_name(cmd: &[u8]) -> bool {
    matches!(
        cmd,
        b"fsck"
            | b"e2fsprogs"
            | b"btrfs-progs"
            | b"xfsprogs"
            | b"ntfsprogs"
            | b"cryptsetup"
            | b"lvm2"
            | b"mdadm"
            | b"memtest"
    )
}

fn trace_name(cmd: &[u8]) -> bool {
    matches!(
        cmd,
        b"dig" | b"ip" | b"ping" | b"dmesg" | b"date" | b"tcpdump" | b"nmap"
    )
}

const fn daily_name(cmd: &[u8]) -> bool {
    matches!(cmd, b"ip" | b"ping" | b"dmesg" | b"date")
}

fn bare<'a>(rest: &[u8], step: Step<'a>) -> Step<'a> {
    if rest.is_empty() {
        step
    } else {
        Step::Absent
    }
}

fn roles<'a>(
    trailer: &[u8],
    rest: &'a [u8],
    view: &View<'_>,
    out: &mut [u8],
    names_only: bool,
) -> Step<'a> {
    if !rest.is_empty() || view.roles.is_empty() {
        return Step::Absent;
    }
    let mut buf = Buf::new(out);
    for role in view.roles {
        if !push_role(&mut buf, role, names_only) {
            return Step::Absent;
        }
    }
    if buf.push(trailer) {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn push_role(buf: &mut Buf<'_>, role: &Role<'_>, names_only: bool) -> bool {
    if names_only {
        return buf.push(role.name) && buf.push(b"\n");
    }
    if !(buf.push(role.name) && buf.push(b" ") && buf.u64(u64::from(role.priority))) {
        return false;
    }
    if let Some(budget) = role.budget_us
        && !(buf.push(b" ") && buf.u64(u64::from(budget)))
    {
        return false;
    }
    buf.push(b"\n")
}

fn literal(prefix: &[u8], value: u32, out: &mut [u8]) -> Step<'static> {
    let mut buf = Buf::new(out);
    if buf.push(prefix) && buf.u64(u64::from(value)) && buf.push(b"\n") {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn lscpu(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    let mut buf = Buf::new(out);
    let cores = u64::from(view.cores);
    if buf.push(b"lscpu ")
        && buf.push(view.arch)
        && buf.push(b" ")
        && buf.u64(cores)
        && buf.push(b"\n")
    {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn lsblk(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    let Some(name) = view.block else {
        return Step::Absent;
    };
    let mut buf = Buf::new(out);
    if buf.push(b"lsblk ") && buf.push(name) && buf.push(b"\n") {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn findmnt(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    if view.flags.is_mounted() {
        put(b"findmnt fat\n", out)
    } else {
        Step::Absent
    }
}

fn pci(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    if view.pci.is_empty() {
        return Step::Absent;
    }
    let mut buf = Buf::new(out);
    for line in view.pci {
        if !(buf.push(line) && buf.push(b"\n")) {
            return Step::Absent;
        }
    }
    Step::Show(buf.len())
}

fn ss(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    if view.flags.has_net() {
        put(b"ss none\n", out)
    } else {
        Step::Absent
    }
}

fn analyze(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    let Some(ns) = view.boot_ns else {
        return Step::Absent;
    };
    let mut buf = Buf::new(out);
    if buf.push(b"systemd-analyze ") && buf.u64(ns) && buf.push(b"\n") {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn systemctl<'a>(rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Step<'a> {
    let Some(name) = one(rest) else {
        return Step::Absent;
    };
    if !view.roles.iter().any(|role| role.name == name) {
        return Step::Absent;
    }
    let mut buf = Buf::new(out);
    if buf.push(name) && buf.push(b" ready\n") {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn fuser<'a>(rest: &'a [u8], view: &View<'_>) -> Step<'a> {
    if !view.flags.is_mounted() {
        return Step::Absent;
    }
    match one(rest) {
        Some(name) => Step::Fuser(name),
        None => Step::Absent,
    }
}

fn journal<'a>(rest: &'a [u8], view: &View<'_>) -> Step<'a> {
    if !view.flags.has_log() {
        return Step::Absent;
    }
    if rest.is_empty() || rest == b"-l" {
        return Step::Journal(None);
    }
    if let Some(name) = tail(rest, b"--pd") {
        return Step::Journal(Some(name));
    }
    if let Some(word) = tail(rest, b"-l") {
        return Step::Journal(Some(word));
    }
    Step::Absent
}

fn smart(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    let Some(page) = view.smart else {
        return Step::Absent;
    };
    if let Some(temp) = nvme_celsius(page) {
        return literal(b"smartctl ", u32::from(temp), out);
    }
    match ata_smart_line(page, out) {
        Some(n) => newline_after(out, n),
        None => Step::Absent,
    }
}

fn nvme<'a>(rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Step<'a> {
    if !rest.is_empty() {
        return Step::Absent;
    }
    let Some(model) = view.nvme_model else {
        return Step::Absent;
    };
    let mut buf = Buf::new(out);
    if buf.push(b"nvme ") && buf.push(model) && buf.push(b"\n") {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn ethtool<'a>(rest: &'a [u8], view: &View<'_>) -> Step<'a> {
    if !rest.is_empty() || !view.flags.has_net() {
        return Step::Absent;
    }
    match view.mac {
        Some(mac) => Step::Link(mac),
        None => Step::Absent,
    }
}

fn table(cmd: &[u8], view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    if !view.flags.is_mounted() {
        return Step::Absent;
    }
    let mut buf = Buf::new(out);
    if buf.push(cmd) && buf.push(b" none\n") {
        Step::Show(buf.len())
    } else {
        Step::Absent
    }
}

fn partclone<'a>(rest: &'a [u8], view: &View<'_>) -> Step<'a> {
    if !view.flags.is_mounted() {
        return Step::Absent;
    }
    let Some((token, tail)) = word(rest) else {
        return Step::Absent;
    };
    if token != b"lerux" {
        return Step::Absent;
    }
    match two(tail) {
        Some((src, dst)) => Step::Clone(src, dst),
        None => Step::Absent,
    }
}

fn ddrescue<'a>(rest: &'a [u8], view: &View<'_>) -> Step<'a> {
    // The source would stay read-only, and the destination would have to be the
    // dedicated volume on a second disk. These images have no second disk, so
    // the copy does not start.
    let Some((_, dst)) = two(rest) else {
        return Step::Absent;
    };
    let allowed = crate::allow_copy(crate::CopyPolicy {
        second_disk: view.flags.has_second(),
        dest_dedicated: dst == b"lerux",
        source_read_only: true,
    });
    if allowed {
        return Step::Absent;
    }
    Step::Absent
}

fn fsck(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    if view.flags.is_mounted() && fat_boot_ok(&qemu_fat_boot()) {
        put(b"fsck ok\n", out)
    } else {
        Step::Absent
    }
}

fn crypt<'a>(rest: &'a [u8], view: &View<'_>, out: &mut [u8]) -> Step<'a> {
    if !rest.is_empty() || !view.flags.is_mounted() {
        return Step::Absent;
    }
    if identify(&qemu_fat_boot()) == FsKind::Luks {
        return put(b"cryptsetup locked\n", out);
    }
    Step::Absent
}

fn tcpdump(view: &View<'_>, out: &mut [u8]) -> Step<'static> {
    if view.flags.has_net() {
        put(b"tcpdump none\n", out)
    } else {
        Step::Absent
    }
}

fn nmap<'a>(rest: &'a [u8], view: &View<'_>) -> Step<'a> {
    if !view.flags.has_net() {
        return Step::Absent;
    }
    match endpoint(rest) {
        Some(end) => Step::Probe(end.addr, end.port),
        None => Step::Absent,
    }
}

fn put(bytes: &'static [u8], out: &mut [u8]) -> Step<'static> {
    match show(bytes, out) {
        Some(n) => Step::Show(n),
        None => Step::Absent,
    }
}

fn newline_after(out: &mut [u8], n: usize) -> Step<'static> {
    if n >= out.len() {
        return Step::Absent;
    }
    out[n] = b'\n';
    Step::Show(n + 1)
}

fn one(rest: &[u8]) -> Option<&[u8]> {
    let (word, extra) = word(rest)?;
    if extra.is_empty() {
        Some(word)
    } else {
        None
    }
}

fn two(rest: &[u8]) -> Option<(&[u8], &[u8])> {
    let (src, tail) = word(rest)?;
    let (dst, extra) = word(tail)?;
    if extra.is_empty() {
        Some((src, dst))
    } else {
        None
    }
}

fn tail<'a>(rest: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let (head, more) = word(rest)?;
    if head != key {
        return None;
    }
    one(more)
}

fn word(text: &[u8]) -> Option<(&[u8], &[u8])> {
    let text = trim(text);
    if text.is_empty() {
        return None;
    }
    let end = text
        .iter()
        .position(|byte| *byte == b' ' || *byte == b'\t')
        .unwrap_or(text.len());
    Some((&text[..end], trim(&text[end..])))
}

fn trim(text: &[u8]) -> &[u8] {
    let start = text
        .iter()
        .position(|byte| *byte != b' ' && *byte != b'\t')
        .unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|byte| *byte != b' ' && *byte != b'\t')
        .map_or(start, |index| index + 1);
    &text[start..end]
}

fn push_mac(buf: &mut Buf<'_>, mac: [u8; 6]) -> bool {
    for (index, byte) in mac.iter().enumerate() {
        if index > 0 && !buf.push(b":") {
            return false;
        }
        if !push_hex(buf, *byte) {
            return false;
        }
    }
    true
}

fn push_hex(buf: &mut Buf<'_>, byte: u8) -> bool {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let hi = usize::from(byte >> 4);
    let lo = usize::from(byte & 0x0f);
    buf.push(&[HEX[hi], HEX[lo]])
}
