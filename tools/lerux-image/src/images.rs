//! Device-framework images.
//!
//! Web is the fetch image with a different mode. It has no second description.

#[cfg(test)]
use crate::model::render_forcing_end;
use crate::model::{
    render_image, Domain, Image, ImageError, Link, Map, Qos, Region, Slot, REGION_SIZE, STACK_SIZE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Serial,
    Fs,
    Net,
    Fetch,
    Browser,
    Agent,
    Program,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Fetch,
    Web,
}

pub(crate) fn render_serial() -> Result<String, ImageError> {
    render_kind(ImageKind::Serial)
}

pub fn render_kind(kind: ImageKind) -> Result<String, ImageError> {
    render_image(&image(kind))
}

pub fn board_registration(board: &str) -> Option<(ImageKind, Option<Mode>)> {
    match board {
        "qemu_virt_aarch64_serial_sddf" => Some((ImageKind::Serial, None)),
        "qemu_virt_aarch64_fs_sddf" => Some((ImageKind::Fs, None)),
        "qemu_virt_aarch64_net_sddf" => Some((ImageKind::Net, None)),
        "qemu_virt_aarch64_fetch_sddf" => Some((ImageKind::Fetch, Some(Mode::Fetch))),
        "qemu_virt_aarch64_web_sddf" => Some((ImageKind::Fetch, Some(Mode::Web))),
        "qemu_virt_aarch64_browser_sddf" => Some((ImageKind::Browser, None)),
        "qemu_virt_aarch64_agent_sddf" => Some((ImageKind::Agent, None)),
        "qemu_virt_aarch64_program_sddf" => Some((ImageKind::Program, None)),
        _ => None,
    }
}

pub fn render_board(board: &str) -> Result<Option<String>, ImageError> {
    let Some((kind, _)) = board_registration(board) else {
        return Ok(None);
    };
    render_kind(kind).map(Some)
}

#[cfg(test)]
pub(crate) fn render_with_region_vaddr(name: &str, vaddr: u64) -> Result<String, ImageError> {
    let mut image = image(ImageKind::Serial);
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
    render_forcing_end(&image(ImageKind::Serial), pd, id)
}

fn image(kind: ImageKind) -> Image {
    match kind {
        ImageKind::Serial => serial_image(),
        ImageKind::Fs => fs_image(),
        ImageKind::Net => net_image(),
        ImageKind::Fetch => fetch_image("shell", "sddf-shell.elf", fetch_client_slots()),
        ImageKind::Browser => browser_image(),
        ImageKind::Agent => agent_image(),
        ImageKind::Program => fetch_image("program", "sddf-program.elf", fetch_client_slots()),
    }
}

