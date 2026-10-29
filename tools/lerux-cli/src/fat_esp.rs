//! FAT16 image for the ISO's EFI system partition.
//!
//! Firmware boots `EFI/BOOT/BOOTX64.EFI` from this volume. GRUB reads
//! `boot/grub/grub.cfg` and the modules from it too. A module that lives only
//! on the ISO9660 session stalls when firmware boots the partition.

use std::collections::BTreeSet;

use anyhow::{bail, Result};

const BPS: usize = 512;
const SPC: usize = 4;
const RESERVED: usize = 1;
const NFATS: usize = 2;
const ROOT_ENTRIES: usize = 512;
const ROOT_BYTES: usize = ROOT_ENTRIES * 32;
const CLUSTER_BYTES: usize = BPS * SPC;
/// GRUB counts the two reserved FAT entries, then treats `num_clusters <= 4087`
/// as FAT12. A volume of exactly 4085 data clusters is FAT16 in the specification
/// and FAT12 to GRUB, so the chain walk returns a short read. 4086 is FAT16 for both.
const MIN_CLUSTERS: usize = 4086;

/// Build a FAT16 volume whose root contains `files`.
///
/// Paths use `/` and are relative to the volume (`EFI/BOOT/BOOTX64.EFI`).
pub fn write_image(files: &[(&str, &[u8])]) -> Result<Vec<u8>> {
    let tree = tree_from(files)?;
    let dir_count = count_dirs(&tree);
    let file_clusters: usize = files
        .iter()
        .map(|(_, bytes)| clusters_for(bytes.len()))
        .sum();
    let clusters = (dir_count + file_clusters).max(MIN_CLUSTERS);
    if clusters > 0xFFF4 {
        bail!("EFI partition payload does not fit in a FAT16 volume");
    }
    let spf = (clusters + 2) * 2;
    let spf = spf.div_ceil(BPS);
    let root_sectors = ROOT_BYTES / BPS;
    let total_sectors = RESERVED + NFATS * spf + root_sectors + clusters * SPC;

    let mut image = vec![0u8; total_sectors * BPS];
    write_boot_sector(&mut image, spf as u16, total_sectors as u32);

    let mut fat = vec![0u16; clusters + 2];
    fat[0] = 0xFFF8;
    fat[1] = 0xFFFF;
    let mut next = 2usize;
    let data_off = (RESERVED + NFATS * spf + root_sectors) * BPS;
    let mut data = vec![0u8; clusters * CLUSTER_BYTES];

    let root_bytes = write_dir(&tree, 0, 0, &mut fat, &mut data, &mut next, true)?;
    let mut root = vec![0u8; ROOT_BYTES];
    root[..root_bytes.len()].copy_from_slice(&root_bytes);

    for (slot, entry) in fat.iter().enumerate() {
        let off = RESERVED * BPS + slot * 2;
        image[off..off + 2].copy_from_slice(&entry.to_le_bytes());
        let fat2 = off + spf * BPS;
        image[fat2..fat2 + 2].copy_from_slice(&entry.to_le_bytes());
    }
    let root_off = (RESERVED + NFATS * spf) * BPS;
    image[root_off..root_off + ROOT_BYTES].copy_from_slice(&root);
    image[data_off..data_off + data.len()].copy_from_slice(&data);
    Ok(image)
}

fn clusters_for(len: usize) -> usize {
    if len == 0 {
        0
    } else {
        len.div_ceil(CLUSTER_BYTES)
    }
}

struct Node {
    name: String,
    kind: Kind,
}

enum Kind {
    Dir(Vec<Node>),
    File(Vec<u8>),
}

fn tree_from(files: &[(&str, &[u8])]) -> Result<Vec<Node>> {
    let mut root = Vec::new();
    let mut seen = BTreeSet::new();
    for (path, bytes) in files {
        if !seen.insert(*path) {
            bail!("duplicate EFI partition path {path}");
        }
        insert(&mut root, path, bytes)?;
    }
    Ok(root)
}

