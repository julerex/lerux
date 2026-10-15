//! The one-client serial image.
//!
//! Rows name regions, protection domains, maps, and links. They do not name
//! channel ids. Assignment gives each protection domain the next free id,
//! interrupts first.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const REGION_SIZE: u64 = 0x1_000;
const STACK_SIZE: u64 = 0x10_000;

#[derive(Debug)]
pub enum ImageError {
    DuplicateEnd { pd: String, id: u8 },
    UnknownRegion { name: String },
    ChannelOverflow { pd: String },
    MissingIrq { pd: String },
}

impl fmt::Display for ImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::DuplicateEnd { pd, id } => write!(formatter, "duplicate end {pd} {id}"),
            ImageError::UnknownRegion { name } => write!(formatter, "unknown region {name}"),
            ImageError::ChannelOverflow { pd } => {
                write!(formatter, "channel id overflow on {pd}")
            }
            ImageError::MissingIrq { pd } => write!(formatter, "missing irq id for {pd}"),
        }
    }
}

impl std::error::Error for ImageError {}

struct Region {
    name: &'static str,
    size: u64,
    vaddr: u64,
    phys_addr: Option<&'static str>,
}

struct Domain {
    name: &'static str,
    priority: u8,
    stack: u64,
    program: &'static str,
    maps: Vec<Map>,
    irq_line: Option<u32>,
}

struct Map {
    region: &'static str,
    kind: MapKind,
}