fn serial_image() -> Image {
    Image {
        groups: vec![serial_regions()],
        domains: vec![
            serial_driver(None),
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

fn fs_image() -> Image {
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
            serial_regions(),
            virtio_regions(true, false, false),
            blk_regions(),
            fs_regions(),
        ],
        domains: vec![
            blk_driver(budget(4000)),
            serial_driver(budget(4000)),
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

fn net_image() -> Image {
    let mut client = serial_client_slots();
    client.extend(net_client_slots());
    Image {
        groups: vec![
            serial_regions(),
            virtio_regions(false, true, false),
            net_regions(false),
        ],
        domains: vec![
            net_driver(),
            serial_driver(None),
            net_virt_tx(),
            net_virt_rx(),
            serial_virt_tx(None),
            serial_virt_rx(None),
            net_copy(),
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

fn fetch_image(client: &'static str, program: &'static str, slots: Vec<Slot>) -> Image {
    Image {
        groups: vec![
            serial_regions(),
            virtio_regions(true, true, false),
            blk_regions(),
            fs_regions(),
            net_regions(false),
        ],
        domains: vec![
            blk_driver(None),
            net_driver(),
            serial_driver(None),
            blk_virt(None),
            net_virt_tx(),
            net_virt_rx(),
            serial_virt_tx(None),
            serial_virt_rx(None),
            fatfs(None),
            net_copy(),
            domain(client, 1, None, program, slots, None),
        ],
        links: fabric_links(client),
    }
}

fn browser_image() -> Image {
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
            serial_regions(),
            virtio_regions(true, true, true),
            blk_regions(),
            fs_regions(),
            net_regions(false),
            gpu_regions(),
        ],
        domains: vec![
            blk_driver(None),
            net_driver(),
            gpu_driver(),
            serial_driver(None),
            blk_virt(None),
            net_virt_tx(),
            net_virt_rx(),
            serial_virt_tx(None),
            serial_virt_rx(None),
            gpu_virt(),
            fatfs(None),
            net_copy(),
            domain("shell", 1, None, "sddf-shell.elf", slots, None),
        ],
        links,
    }
}

fn agent_image() -> Image {
    let mut grok = serial_client_slots();
    grok.extend(cached(&["agent_tool"]));
    let mut agent = fs_client_slots();
    agent.extend(net_client_slots());
    agent.extend(cached(&["agent_tool"]));
    Image {
        groups: vec![
            serial_regions(),
            virtio_regions(true, true, false),
            blk_regions(),
            fs_regions(),
            net_regions(true),
        ],
        domains: vec![
            blk_driver(None),
            net_driver(),
            serial_driver(None),
            blk_virt(None),
            net_virt_tx(),
            net_virt_rx(),
            serial_virt_tx(None),
            serial_virt_rx(None),
            fatfs(None),
            net_copy(),
            domain("shell", 1, None, "sddf-grok.elf", grok, None),
            domain("agent", 2, None, "sddf-agent.elf", agent, None),
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

fn serial_driver(qos: Option<Qos>) -> Domain {
    let mut slots = vec![device("serial_mmio", "serial_register_block")];
    slots.extend(cached(&[
        "serial_tx_driver_queue",
        "serial_tx_driver_data",
        "serial_rx_driver_queue",
        "serial_rx_driver_data",
    ]));
    domain(
        "serial_driver",
        4,
        qos,
        "sddf-serial-driver.elf",
        slots,
        Some(33),
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

fn blk_driver(qos: Option<Qos>) -> Domain {
    let mut slots = vec![
        mmio("virtio_mmio", "virtio_blk_mmio_vaddr"),
        dma("virtio_blk_driver_dma", "virtio_blk_driver_dma_vaddr"),
        setvar("virtio_blk_driver_dma_paddr", "virtio_blk_driver_dma"),
    ];
    slots.extend(cached(&[
        "blk_storage_info",
        "blk_driver_req_queue",
        "blk_driver_resp_queue",
        "blk_data",
    ]));
    domain("blk_driver", 4, qos, "sddf-blk-driver.elf", slots, Some(78))
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

fn net_driver() -> Domain {
    let mut slots = vec![
        mmio("virtio_mmio", "virtio_net_mmio_vaddr"),
        dma("virtio_net_driver_dma", "virtio_net_driver_dma_vaddr"),
        setvar("virtio_net_driver_dma_paddr", "virtio_net_driver_dma"),
    ];
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
        4,
        None,
        "sddf-net-driver.elf",
        slots,
        Some(79),
    )
}

fn net_virt_tx() -> Domain {
    domain(
        "net_virt_tx",
        3,
        None,
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

fn net_virt_rx() -> Domain {
    domain(
        "net_virt_rx",
        3,
        None,
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

fn net_copy() -> Domain {
    domain(
        "net_copy",
        2,
        None,
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

fn gpu_driver() -> Domain {
    let mut slots = vec![
        mmio("virtio_mmio", "virtio_gpu_mmio_vaddr"),
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
        Some(77),
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

fn serial_regions() -> Vec<Region> {
    vec![
        phys("serial_mmio", REGION_SIZE, 0x2_000_000, "0x9_000_000"),
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
    irq_line: Option<u32>,
) -> Domain {
    Domain {
        name,
        priority,
        qos,
        stack: STACK_SIZE,
        program,
        slots,
        irq_line,
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