fn insert(dir: &mut Vec<Node>, path: &str, bytes: &[u8]) -> Result<()> {
    let mut parts = path.split('/');
    let Some(first) = parts.next() else {
        bail!("empty EFI partition path");
    };
    if first.is_empty() || first == "." || first == ".." || first.len() > 255 {
        bail!("invalid EFI partition path {path}");
    }
    match parts.next() {
        None => {
            if dir.iter().any(|n| n.name == first) {
                bail!("EFI partition path {path} collides with a directory");
            }
            dir.push(Node {
                name: first.to_string(),
                kind: Kind::File(bytes.to_vec()),
            });
            Ok(())
        }
        Some(rest_first) => {
            let rest = path[first.len() + 1..].to_string();
            if rest_first.is_empty() {
                bail!("invalid EFI partition path {path}");
            }
            if let Some(node) = dir.iter_mut().find(|n| n.name == first) {
                let Kind::Dir(children) = &mut node.kind else {
                    bail!("EFI partition path {path} collides with a file");
                };
                return insert(children, &rest, bytes);
            }
            let mut child = Vec::new();
            insert(&mut child, &rest, bytes)?;
            dir.push(Node {
                name: first.to_string(),
                kind: Kind::Dir(child),
            });
            Ok(())
        }
    }
}

fn count_dirs(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match &node.kind {
            Kind::Dir(children) => 1 + count_dirs(children),
            Kind::File(_) => 0,
        })
        .sum()
}

fn write_boot_sector(image: &mut [u8], spf: u16, total_sectors: u32) {
    image[0] = 0xEB;
    image[1] = 0x3C;
    image[2] = 0x90;
    image[3..11].copy_from_slice(b"LERUX   ");
    image[11..13].copy_from_slice(&512u16.to_le_bytes());
    image[13] = SPC as u8;
    image[14..16].copy_from_slice(&(RESERVED as u16).to_le_bytes());
    image[16] = NFATS as u8;
    image[17..19].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
    if total_sectors <= u16::MAX as u32 {
        image[19..21].copy_from_slice(&(total_sectors as u16).to_le_bytes());
    }
    image[21] = 0xF8;
    image[22..24].copy_from_slice(&spf.to_le_bytes());
    image[24..26].copy_from_slice(&63u16.to_le_bytes());
    image[26..28].copy_from_slice(&255u16.to_le_bytes());
    image[32..36].copy_from_slice(&total_sectors.to_le_bytes());
    image[36] = 0x80;
    image[38] = 0x29;
    image[39..43].copy_from_slice(&0x4C45_5258u32.to_le_bytes());
    image[43..54].copy_from_slice(b"LERUX      ");
    image[54..62].copy_from_slice(b"FAT16   ");
    image[510] = 0x55;
    image[511] = 0xAA;
}

fn write_dir(
    nodes: &[Node],
    self_cluster: u16,
    parent_cluster: u16,
    fat: &mut [u16],
    data: &mut [u8],
    next: &mut usize,
    is_root: bool,
) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    let mut used_short = BTreeSet::new();
    if !is_root {
        push_dot(&mut raw, b".          ", self_cluster);
        push_dot(&mut raw, b"..         ", parent_cluster);
    }
    for node in nodes {
        let short = short_name(&node.name, &mut used_short)?;
        push_lfn(&mut raw, &node.name, lfn_checksum(&short));
        match &node.kind {
            Kind::Dir(children) => {
                let cluster = alloc_one(fat, next)?;
                push_short(&mut raw, &short, 0x10, cluster, 0);
                let parent = if is_root { 0 } else { self_cluster };
                let child = write_dir(children, cluster, parent, fat, data, next, false)?;
                let off = (cluster as usize - 2) * CLUSTER_BYTES;
                data[off..off + child.len()].copy_from_slice(&child);
            }
            Kind::File(bytes) => {
                let cluster = alloc_file(fat, data, next, bytes)?;
                let size = u32::try_from(bytes.len()).map_err(|_| {
                    anyhow::anyhow!("EFI partition file {} is larger than 4 GiB", node.name)
                })?;
                push_short(&mut raw, &short, 0x20, cluster, size);
            }
        }
    }
    let limit = if is_root { ROOT_BYTES } else { CLUSTER_BYTES };
    if raw.len() > limit {
        bail!("EFI partition directory does not fit in one FAT cluster");
    }
    Ok(raw)
}