enum MapKind {
    Device { setvar: &'static str },
    Cached,
}

struct Link {
    a: &'static str,
    b: &'static str,
}

struct Image {
    regions: Vec<Region>,
    domains: Vec<Domain>,
    links: Vec<Link>,
}

struct AssignedChannel {
    a_pd: &'static str,
    a_id: u8,
    b_pd: &'static str,
    b_id: u8,
}

struct AssignedEnd {
    pd: &'static str,
    id: u8,
}

struct AssignedIrq {
    pd: &'static str,
    id: u8,
}

struct Assigned {
    channels: Vec<AssignedChannel>,
    ends: Vec<AssignedEnd>,
    irqs: Vec<AssignedIrq>,
}

pub(crate) fn render_serial() -> Result<String, ImageError> {
    render_image(&serial_image())
}

#[cfg(test)]
pub(crate) fn render_with_region_vaddr(name: &str, vaddr: u64) -> Result<String, ImageError> {
    let mut image = serial_image();
    {
        let region = image
            .regions
            .iter_mut()
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
    let image = serial_image();
    let mut assigned = assign(&image)?;
    assigned.ends.push(AssignedEnd { pd, id });
    render_assigned(&image, assigned)
}

fn render_image(image: &Image) -> Result<String, ImageError> {
    render_assigned(image, assign(image)?)
}

fn render_assigned(image: &Image, assigned: Assigned) -> Result<String, ImageError> {
    validate(&assigned.ends)?;
    write_xml(image, &assigned)
}

fn serial_image() -> Image {
    Image {
        regions: vec![
            Region {
                name: "serial_mmio",
                size: REGION_SIZE,
                vaddr: 0x2_000_000,
                phys_addr: Some("0x9_000_000"),
            },
            Region {
                name: "serial_tx_driver_queue",
                size: REGION_SIZE,
                vaddr: 0x3_000_000,
                phys_addr: None,
            },
            Region {
                name: "serial_tx_driver_data",
                size: REGION_SIZE,
                vaddr: 0x3_001_000,
                phys_addr: None,
            },
            Region {
                name: "serial_rx_driver_queue",
                size: REGION_SIZE,
                vaddr: 0x3_002_000,
                phys_addr: None,
            },
            Region {
                name: "serial_rx_driver_data",
                size: REGION_SIZE,
                vaddr: 0x3_003_000,
                phys_addr: None,
            },
            Region {
                name: "serial_tx_client_queue",
                size: REGION_SIZE,
                vaddr: 0x3_004_000,
                phys_addr: None,
            },
            Region {
                name: "serial_tx_client_data",
                size: REGION_SIZE,
                vaddr: 0x3_005_000,
                phys_addr: None,
            },
            Region {
                name: "serial_rx_client_queue",
                size: REGION_SIZE,
                vaddr: 0x3_006_000,
                phys_addr: None,
            },
            Region {
                name: "serial_rx_client_data",
                size: REGION_SIZE,
                vaddr: 0x3_007_000,
                phys_addr: None,
            },
        ],
        domains: vec![
            Domain {
                name: "serial_driver",
                priority: 4,
                stack: STACK_SIZE,
                program: "sddf-serial-driver.elf",
                irq_line: Some(33),
                maps: vec![
                    Map {
                        region: "serial_mmio",
                        kind: MapKind::Device {
                            setvar: "serial_register_block",
                        },
                    },
                    Map {
                        region: "serial_tx_driver_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_tx_driver_data",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_driver_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_driver_data",
                        kind: MapKind::Cached,
                    },
                ],
            },
            Domain {
                name: "serial_virt_tx",
                priority: 3,
                stack: STACK_SIZE,
                program: "sddf-serial-virt-tx.elf",
                irq_line: None,
                maps: vec![
                    Map {
                        region: "serial_tx_driver_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_tx_driver_data",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_tx_client_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_tx_client_data",
                        kind: MapKind::Cached,
                    },
                ],
            },
            Domain {
                name: "serial_virt_rx",
                priority: 3,
                stack: STACK_SIZE,
                program: "sddf-serial-virt-rx.elf",
                irq_line: None,
                maps: vec![
                    Map {
                        region: "serial_rx_driver_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_driver_data",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_client_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_client_data",
                        kind: MapKind::Cached,
                    },
                ],
            },
            Domain {
                name: "serial_client",
                priority: 1,
                stack: STACK_SIZE,
                program: "sddf-serial-client.elf",
                irq_line: None,
                maps: vec![
                    Map {
                        region: "serial_tx_client_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_tx_client_data",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_client_queue",
                        kind: MapKind::Cached,
                    },
                    Map {
                        region: "serial_rx_client_data",
                        kind: MapKind::Cached,
                    },
                ],
            },
        ],
        links: vec![
            Link {
                a: "serial_driver",
                b: "serial_virt_tx",
            },
            Link {
                a: "serial_driver",
                b: "serial_virt_rx",
            },
            Link {
                a: "serial_virt_tx",
                b: "serial_client",
            },
            Link {
                a: "serial_virt_rx",
                b: "serial_client",
            },
        ],
    }
}

fn assign(image: &Image) -> Result<Assigned, ImageError> {
    let mut next = BTreeMap::new();
    let mut assigned = Assigned {
        channels: Vec::new(),
        ends: Vec::new(),
        irqs: Vec::new(),
    };
    for domain in &image.domains {
        if domain.irq_line.is_some() {
            let id = take_id(&mut next, domain.name)?;
            assigned.irqs.push(AssignedIrq {
                pd: domain.name,
                id,
            });
            assigned.ends.push(AssignedEnd {
                pd: domain.name,
                id,
            });
        }
    }
    for link in &image.links {
        let a_id = take_id(&mut next, link.a)?;
        let b_id = take_id(&mut next, link.b)?;
        assigned.channels.push(AssignedChannel {
            a_pd: link.a,
            a_id,
            b_pd: link.b,
            b_id,
        });
        assigned.ends.push(AssignedEnd {
            pd: link.a,
            id: a_id,
        });
        assigned.ends.push(AssignedEnd {
            pd: link.b,
            id: b_id,
        });
    }
    Ok(assigned)
}

fn take_id(next: &mut BTreeMap<&'static str, u8>, pd: &'static str) -> Result<u8, ImageError> {
    let id = next.get(pd).copied().unwrap_or(0);
    let bumped = id
        .checked_add(1)
        .ok_or_else(|| ImageError::ChannelOverflow { pd: pd.to_string() })?;
    next.insert(pd, bumped);
    Ok(id)
}

fn validate(ends: &[AssignedEnd]) -> Result<(), ImageError> {
    let mut seen = BTreeSet::new();
    for end in ends {
        if !seen.insert((end.pd, end.id)) {
            return Err(ImageError::DuplicateEnd {
                pd: end.pd.to_string(),
                id: end.id,
            });
        }
    }
    Ok(())
}

fn write_xml(image: &Image, assigned: &Assigned) -> Result<String, ImageError> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<system>\n");
    for region in &image.regions {
        out.push_str(&region_line(region));
        out.push('\n');
    }
    out.push('\n');
    for domain in &image.domains {
        out.push_str(&domain_block(image, domain, assigned)?);
        out.push('\n');
    }
    for channel in &assigned.channels {
        out.push_str(&channel_block(channel));
    }
    out.push_str("</system>\n");
    Ok(out)
}

fn region_line(region: &Region) -> String {
    let size = hex_group(region.size);
    match region.phys_addr {
        Some(phys) => format!(
            "    <memory_region name=\"{}\" size=\"{size}\" phys_addr=\"{phys}\" />",
            region.name
        ),
        None => format!(
            "    <memory_region name=\"{}\" size=\"{size}\" />",
            region.name
        ),
    }
}

fn domain_block(image: &Image, domain: &Domain, assigned: &Assigned) -> Result<String, ImageError> {
    let mut out = format!(
        "    <protection_domain name=\"{}\" priority=\"{}\" stack_size=\"{}\">\n        <program_image path=\"{}\" />\n",
        domain.name,
        domain.priority,
        hex_group(domain.stack),
        domain.program
    );
    for map in &domain.maps {
        let region = find_region(&image.regions, map.region)?;
        out.push_str(&map_line(region, map));
        out.push('\n');
    }
    if let Some(line) = domain.irq_line {
        let id = irq_id(&assigned.irqs, domain.name)?;
        out.push_str(&format!("        <irq irq=\"{line}\" id=\"{id}\" />\n"));
    }
    out.push_str("    </protection_domain>\n");
    Ok(out)
}

fn map_line(region: &Region, map: &Map) -> String {
    let vaddr = hex_group(region.vaddr);
    match map.kind {
        MapKind::Device { setvar } => format!(
            "        <map mr=\"{}\" vaddr=\"{vaddr}\" perms=\"rw\" setvar_vaddr=\"{setvar}\" />",
            map.region
        ),
        MapKind::Cached => format!(
            "        <map mr=\"{}\" vaddr=\"{vaddr}\" perms=\"rw\" cached=\"true\" />",
            map.region
        ),
    }
}

fn channel_block(channel: &AssignedChannel) -> String {
    format!(
        "    <channel>\n        <end pd=\"{}\" id=\"{}\" />\n        <end pd=\"{}\" id=\"{}\" />\n    </channel>\n",
        channel.a_pd, channel.a_id, channel.b_pd, channel.b_id
    )
}

fn find_region<'a>(regions: &'a [Region], name: &str) -> Result<&'a Region, ImageError> {
    regions
        .iter()
        .find(|region| region.name == name)
        .ok_or_else(|| ImageError::UnknownRegion {
            name: name.to_string(),
        })
}

fn irq_id(irqs: &[AssignedIrq], pd: &str) -> Result<u8, ImageError> {
    irqs.iter()
        .find(|irq| irq.pd == pd)
        .map(|irq| irq.id)
        .ok_or_else(|| ImageError::MissingIrq { pd: pd.to_string() })
}

fn hex_group(value: u64) -> String {
    let hex = format!("{value:x}");
    let mut grouped = String::new();
    for (index, ch) in hex.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('_');
        }
        grouped.push(ch);
    }
    let body: String = grouped.chars().rev().collect();
    format!("0x{body}")
}
