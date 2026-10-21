//! Device-framework images.
//!
//! Web is the fetch image with a different mode. It has no second description.

#[cfg(test)]
use crate::model::render_forcing_end;
use lerux_machine::{
    description::{
        IoApicPin, IoApicVector, Irq, MmioDevice, MmioWindow, PciBar, Polarity, SerialNode,
        Trigger, VirtioNode, X86Irq,
    },
    virtio_dma_guest, virtio_dma_phys_addr, virtio_node, Arch, VirtioRole,
};

use crate::model::{
    line_irq, render_image, Domain, Image, ImageError, ImageKind, Link, Map, Mode, Qos, Region,
    Slot, REGION_SIZE, STACK_SIZE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardId {
    pub kind: ImageKind,
    pub mode: Option<Mode>,
    pub arch: Arch,
}

pub(crate) fn render_serial() -> Result<String, ImageError> {
    render_kind(ImageKind::Serial, Arch::Aarch64)
}

pub fn render_kind(kind: ImageKind, arch: Arch) -> Result<String, ImageError> {
    render_image(&image(kind, arch)?)
}

const SDDF_SUFFIX: &str = "_sddf";

pub fn board_registration(board: &str) -> Result<Option<BoardId>, ImageError> {
    if !board.ends_with(SDDF_SUFFIX) {
        return Ok(None);
    }
    let stem = &board[..board.len() - SDDF_SUFFIX.len()];
    let (arch, token) = Arch::split_prefix(stem).ok_or_else(|| ImageError::UnknownBoard {
        name: board.to_string(),
    })?;
    let (kind, mode) = kind_from_token(token).ok_or_else(|| ImageError::UnknownBoard {
        name: board.to_string(),
    })?;
    Ok(Some(BoardId { kind, mode, arch }))
}

fn kind_from_token(token: &str) -> Option<(ImageKind, Option<Mode>)> {
    match token {
        "serial" => Some((ImageKind::Serial, None)),
        "fs" => Some((ImageKind::Fs, None)),
        "net" => Some((ImageKind::Net, None)),
        "fetch" => Some((ImageKind::Fetch, Some(Mode::Fetch))),
        "web" => Some((ImageKind::Fetch, Some(Mode::Web))),
        "browser" => Some((ImageKind::Browser, None)),
        "agent" => Some((ImageKind::Agent, None)),
        "program" => Some((ImageKind::Program, None)),
        _ => None,
    }
}

pub fn render_board(board: &str) -> Result<Option<String>, ImageError> {
    let Some(id) = board_registration(board)? else {
        return Ok(None);
    };
    render_kind(id.kind, id.arch).map(Some)
}

#[cfg(test)]
pub(crate) fn render_with_region_vaddr(name: &str, vaddr: u64) -> Result<String, ImageError> {
    let mut image = image(ImageKind::Serial, Arch::Aarch64)?;
    {
        let region = image
            .groups
            .iter_mut()
            .flatten()
            .find(|region| region.name == name)
            .ok_or_else(|| ImageError::UnknownRegion {
                name: name.to_string(),
            })?;
        region.vaddr = vaddr;
    }
    render_image(&image)
}

#[cfg(test)]
pub(crate) fn render_with_extra_end(pd: &'static str, id: u8) -> Result<String, ImageError> {
    render_forcing_end(&image(ImageKind::Serial, Arch::Aarch64)?, pd, id)
}

fn image(kind: ImageKind, arch: Arch) -> Result<Image, ImageError> {
    let supported = kind == ImageKind::Serial
        || arch == Arch::Aarch64
        || (kind == ImageKind::Fs && matches!(arch, Arch::Riscv64 | Arch::X86_64))
        || (kind == ImageKind::Net && matches!(arch, Arch::Riscv64 | Arch::X86_64))
        || (kind == ImageKind::Fetch && matches!(arch, Arch::Riscv64 | Arch::X86_64))
        || (kind == ImageKind::Browser && matches!(arch, Arch::Riscv64 | Arch::Aarch64))
        || (kind == ImageKind::Agent
            && matches!(arch, Arch::Riscv64 | Arch::Aarch64 | Arch::X86_64))
        || (kind == ImageKind::Program && matches!(arch, Arch::Riscv64 | Arch::X86_64));
    if !supported {
        return Err(ImageError::Unsupported { kind, arch });
    }
    Ok(match kind {
        ImageKind::Serial => serial_image(arch),
        ImageKind::Fs => fs_image(arch),
        ImageKind::Net => net_image(arch),
        ImageKind::Fetch => fetch_image(arch, "shell", "sddf-shell.elf", fetch_client_slots()),
        ImageKind::Browser => browser_image(arch),
        ImageKind::Agent => agent_image(arch),
        ImageKind::Program => {
            fetch_image(arch, "program", "sddf-program.elf", fetch_client_slots())
        }
    })
}

struct Attachment {
    regions: Vec<Region>,
    slots: Vec<Slot>,
    irq: Irq,
}

fn serial_image(arch: Arch) -> Image {
    let attachment = serial_attachment(arch);
    Image {
        groups: vec![attachment.regions],
        domains: vec![
            serial_driver(attachment.slots, attachment.irq, None),
            serial_virt_tx(None),
            serial_virt_rx(None),
            domain(
                "serial_client",
                1,
                None,
                "sddf-serial-client.elf",
                serial_client_slots(),
                None,
            ),
        ],
        links: link_rows(&[
            ("serial_driver", "serial_virt_tx", 0),
            ("serial_driver", "serial_virt_rx", 1),
            ("serial_virt_tx", "serial_client", 2),
            ("serial_virt_rx", "serial_client", 3),
        ]),
    }
}

fn serial_attachment(arch: Arch) -> Attachment {
    let mut regions = Vec::new();
    let mut slots = Vec::new();
    let irq = match lerux_machine::serial_node(arch) {
        SerialNode::Mmio(device) => {
            regions.push(window_region(&device.window));
            slots.push(window_map(&device));
            Irq::Line(device.irq)
        }
        SerialNode::Port { port, irq: x86_irq } => {
            slots.push(Slot::Port(port));
            Irq::X86(x86_irq)
        }
    };
    regions.extend(serial_queue_regions());
    slots.extend(serial_driver_queue_maps());
    Attachment {
        regions,
        slots,
        irq,
    }
}

fn fs_image(arch: Arch) -> Image {
    let serial = serial_attachment(arch);
    let BlkVirtioAttachment {
        regions: blk_virtio_regions,
        driver_slots: blk_driver_slots,
        irq: blk_irq,
    } = blk_virtio_attachment(arch, false);
    let mut shell = serial_client_slots();
    shell.extend(fs_client_slots());
    let mut links = link_rows(&[
        ("blk_driver", "blk_virt", 0),
        ("blk_virt", "fatfs", 1),
        ("fatfs", "shell", 6),
        ("serial_driver", "serial_virt_tx", 2),
        ("serial_driver", "serial_virt_rx", 3),
        ("serial_virt_tx", "shell", 4),
        ("serial_virt_rx", "shell", 5),
    ]);
    links.push(Link {
        a: "timer_driver",
        b: "shell",
        pp_b: true,
        order: 7,
    });
    Image {
        groups: vec![
            serial.regions,
            blk_virtio_regions,
            blk_regions(),
            fs_regions(),
        ],
        domains: vec![
            blk_driver_domain(blk_driver_slots, blk_irq, budget(4000), 4),
            serial_driver(serial.slots, serial.irq, budget(4000)),
            blk_virt(budget(3000)),
            serial_virt_tx(budget(3000)),
            serial_virt_rx(budget(3000)),
            fatfs(budget(2000)),
            domain(
                "timer_driver",
                5,
                Some(Qos {
                    budget: 5000,
                    passive: true,
                }),
                "sddf-timer-driver.elf",
                Vec::new(),
                None,
            ),
            domain("shell", 1, budget(1000), "sddf-shell.elf", shell, None),
        ],
        links,
    }
}

fn net_image(arch: Arch) -> Image {
    let serial = serial_attachment(arch);
    let mut client = serial_client_slots();
    client.extend(net_client_slots());
    Image {
        groups: vec![
            serial.regions,
            net_virtio_regions(arch, false),
            net_regions(false),
        ],
        domains: vec![
            net_driver(arch, false, None, 4),
            serial_driver(serial.slots, serial.irq, None),
            net_virt_tx(None),
            net_virt_rx(None),
            serial_virt_tx(None),
            serial_virt_rx(None),
            net_copy(None),
            domain("net_client", 1, None, "sddf-net-client.elf", client, None),
        ],
        links: link_rows(&[
            ("serial_driver", "serial_virt_tx", 0),
            ("serial_driver", "serial_virt_rx", 1),
            ("serial_virt_tx", "net_client", 2),
            ("serial_virt_rx", "net_client", 3),
            ("net_driver", "net_virt_rx", 4),
            ("net_driver", "net_virt_tx", 5),
            ("net_virt_rx", "net_copy", 6),
            ("net_virt_tx", "net_client", 8),
            ("net_copy", "net_client", 7),
        ]),
    }
}

fn fetch_virtio_regions(arch: Arch) -> Vec<Region> {
    match arch {
        Arch::Aarch64 => virtio_regions(true, true, false),
        Arch::Riscv64 => {
            let mut regions = blk_virtio_attachment(arch, false).regions;
            regions.extend(net_virtio_regions(arch, false));
            regions
        }
        Arch::X86_64 => {
            let mut regions = blk_virtio_attachment(arch, true).regions;
            regions.extend(net_virtio_regions(arch, false));
            regions
        }
    }
}

/// Round-robin budgets on x86 match the filesystem image for every role so
/// network domains cannot starve `fatfs`.
fn fetch_qos(arch: Arch, value: u32) -> Option<Qos> {
    if arch == Arch::X86_64 {
        budget(value)
    } else {
        None
    }
}

/// QEMU q35 with both virtio PCI functions routes block at pin 11 and net at pin 10.
fn fetch_x86_pci_irq(role: VirtioRole) -> Irq {
    let (pin, vector) = match role {
        VirtioRole::Blk => (11, 49),
        VirtioRole::Net => (10, 50),
        VirtioRole::Gpu => panic!("fetch image has no virtio-gpu"),
    };
    Irq::X86(X86Irq {
        pin: IoApicPin(pin),
        vector: IoApicVector(vector),
        trigger: Trigger::Level,
        polarity: Polarity::High,
    })
}

fn fetch_image(arch: Arch, client: &'static str, program: &'static str, slots: Vec<Slot>) -> Image {
    let serial = serial_attachment(arch);
    let blk = blk_virtio_attachment(arch, arch == Arch::X86_64);
    let blk_irq = if arch == Arch::X86_64 {
        fetch_x86_pci_irq(VirtioRole::Blk)
    } else {
        blk.irq
    };
    let net_irq = if arch == Arch::X86_64 {
        fetch_x86_pci_irq(VirtioRole::Net)
    } else {
        net_virtio_attachment(arch, false).irq
    };
    Image {
        groups: vec![
            serial.regions,
            fetch_virtio_regions(arch),
            blk_regions(),
            fs_regions(),
            net_regions(false),
        ],
        domains: vec![
            blk_driver_domain(blk.driver_slots, blk_irq, fetch_qos(arch, 4000), 4),
            serial_driver(serial.slots, serial.irq, fetch_qos(arch, 4000)),
            blk_virt(fetch_qos(arch, 3000)),
            serial_virt_tx(fetch_qos(arch, 3000)),
            serial_virt_rx(fetch_qos(arch, 3000)),
            fatfs(fetch_qos(arch, 2000)),
            net_driver_with_irq(
                arch,
                false,
                net_irq,
                fetch_qos(arch, if arch == Arch::X86_64 { 2000 } else { 4000 }),
                if arch == Arch::X86_64 { 2 } else { 4 },
            ),
            net_virt_tx(fetch_qos(arch, 3000)),
            net_virt_rx(fetch_qos(arch, 3000)),
            net_copy(fetch_qos(arch, 2000)),
            domain(client, 1, fetch_qos(arch, 1000), program, slots, None),
        ],
        links: fabric_links(client),
    }
}

fn browser_virtio_regions(arch: Arch) -> Vec<Region> {
    match arch {
        Arch::Aarch64 => virtio_regions(true, true, true),
        Arch::Riscv64 => {
            let mut regions = blk_virtio_attachment(arch, false).regions;
            regions.extend(net_virtio_regions(arch, false));
            regions.extend(gpu_virtio_regions(arch));
            regions
        }
        Arch::X86_64 => panic!("browser image is not built for x86"),
    }
}

fn gpu_virtio_regions(arch: Arch) -> Vec<Region> {
    let VirtioNode::Mmio(device) =
        virtio_node(arch, VirtioRole::Gpu).expect("browser image includes virtio-gpu")
    else {
        panic!("virtio-gpu is memory-mapped on this board");
    };
    vec![window_region(&device.window), gpu_driver_dma_region(arch)]
}

fn gpu_driver_dma_region(arch: Arch) -> Region {
    let size = 0x200_000;
    let vaddr = virtio_dma_guest(arch, VirtioRole::Gpu);
    if let Some(phys) = virtio_dma_phys_addr(arch, VirtioRole::Gpu) {
        Region {
            name: "virtio_gpu_driver_dma",
            size,
            vaddr,
            phys_addr: Some(phys),
            page_size: Some(size),
        }
    } else {
        paged("virtio_gpu_driver_dma", size, vaddr)
    }
}

fn browser_image(arch: Arch) -> Image {
    let serial = serial_attachment(arch);
    let blk = blk_virtio_attachment(arch, false);
    let net_irq = net_virtio_attachment(arch, false).irq;
    let mut slots = fetch_client_slots();
    slots.extend(cached(&[
        "gpu_client_events",
        "gpu_client_req",
        "gpu_client_resp",
        "gpu_client_data",
    ]));
    let mut links = fabric_links("shell");
    links.extend(link_rows(&[
        ("gpu_driver", "gpu_virt", 12),
        ("gpu_virt", "shell", 13),
    ]));
    Image {
        groups: vec![
            serial.regions,
            browser_virtio_regions(arch),
            blk_regions(),
            fs_regions(),
            net_regions(false),
            gpu_regions(),
        ],
        domains: vec![
            blk_driver_domain(blk.driver_slots, blk.irq, None, 4),
            net_driver_with_irq(arch, false, net_irq, None, 4),
            gpu_driver(arch),
            serial_driver(serial.slots, serial.irq, None),
            blk_virt(None),
            net_virt_tx(None),
            net_virt_rx(None),
            serial_virt_tx(None),
            serial_virt_rx(None),
            gpu_virt(),
            fatfs(None),
            net_copy(None),
            domain("shell", 1, None, "sddf-shell.elf", slots, None),
        ],
        links,
    }
}

fn agent_image(arch: Arch) -> Image {
    let serial = serial_attachment(arch);
    let blk = blk_virtio_attachment(arch, arch == Arch::X86_64);
    let blk_irq = if arch == Arch::X86_64 {
        fetch_x86_pci_irq(VirtioRole::Blk)
    } else {
        blk.irq
    };
    let net_irq = if arch == Arch::X86_64 {
        fetch_x86_pci_irq(VirtioRole::Net)
    } else {
        net_virtio_attachment(arch, false).irq
    };
    let mut grok = serial_client_slots();
    grok.extend(cached(&["agent_tool"]));
    let mut agent = fs_client_slots();
    agent.extend(net_client_slots());
    agent.extend(cached(&["agent_tool"]));
    Image {
        groups: vec![
            serial.regions,
            fetch_virtio_regions(arch),
            blk_regions(),
            fs_regions(),
            net_regions(true),
        ],
        domains: vec![
            blk_driver_domain(blk.driver_slots, blk_irq, fetch_qos(arch, 4000), 4),
            net_driver_with_irq(
                arch,
                false,
                net_irq,
                fetch_qos(arch, if arch == Arch::X86_64 { 2000 } else { 4000 }),
                if arch == Arch::X86_64 { 2 } else { 4 },
            ),
            serial_driver(serial.slots, serial.irq, fetch_qos(arch, 4000)),
            blk_virt(fetch_qos(arch, 3000)),
            net_virt_tx(fetch_qos(arch, 3000)),
            net_virt_rx(fetch_qos(arch, 3000)),
            serial_virt_tx(fetch_qos(arch, 3000)),
            serial_virt_rx(fetch_qos(arch, 3000)),
            fatfs(fetch_qos(arch, 2000)),
            net_copy(fetch_qos(arch, 2000)),
            domain(
                "shell",
                1,
                fetch_qos(arch, 1000),
                "sddf-grok.elf",
                grok,
                None,
            ),
            domain(
                "agent",
                2,
                fetch_qos(arch, 2000),
                "sddf-agent.elf",
                agent,
                None,
            ),
        ],
        links: link_rows(&[
            ("blk_driver", "blk_virt", 0),
            ("blk_virt", "fatfs", 1),
            ("fatfs", "agent", 8),
            ("serial_driver", "serial_virt_tx", 2),
            ("serial_driver", "serial_virt_rx", 3),
            ("serial_virt_tx", "shell", 4),
            ("serial_virt_rx", "shell", 5),
            ("net_driver", "net_virt_rx", 9),
            ("net_driver", "net_virt_tx", 10),
            ("net_virt_rx", "net_copy", 11),
            ("net_copy", "agent", 12),
            ("net_virt_tx", "agent", 13),
            ("shell", "agent", 6),
            ("shell", "agent", 7),
        ]),
    }
}

fn fabric_links(client: &'static str) -> Vec<Link> {
    link_rows(&[
        ("blk_driver", "blk_virt", 0),
        ("blk_virt", "fatfs", 1),
        ("fatfs", client, 6),
        ("serial_driver", "serial_virt_tx", 2),
        ("serial_driver", "serial_virt_rx", 3),
        ("serial_virt_tx", client, 4),
        ("serial_virt_rx", client, 5),
        ("net_driver", "net_virt_rx", 7),
        ("net_driver", "net_virt_tx", 8),
        ("net_virt_rx", "net_copy", 9),
        ("net_copy", client, 10),
        ("net_virt_tx", client, 11),
    ])
}

fn serial_driver(slots: Vec<Slot>, irq: Irq, qos: Option<Qos>) -> Domain {
    domain(
        "serial_driver",
        4,
        qos,
        "sddf-serial-driver.elf",
        slots,
        Some(irq),
    )
}

fn serial_virt_tx(qos: Option<Qos>) -> Domain {
    domain(
        "serial_virt_tx",
        3,
        qos,
        "sddf-serial-virt-tx.elf",
        cached(&[
            "serial_tx_driver_queue",
            "serial_tx_driver_data",
            "serial_tx_client_queue",
            "serial_tx_client_data",
        ]),
        None,
    )
}

fn serial_virt_rx(qos: Option<Qos>) -> Domain {
    domain(
        "serial_virt_rx",
        3,
        qos,
        "sddf-serial-virt-rx.elf",
        cached(&[
            "serial_rx_driver_queue",
            "serial_rx_driver_data",
            "serial_rx_client_queue",
            "serial_rx_client_data",
        ]),
        None,
    )
}

struct BlkVirtioAttachment {
    regions: Vec<Region>,
    driver_slots: Vec<Slot>,
    irq: Irq,
}

fn blk_virtio_attachment(arch: Arch, blk_pci_ecam: bool) -> BlkVirtioAttachment {
    match arch {
        Arch::Aarch64 => BlkVirtioAttachment {
            regions: virtio_regions(true, false, false),
            driver_slots: vec![
                mmio("virtio_mmio", "virtio_blk_mmio_vaddr"),
                dma("virtio_blk_driver_dma", "virtio_blk_driver_dma_vaddr"),
                setvar("virtio_blk_driver_dma_paddr", "virtio_blk_driver_dma"),
            ],
            irq: line_irq(78),
        },
        Arch::Riscv64 => {
            let VirtioNode::Mmio(device) =
                virtio_node(arch, VirtioRole::Blk).expect("filesystem image includes virtio-blk")
            else {
                panic!("riscv virtio-blk is memory-mapped input/output");
            };
            BlkVirtioAttachment {
                regions: vec![window_region(&device.window), blk_driver_dma_region(arch)],
                driver_slots: vec![
                    mmio(device.window.name, device.setvar),
                    dma("virtio_blk_driver_dma", "virtio_blk_driver_dma_vaddr"),
                    setvar("virtio_blk_driver_dma_paddr", "virtio_blk_driver_dma"),
                ],
                irq: Irq::Line(device.irq),
            }
        }
        Arch::X86_64 => {
            let VirtioNode::Pci {
                config, irq, bars, ..
            } = virtio_node(arch, VirtioRole::Blk).expect("filesystem image includes virtio-blk")
            else {
                panic!("x86 virtio-blk is PCI");
            };
            let mut regions = pci_bar_regions(bars);
            regions.push(blk_driver_dma_region(arch));
            let mut driver_slots = Vec::new();
            if blk_pci_ecam {
                use lerux_machine::{
                    X86_PCI_ECAM_GUEST, X86_PCI_ECAM_PHYS, X86_PCI_ECAM_SETVAR, X86_PCI_ECAM_SIZE,
                    X86_PCI_ECAM_WINDOW,
                };
                regions.push(Region {
                    name: X86_PCI_ECAM_WINDOW,
                    size: X86_PCI_ECAM_SIZE,
                    vaddr: X86_PCI_ECAM_GUEST,
                    phys_addr: Some(X86_PCI_ECAM_PHYS.text()),
                    page_size: None,
                });
                driver_slots.push(mmio(X86_PCI_ECAM_WINDOW, X86_PCI_ECAM_SETVAR));
            } else {
                driver_slots.push(Slot::Port(config));
            }
            for bar in bars {
                driver_slots.push(Slot::Map(Map {
                    region: bar.setvar,
                    cached: Some(false),
                    setvar: None,
                }));
            }
            driver_slots.extend([
                dma("virtio_blk_driver_dma", "virtio_blk_driver_dma_vaddr"),
                setvar("virtio_blk_driver_dma_paddr", "virtio_blk_driver_dma"),
            ]);
            BlkVirtioAttachment {
                regions,
                driver_slots,
                irq: Irq::X86(irq),
            }
        }
    }
}

fn blk_driver_dma_region(arch: Arch) -> Region {
    let size = 0x200_000;
    let vaddr = virtio_dma_guest(arch, VirtioRole::Blk);
    if let Some(phys) = virtio_dma_phys_addr(arch, VirtioRole::Blk) {
        Region {
            name: "virtio_blk_driver_dma",
            size,
            vaddr,
            phys_addr: Some(phys),
            page_size: Some(size),
        }
    } else {
        paged("virtio_blk_driver_dma", size, vaddr)
    }
}

fn blk_driver_domain(slots: Vec<Slot>, irq: Irq, qos: Option<Qos>, priority: u8) -> Domain {
    let mut slots = slots;
    slots.extend(cached(&[
        "blk_storage_info",
        "blk_driver_req_queue",
        "blk_driver_resp_queue",
        "blk_data",
    ]));
    domain(
        "blk_driver",
        priority,
        qos,
        "sddf-blk-driver.elf",
        slots,
        Some(irq),
    )
}

fn blk_virt(qos: Option<Qos>) -> Domain {
    domain(
        "blk_virt",
        3,
        qos,
        "sddf-blk-virt.elf",
        cached(&[
            "blk_storage_info",
            "blk_driver_req_queue",
            "blk_driver_resp_queue",
            "blk_client_req_queue",
            "blk_client_resp_queue",
        ]),
        None,
    )
}

fn fatfs(qos: Option<Qos>) -> Domain {
    domain(
        "fatfs",
        2,
        qos,
        "sddf-fatfs.elf",
        cached(&[
            "blk_storage_info",
            "blk_client_req_queue",
            "blk_client_resp_queue",
            "blk_data",
            "fs_command_queue",
            "fs_completion_queue",
            "fs_share",
        ]),
        None,
    )
}

struct NetVirtioAttachment {
    regions: Vec<Region>,
    driver_slots: Vec<Slot>,
    irq: Irq,
}

fn net_virtio_attachment(arch: Arch, net_pci_ecam: bool) -> NetVirtioAttachment {
    match virtio_node(arch, VirtioRole::Net).expect("network image includes virtio-net") {
        VirtioNode::Mmio(device) => {
            let mut regions = vec![window_region(&device.window)];
            regions.push(net_driver_dma_region(arch));
            let driver_slots = vec![
                mmio(device.window.name, device.setvar),
                dma("virtio_net_driver_dma", "virtio_net_driver_dma_vaddr"),
                setvar("virtio_net_driver_dma_paddr", "virtio_net_driver_dma"),
            ];
            NetVirtioAttachment {
                regions,
                driver_slots,
                irq: Irq::Line(device.irq),
            }
        }
        VirtioNode::Pci {
            config, irq, bars, ..
        } => {
            let mut regions = pci_bar_regions(bars);
            regions.push(net_driver_dma_region(arch));
            let mut driver_slots = Vec::new();
            if net_pci_ecam {
                use lerux_machine::{
                    X86_PCI_ECAM_GUEST, X86_PCI_ECAM_PHYS, X86_PCI_ECAM_SETVAR, X86_PCI_ECAM_SIZE,
                    X86_PCI_ECAM_WINDOW,
                };
                regions.push(Region {
                    name: X86_PCI_ECAM_WINDOW,
                    size: X86_PCI_ECAM_SIZE,
                    vaddr: X86_PCI_ECAM_GUEST,
                    phys_addr: Some(X86_PCI_ECAM_PHYS.text()),
                    page_size: None,
                });
                driver_slots.push(mmio(X86_PCI_ECAM_WINDOW, X86_PCI_ECAM_SETVAR));
            } else {
                driver_slots.push(Slot::Port(config));
            }
            for bar in bars {
                driver_slots.push(Slot::Map(Map {
                    region: bar.setvar,
                    cached: Some(false),
                    setvar: None,
                }));
            }
            driver_slots.extend([
                dma("virtio_net_driver_dma", "virtio_net_driver_dma_vaddr"),
                setvar("virtio_net_driver_dma_paddr", "virtio_net_driver_dma"),
            ]);
            NetVirtioAttachment {
                regions,
                driver_slots,
                irq: Irq::X86(irq),
            }
        }
    }
}

fn net_driver_dma_region(arch: Arch) -> Region {
    let size = 0x200_000;
    let vaddr = virtio_dma_guest(arch, VirtioRole::Net);
    if let Some(phys) = virtio_dma_phys_addr(arch, VirtioRole::Net) {
        Region {
            name: "virtio_net_driver_dma",
            size,
            vaddr,
            phys_addr: Some(phys),
            page_size: Some(size),
        }
    } else {
        paged("virtio_net_driver_dma", size, vaddr)
    }
}

fn pci_bar_regions(bars: &[PciBar]) -> Vec<Region> {
    const BAR_SIZES: [u64; 2] = [0x1_000, 0x4_000];
    bars.iter()
        .zip(BAR_SIZES)
        .map(|(bar, size)| Region {
            name: bar.setvar,
            size,
            vaddr: bar.phys.value(),
            phys_addr: Some(bar.phys.text()),
            page_size: None,
        })
        .collect()
}

fn net_virtio_regions(arch: Arch, net_pci_ecam: bool) -> Vec<Region> {
    net_virtio_attachment(arch, net_pci_ecam).regions
}

fn net_driver(arch: Arch, net_pci_ecam: bool, qos: Option<Qos>, priority: u8) -> Domain {
    let attachment = net_virtio_attachment(arch, net_pci_ecam);
    net_driver_with_irq(arch, net_pci_ecam, attachment.irq, qos, priority)
}

fn net_driver_with_irq(
    arch: Arch,
    net_pci_ecam: bool,
    irq: Irq,
    qos: Option<Qos>,
    priority: u8,
) -> Domain {
    let attachment = net_virtio_attachment(arch, net_pci_ecam);
    let mut slots = attachment.driver_slots;
    slots.extend(cached(&[
        "net_rx_drv_free",
        "net_rx_drv_active",
        "net_tx_drv_free",
        "net_tx_drv_active",
        "net_rx_data",
        "net_tx_data",
    ]));
    domain(
        "net_driver",
        priority,
        qos,
        "sddf-net-driver.elf",
        slots,
        Some(irq),
    )
}

fn net_virt_tx(qos: Option<Qos>) -> Domain {
    domain(
        "net_virt_tx",
        3,
        qos,
        "sddf-net-virt-tx.elf",
        cached(&[
            "net_tx_drv_free",
            "net_tx_drv_active",
            "net_tx_client_free",
            "net_tx_client_active",
            "net_tx_data",
        ]),
        None,
    )
}

fn net_virt_rx(qos: Option<Qos>) -> Domain {
    domain(
        "net_virt_rx",
        3,
        qos,
        "sddf-net-virt-rx.elf",
        cached(&[
            "net_rx_drv_free",
            "net_rx_drv_active",
            "net_rx_copy_free",
            "net_rx_copy_active",
            "net_rx_data",
            "net_rx_meta",
        ]),
        None,
    )
}

fn net_copy(qos: Option<Qos>) -> Domain {
    domain(
        "net_copy",
        2,
        qos,
        "sddf-net-copy.elf",
        cached(&[
            "net_rx_copy_free",
            "net_rx_copy_active",
            "net_rx_data",
            "net_rx_client_free",
            "net_rx_client_active",
            "net_rx_client_data",
        ]),
        None,
    )
}

fn gpu_driver(arch: Arch) -> Domain {
    let VirtioNode::Mmio(device) =
        virtio_node(arch, VirtioRole::Gpu).expect("browser image includes virtio-gpu")
    else {
        panic!("virtio-gpu is memory-mapped on this board");
    };
    let mut slots = vec![
        mmio(device.window.name, device.setvar),
        dma("virtio_gpu_driver_dma", "virtio_gpu_driver_dma_vaddr"),
        setvar("virtio_gpu_driver_dma_paddr", "virtio_gpu_driver_dma"),
    ];
    slots.extend(cached(&[
        "gpu_driver_events",
        "gpu_driver_req",
        "gpu_driver_resp",
        "gpu_driver_data",
        "gpu_client_data",
    ]));
    domain(
        "gpu_driver",
        4,
        None,
        "sddf-gpu-driver.elf",
        slots,
        Some(Irq::Line(device.irq)),
    )
}

fn gpu_virt() -> Domain {
    domain(
        "gpu_virt",
        3,
        None,
        "sddf-gpu-virt.elf",
        cached(&[
            "gpu_driver_events",
            "gpu_driver_req",
            "gpu_driver_resp",
            "gpu_driver_data",
            "gpu_client_events",
            "gpu_client_req",
            "gpu_client_resp",
            "gpu_client_data",
        ]),
        None,
    )
}

fn serial_client_slots() -> Vec<Slot> {
    cached(&[
        "serial_tx_client_queue",
        "serial_tx_client_data",
        "serial_rx_client_queue",
        "serial_rx_client_data",
    ])
}

fn fs_client_slots() -> Vec<Slot> {
    cached(&["fs_command_queue", "fs_completion_queue", "fs_share"])
}

fn net_client_slots() -> Vec<Slot> {
    cached(&[
        "net_rx_client_free",
        "net_rx_client_active",
        "net_tx_client_free",
        "net_tx_client_active",
        "net_rx_client_data",
        "net_tx_data",
    ])
}

fn fetch_client_slots() -> Vec<Slot> {
    let mut slots = serial_client_slots();
    slots.extend(fs_client_slots());
    slots.extend(net_client_slots());
    slots
}

fn serial_queue_regions() -> Vec<Region> {
    vec![
        ram("serial_tx_driver_queue", REGION_SIZE, 0x3_000_000),
        ram("serial_tx_driver_data", REGION_SIZE, 0x3_001_000),
        ram("serial_rx_driver_queue", REGION_SIZE, 0x3_002_000),
        ram("serial_rx_driver_data", REGION_SIZE, 0x3_003_000),
        ram("serial_tx_client_queue", REGION_SIZE, 0x3_004_000),
        ram("serial_tx_client_data", REGION_SIZE, 0x3_005_000),
        ram("serial_rx_client_queue", REGION_SIZE, 0x3_006_000),
        ram("serial_rx_client_data", REGION_SIZE, 0x3_007_000),
    ]
}

fn serial_driver_queue_maps() -> Vec<Slot> {
    cached(&[
        "serial_tx_driver_queue",
        "serial_tx_driver_data",
        "serial_rx_driver_queue",
        "serial_rx_driver_data",
    ])
}

fn window_region(window: &MmioWindow) -> Region {
    phys(window.name, window.size, window.vaddr, window.phys.text())
}

fn window_map(mmio: &MmioDevice) -> Slot {
    device(mmio.window.name, mmio.setvar)
}

fn virtio_regions(blk: bool, net: bool, gpu: bool) -> Vec<Region> {
    let mut group = vec![phys(
        "virtio_mmio",
        REGION_SIZE,
        0x6_000_000_000,
        "0xa003000",
    )];
    if blk {
        group.push(paged("virtio_blk_driver_dma", 0x200_000, 0x8_000_000_000));
    }
    if net {
        group.push(paged("virtio_net_driver_dma", 0x200_000, 0x8_000_000_000));
    }
    if gpu {
        group.push(paged("virtio_gpu_driver_dma", 0x200_000, 0x8_000_000_000));
    }
    group
}

fn blk_regions() -> Vec<Region> {
    vec![
        ram("blk_storage_info", REGION_SIZE, 0x4_000_000),
        ram("blk_driver_req_queue", REGION_SIZE, 0x4_001_000),
        ram("blk_driver_resp_queue", REGION_SIZE, 0x4_002_000),
        ram("blk_client_req_queue", REGION_SIZE, 0x4_003_000),
        ram("blk_client_resp_queue", REGION_SIZE, 0x4_004_000),
        ram("blk_data", 0x10_000, 0x4_010_000),
    ]
}

fn fs_regions() -> Vec<Region> {
    vec![
        ram("fs_command_queue", 0x8_000, 0x4_020_000),
        ram("fs_completion_queue", 0x8_000, 0x4_028_000),
        ram("fs_share", 0x8_000, 0x4_030_000),
    ]
}

fn net_regions(tool: bool) -> Vec<Region> {
    let mut group = vec![
        ram("net_rx_drv_free", REGION_SIZE, 0x5_000_000),
        ram("net_rx_drv_active", REGION_SIZE, 0x5_001_000),
        ram("net_tx_drv_free", REGION_SIZE, 0x5_002_000),
        ram("net_tx_drv_active", REGION_SIZE, 0x5_003_000),
        ram("net_rx_copy_free", REGION_SIZE, 0x5_004_000),
        ram("net_rx_copy_active", REGION_SIZE, 0x5_005_000),
        ram("net_rx_client_free", REGION_SIZE, 0x5_006_000),
        ram("net_rx_client_active", REGION_SIZE, 0x5_007_000),
        ram("net_tx_client_free", REGION_SIZE, 0x5_008_000),
        ram("net_tx_client_active", REGION_SIZE, 0x5_009_000),
        ram("net_rx_data", 0x8_000, 0x5_010_000),
        ram("net_tx_data", 0x8_000, 0x5_020_000),
        ram("net_rx_client_data", 0x8_000, 0x5_030_000),
        ram("net_rx_meta", REGION_SIZE, 0x5_040_000),
    ];
    if tool {
        group.push(ram("agent_tool", REGION_SIZE, 0xa_000_000));
    }
    group
}

fn gpu_regions() -> Vec<Region> {
    vec![
        ram("gpu_driver_events", REGION_SIZE, 0x7_000_000),
        ram("gpu_driver_req", REGION_SIZE, 0x7_001_000),
        ram("gpu_driver_resp", REGION_SIZE, 0x7_002_000),
        ram("gpu_driver_data", REGION_SIZE, 0x7_003_000),
        ram("gpu_client_events", REGION_SIZE, 0x7_004_000),
        ram("gpu_client_req", REGION_SIZE, 0x7_005_000),
        ram("gpu_client_resp", REGION_SIZE, 0x7_006_000),
        ram("gpu_client_data", REGION_SIZE, 0x7_007_000),
    ]
}

fn domain(
    name: &'static str,
    priority: u8,
    qos: Option<Qos>,
    program: &'static str,
    slots: Vec<Slot>,
    irq: Option<Irq>,
) -> Domain {
    Domain {
        name,
        priority,
        qos,
        stack: STACK_SIZE,
        program,
        slots,
        irq,
    }
}

fn budget(budget: u32) -> Option<Qos> {
    Some(Qos {
        budget,
        passive: false,
    })
}

fn link_rows(rows: &[(&'static str, &'static str, u8)]) -> Vec<Link> {
    rows.iter()
        .map(|(a, b, order)| Link {
            a,
            b,
            pp_b: false,
            order: *order,
        })
        .collect()
}

fn ram(name: &'static str, size: u64, vaddr: u64) -> Region {
    Region {
        name,
        size,
        vaddr,
        phys_addr: None,
        page_size: None,
    }
}

fn phys(name: &'static str, size: u64, vaddr: u64, phys_addr: &'static str) -> Region {
    Region {
        name,
        size,
        vaddr,
        phys_addr: Some(phys_addr),
        page_size: None,
    }
}

fn paged(name: &'static str, size: u64, vaddr: u64) -> Region {
    Region {
        name,
        size,
        vaddr,
        phys_addr: None,
        page_size: Some(size),
    }
}

fn device(region: &'static str, setvar: &'static str) -> Slot {
    Slot::Map(Map {
        region,
        cached: None,
        setvar: Some(setvar),
    })
}

fn mmio(region: &'static str, setvar: &'static str) -> Slot {
    Slot::Map(Map {
        region,
        cached: Some(false),
        setvar: Some(setvar),
    })
}

fn dma(region: &'static str, setvar: &'static str) -> Slot {
    Slot::Map(Map {
        region,
        cached: Some(true),
        setvar: Some(setvar),
    })
}

fn setvar(symbol: &'static str, region: &'static str) -> Slot {
    Slot::Setvar { symbol, region }
}

fn cached(names: &[&'static str]) -> Vec<Slot> {
    names
        .iter()
        .copied()
        .map(|region| {
            Slot::Map(Map {
                region,
                cached: Some(true),
                setvar: None,
            })
        })
        .collect()
}