fn alloc_one(fat: &mut [u16], next: &mut usize) -> Result<u16> {
    if *next >= fat.len() {
        bail!("EFI partition ran out of FAT16 clusters");
    }
    let cluster = *next;
    fat[cluster] = 0xFFFF;
    *next += 1;
    u16::try_from(cluster).map_err(|_| anyhow::anyhow!("cluster number exceeds FAT16"))
}

fn alloc_file(fat: &mut [u16], data: &mut [u8], next: &mut usize, bytes: &[u8]) -> Result<u16> {
    let n = clusters_for(bytes.len());
    if n == 0 {
        return Ok(0);
    }
    let start = *next;
    if start + n > fat.len() {
        bail!("EFI partition ran out of FAT16 clusters");
    }
    for i in 0..n {
        let cluster = start + i;
        let off = (cluster - 2) * CLUSTER_BYTES;
        let begin = i * CLUSTER_BYTES;
        let end = (begin + CLUSTER_BYTES).min(bytes.len());
        data[off..off + (end - begin)].copy_from_slice(&bytes[begin..end]);
        fat[cluster] = if i + 1 == n {
            0xFFFF
        } else {
            u16::try_from(cluster + 1)
                .map_err(|_| anyhow::anyhow!("cluster number exceeds FAT16"))?
        };
    }
    *next += n;
    u16::try_from(start).map_err(|_| anyhow::anyhow!("cluster number exceeds FAT16"))
}

fn push_dot(out: &mut Vec<u8>, name: &[u8; 11], cluster: u16) {
    push_short(out, name, 0x10, cluster, 0);
}

fn push_short(out: &mut Vec<u8>, name: &[u8; 11], attr: u8, cluster: u16, size: u32) {
    let mut ent = [0u8; 32];
    ent[..11].copy_from_slice(name);
    ent[11] = attr;
    // 2026-10-29. The stamp is fixed so two builds of the same files match.
    let date = (46u16 << 9) | (10 << 5) | 29;
    ent[16..18].copy_from_slice(&date.to_le_bytes());
    ent[18..20].copy_from_slice(&date.to_le_bytes());
    ent[24..26].copy_from_slice(&date.to_le_bytes());
    ent[26..28].copy_from_slice(&cluster.to_le_bytes());
    ent[28..32].copy_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&ent);
}

fn push_lfn(out: &mut Vec<u8>, name: &str, checksum: u8) {
    let units: Vec<u16> = name.encode_utf16().collect();
    let nentries = units.len().div_ceil(13).max(1);
    for seq in (1..=nentries).rev() {
        let mut ent = [0u8; 32];
        let ord = seq as u8 | if seq == nentries { 0x40 } else { 0 };
        ent[0] = ord;
        ent[11] = 0x0F;
        ent[13] = checksum;
        let chars = slot_chars(&units, seq - 1);
        write_utf16(&mut ent[1..11], &chars[0..5]);
        write_utf16(&mut ent[14..26], &chars[5..11]);
        write_utf16(&mut ent[28..32], &chars[11..13]);
        out.extend_from_slice(&ent);
    }
}

fn slot_chars(units: &[u16], index: usize) -> [u16; 13] {
    let mut out = [0xFFFF; 13];
    let start = index * 13;
    let mut terminated = false;
    for (i, slot) in out.iter_mut().enumerate() {
        let src = start + i;
        if src < units.len() {
            *slot = units[src];
        } else if !terminated {
            *slot = 0;
            terminated = true;
        }
    }
    out
}

fn write_utf16(dest: &mut [u8], chars: &[u16]) {
    for (i, ch) in chars.iter().enumerate() {
        dest[i * 2..i * 2 + 2].copy_from_slice(&ch.to_le_bytes());
    }
}

fn lfn_checksum(short: &[u8; 11]) -> u8 {
    let mut sum = 0u8;
    for &byte in short {
        sum = sum.rotate_right(1).wrapping_add(byte);
    }
    sum
}

