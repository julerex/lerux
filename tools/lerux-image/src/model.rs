//! Regions, protection domains, and links for one device-framework image.
//!
//! Rows do not name channel ids. Assignment gives each protection domain the
//! next free id, interrupts first, then links in `order`. The vector order is
//! the system-description order.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub(crate) const REGION_SIZE: u64 = 0x1_000;
pub(crate) const STACK_SIZE: u64 = 0x10_000;

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

pub(crate) struct Region {
    pub name: &'static str,
    pub size: u64,
    pub vaddr: u64,
    pub phys_addr: Option<&'static str>,
    pub page_size: Option<u64>,
}

pub(crate) struct Qos {
    pub budget: u32,
    pub passive: bool,
}

pub(crate) struct Domain {
    pub name: &'static str,
    pub priority: u8,
    pub qos: Option<Qos>,
    pub stack: u64,
    pub program: &'static str,
    pub slots: Vec<Slot>,
    pub irq_line: Option<u32>,
}

pub(crate) enum Slot {
    Map(Map),
    Setvar {
        symbol: &'static str,
        region: &'static str,
    },
}

pub(crate) struct Map {
    pub region: &'static str,
    pub cached: Option<bool>,
    pub setvar: Option<&'static str>,
}

pub(crate) struct Link {
    pub a: &'static str,
    pub b: &'static str,
    pub pp_b: bool,
    /// Assignment sequence. Lower runs first. Emission follows `Image::links`.
    pub order: u8,
}

pub(crate) struct Image {
    pub groups: Vec<Vec<Region>>,
    pub domains: Vec<Domain>,
    pub links: Vec<Link>,
}

struct AssignedChannel {
    a_pd: &'static str,
    a_id: u8,
    b_pd: &'static str,
    b_id: u8,
    pp_b: bool,
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

pub(crate) fn render_image(image: &Image) -> Result<String, ImageError> {
    render_assigned(image, assign(image)?)
}

#[cfg(test)]
pub(crate) fn render_forcing_end(
    image: &Image,
    pd: &'static str,
    id: u8,
) -> Result<String, ImageError> {
    let mut assigned = assign(image)?;
    assigned.ends.push(AssignedEnd { pd, id });
    render_assigned(image, assigned)
}

fn render_assigned(image: &Image, assigned: Assigned) -> Result<String, ImageError> {
    validate(&assigned.ends)?;
    write_xml(image, &assigned)
}

fn assign(image: &Image) -> Result<Assigned, ImageError> {
    let mut next = BTreeMap::new();
    let mut irqs = Vec::new();
    let mut ends = Vec::new();
    for domain in &image.domains {
        if domain.irq_line.is_some() {
            let id = take_id(&mut next, domain.name)?;
            irqs.push(AssignedIrq {
                pd: domain.name,
                id,
            });
            ends.push(AssignedEnd {
                pd: domain.name,
                id,
            });
        }
    }
    let mut sequence: Vec<usize> = (0..image.links.len()).collect();
    sequence.sort_by_key(|&index| image.links[index].order);
    let mut ids = vec![(0u8, 0u8); image.links.len()];
    for index in sequence {
        let link = &image.links[index];
        let a_id = take_id(&mut next, link.a)?;
        let b_id = take_id(&mut next, link.b)?;
        ids[index] = (a_id, b_id);
        ends.push(AssignedEnd {
            pd: link.a,
            id: a_id,
        });
        ends.push(AssignedEnd {
            pd: link.b,
            id: b_id,
        });
    }
    let channels = image
        .links
        .iter()
        .enumerate()
        .map(|(index, link)| AssignedChannel {
            a_pd: link.a,
            a_id: ids[index].0,
            b_pd: link.b,
            b_id: ids[index].1,
            pp_b: link.pp_b,
        })
        .collect();
    Ok(Assigned {
        channels,
        ends,
        irqs,
    })
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
    for (index, group) in image.groups.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        for region in group {
            out.push_str(&region_line(region));
            out.push('\n');
        }
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
    if let Some(phys) = region.phys_addr {
        return format!(
            "    <memory_region name=\"{}\" size=\"{size}\" phys_addr=\"{phys}\" />",
            region.name
        );
    }
    if let Some(page) = region.page_size {
        return format!(
            "    <memory_region name=\"{}\" size=\"{size}\" page_size=\"{}\" />",
            region.name,
            hex_group(page)
        );
    }
    format!(
        "    <memory_region name=\"{}\" size=\"{size}\" />",
        region.name
    )
}

fn domain_block(image: &Image, domain: &Domain, assigned: &Assigned) -> Result<String, ImageError> {
    let mut out = format!(
        "    <protection_domain name=\"{}\" {} stack_size=\"{}\">\n        <program_image path=\"{}\" />\n",
        domain.name,
        domain_attrs(domain),
        hex_group(domain.stack),
        domain.program
    );
    for slot in &domain.slots {
        match slot {
            Slot::Map(map) => {
                let region = find_region(&image.groups, map.region)?;
                out.push_str(&map_line(region, map));
                out.push('\n');
            }
            Slot::Setvar { symbol, region } => {
                out.push_str(&format!(
                    "        <setvar symbol=\"{symbol}\" region_paddr=\"{region}\" />\n"
                ));
            }
        }
    }
    if let Some(line) = domain.irq_line {
        let id = irq_id(&assigned.irqs, domain.name)?;
        out.push_str(&format!("        <irq irq=\"{line}\" id=\"{id}\" />\n"));
    }
    out.push_str("    </protection_domain>\n");
    Ok(out)
}

fn domain_attrs(domain: &Domain) -> String {
    match &domain.qos {
        None => format!("priority=\"{}\"", domain.priority),
        Some(qos) if qos.passive => format!(
            "priority=\"{}\" budget=\"{}\" period=\"{}\" passive=\"true\"",
            domain.priority, qos.budget, qos.budget
        ),
        Some(qos) => format!(
            "priority=\"{}\" budget=\"{}\" period=\"{}\"",
            domain.priority, qos.budget, qos.budget
        ),
    }
}

fn map_line(region: &Region, map: &Map) -> String {
    let vaddr = hex_group(region.vaddr);
    let mut line = format!(
        "        <map mr=\"{}\" vaddr=\"{vaddr}\" perms=\"rw\"",
        map.region
    );
    if let Some(cached) = map.cached {
        line.push_str(&format!(" cached=\"{cached}\""));
    }
    if let Some(setvar) = map.setvar {
        line.push_str(&format!(" setvar_vaddr=\"{setvar}\""));
    }
    line.push_str(" />");
    line
}

fn channel_block(channel: &AssignedChannel) -> String {
    let b_pp = if channel.pp_b { " pp=\"true\"" } else { "" };
    format!(
        "    <channel>\n        <end pd=\"{}\" id=\"{}\" />\n        <end pd=\"{}\" id=\"{}\"{b_pp} />\n    </channel>\n",
        channel.a_pd, channel.a_id, channel.b_pd, channel.b_id
    )
}

pub(crate) fn find_region<'a>(
    groups: &'a [Vec<Region>],
    name: &str,
) -> Result<&'a Region, ImageError> {
    groups
        .iter()
        .flatten()
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

pub(crate) fn hex_group(value: u64) -> String {
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