fn short_name(name: &str, used: &mut BTreeSet<[u8; 11]>) -> Result<[u8; 11]> {
    let (base, ext) = match name.rsplit_once('.') {
        Some((base, ext)) if !base.is_empty() && !ext.is_empty() && !ext.contains('.') => {
            (base, ext)
        }
        _ => (name, ""),
    };
    let base_u = fat_chars(base);
    let ext_u = fat_chars(ext);
    if base_u.len() <= 8 && ext_u.len() <= 3 {
        let plain = pad_8_3(&base_u, &ext_u);
        if used.insert(plain) {
            return Ok(plain);
        }
    }
    let ext3: String = ext_u.chars().take(3).collect();
    for n in 1..=999 {
        let suffix = format!("~{n}");
        let keep = 8 - suffix.len();
        let stem: String = base_u.chars().take(keep).collect();
        let plain = pad_8_3(&format!("{stem}{suffix}"), &ext3);
        if used.insert(plain) {
            return Ok(plain);
        }
    }
    bail!("could not make a unique FAT short name for {name}");
}

fn fat_chars(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn pad_8_3(base: &str, ext: &str) -> [u8; 11] {
    let mut out = [b' '; 11];
    for (i, byte) in base.bytes().take(8).enumerate() {
        out[i] = byte;
    }
    for (i, byte) in ext.bytes().take(3).enumerate() {
        out[8 + i] = byte;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_file(image: &[u8], path: &str) -> Vec<u8> {
        let bps = 512usize;
        let spc = image[13] as usize;
        let reserved = u16::from_le_bytes(image[14..16].try_into().unwrap()) as usize;
        let nfats = image[16] as usize;
        let root_ents = u16::from_le_bytes(image[17..19].try_into().unwrap()) as usize;
        let spf = u16::from_le_bytes(image[22..24].try_into().unwrap()) as usize;
        let root_off = (reserved + nfats * spf) * bps;
        let data_off = root_off + root_ents * 32;
        let fat_off = reserved * bps;
        let cluster_bytes = bps * spc;

        fn next(image: &[u8], fat_off: usize, cluster: u16) -> u16 {
            let off = fat_off + cluster as usize * 2;
            u16::from_le_bytes(image[off..off + 2].try_into().unwrap())
        }

        fn read_chain(
            image: &[u8],
            fat_off: usize,
            data_off: usize,
            cluster_bytes: usize,
            start: u16,
            size: usize,
        ) -> Vec<u8> {
            let mut out = Vec::new();
            let mut cluster = start;
            while (2..0xFFF8).contains(&cluster) && out.len() < size.max(cluster_bytes) {
                let off = data_off + (cluster as usize - 2) * cluster_bytes;
                out.extend_from_slice(&image[off..off + cluster_bytes]);
                cluster = next(image, fat_off, cluster);
            }
            out.truncate(size);
            out
        }

        fn lfn_name(entries: &[(u8, [u8; 32])]) -> String {
            let mut units = Vec::new();
            for (_, ent) in entries {
                let mut chars = [0u16; 13];
                for i in 0..5 {
                    chars[i] = u16::from_le_bytes(ent[1 + i * 2..3 + i * 2].try_into().unwrap());
                }
                for i in 0..6 {
                    chars[5 + i] =
                        u16::from_le_bytes(ent[14 + i * 2..16 + i * 2].try_into().unwrap());
                }
                chars[11] = u16::from_le_bytes(ent[28..30].try_into().unwrap());
                chars[12] = u16::from_le_bytes(ent[30..32].try_into().unwrap());
                for ch in chars {
                    if ch == 0 || ch == 0xFFFF {
                        break;
                    }
                    units.push(ch);
                }
            }
            String::from_utf16(&units).unwrap()
        }

        fn walk(
            image: &[u8],
            fat_off: usize,
            data_off: usize,
            cluster_bytes: usize,
            raw: &[u8],
            parts: &[&str],
        ) -> Vec<u8> {
            let mut i = 0;
            let mut lfns = Vec::new();
            while i + 32 <= raw.len() {
                let ent: [u8; 32] = raw[i..i + 32].try_into().unwrap();
                i += 32;
                if ent[0] == 0 {
                    break;
                }
                if ent[0] == 0xE5 {
                    lfns.clear();
                    continue;
                }
                if ent[11] == 0x0F {
                    lfns.push((ent[0], ent));
                    continue;
                }
                let name = if lfns.is_empty() {
                    let text = std::str::from_utf8(&ent[..11]).unwrap().trim();
                    text.to_string()
                } else {
                    lfns.sort_by_key(|(ord, _)| *ord & 0x1F);
                    let got = lfn_name(&lfns);
                    lfns.clear();
                    got
                };
                if name == "." || name == ".." || !name.eq_ignore_ascii_case(parts[0]) {
                    continue;
                }
                let cluster = u16::from_le_bytes(ent[26..28].try_into().unwrap());
                let size = u32::from_le_bytes(ent[28..32].try_into().unwrap()) as usize;
                let body = if cluster == 0 {
                    Vec::new()
                } else {
                    read_chain(
                        image,
                        fat_off,
                        data_off,
                        cluster_bytes,
                        cluster,
                        if ent[11] & 0x10 != 0 {
                            cluster_bytes
                        } else {
                            size
                        },
                    )
                };
                if parts.len() == 1 {
                    return body;
                }
                return walk(image, fat_off, data_off, cluster_bytes, &body, &parts[1..]);
            }
            panic!("missing {parts:?}");
        }

        let root = &image[root_off..data_off];
        walk(
            image,
            fat_off,
            data_off,
            cluster_bytes,
            root,
            &path.split('/').collect::<Vec<_>>(),
        )
    }

    #[test]
    fn fat_image_round_trips_boot_files() {
        let kernel = vec![0x7fu8, b'E', b'L', b'F', 7, 8, 9];
        let mut loader = vec![0u8; CLUSTER_BYTES + 50];
        loader[0] = 0x11;
        loader[CLUSTER_BYTES] = 0x22;
        loader[CLUSTER_BYTES + 49] = 0x33;
        let conf = b"timeout: 0\nmodule_path: boot():/boot/sel4.elf\n";
        let image = write_image(&[
            ("EFI/BOOT/BOOTX64.EFI", b"efi-loader"),
            ("boot/sel4.elf", &kernel),
            ("boot/loader.img", &loader),
            ("boot/limine.conf", conf),
            ("boot/mb2-stub.elf", b"stub"),
        ])
        .unwrap();
        assert_eq!(&image[510..512], &[0x55, 0xAA]);
        assert_eq!(read_file(&image, "EFI/BOOT/BOOTX64.EFI"), b"efi-loader");
        assert_eq!(read_file(&image, "boot/sel4.elf"), kernel);
        assert_eq!(read_file(&image, "boot/loader.img"), loader);
        assert_eq!(read_file(&image, "boot/limine.conf"), conf);
        assert_eq!(read_file(&image, "boot/mb2-stub.elf"), b"stub");
        assert!(
            grub_counts_fat16(&image),
            "GRUB would probe this volume as FAT12"
        );
    }

    /// GRUB's probe: data clusters plus the two reserved entries, FAT12 at `<= 4087`.
    fn grub_counts_fat16(image: &[u8]) -> bool {
        let bps = u16::from_le_bytes(image[11..13].try_into().unwrap()) as usize;
        let spc = image[13] as usize;
        let reserved = u16::from_le_bytes(image[14..16].try_into().unwrap()) as usize;
        let nfats = image[16] as usize;
        let root_ents = u16::from_le_bytes(image[17..19].try_into().unwrap()) as usize;
        let total16 = u16::from_le_bytes(image[19..21].try_into().unwrap()) as usize;
        let spf = u16::from_le_bytes(image[22..24].try_into().unwrap()) as usize;
        let total32 = u32::from_le_bytes(image[32..36].try_into().unwrap()) as usize;
        let total = if total16 != 0 { total16 } else { total32 };
        let root_sectors = (root_ents * 32).div_ceil(bps);
        let data_clusters = (total - (reserved + nfats * spf + root_sectors)) / spc;
        data_clusters + 2 > 4085 + 2
    }
}
