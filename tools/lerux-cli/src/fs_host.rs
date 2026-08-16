//! Phase 63: seed a host directory into LERUXFS2 at `/host/` (QEMU inject).

use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{bail, Context, Result};
use lerux_fs::{
    alloc_contiguous, encode_dir_entry, encode_free_map_fresh, encode_superblock, make_entry,
    Superblock, FREE_MAP_LBA, MAX_FILE_SECTORS, NAME_LEN, ROOT_DIR_LBA, SUPERBLOCK_LBA,
};
use lerux_interface_types::SECTOR_SIZE;

/// Format `disk` as LERUXFS2 and copy regular files from `dir` into `/host/`.
pub fn seed_host_dir(root: &Path, dir: &Path, disk_path: Option<&Path>) -> Result<usize> {
    let disk = disk_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("support/disk.img"));
    if !disk.is_file() {
        crate::disk_img::disk_img(root)?;
    }
    if !dir.is_dir() {
        bail!("{} is not a directory", dir.display());
    }

    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&disk)
        .with_context(|| format!("open {}", disk.display()))?;

    let sb = Superblock::new();
    let mut sector = [0u8; SECTOR_SIZE];
    encode_superblock(&mut sector, &sb);
    write_lba(&mut f, SUPERBLOCK_LBA, &sector)?;

    let mut map = [0u8; SECTOR_SIZE];
    encode_free_map_fresh(&mut map, &sb);
    let host_lba = alloc_contiguous(&mut map, &sb, 1).context("alloc /host dir")?;
    let mut host_dir = [0u8; SECTOR_SIZE];
    let mut root_dir = [0u8; SECTOR_SIZE];
    encode_dir_entry(&mut root_dir, 0, &make_entry(b"host", host_lba, 0, true));

    let mut slot = 0usize;
    let mut count = 0usize;
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let name_bytes = name.as_bytes();
        if name_bytes.is_empty() || name_bytes.len() > NAME_LEN {
            bail!(
                "host file name {:?} must be 1..={NAME_LEN} bytes",
                path.display()
            );
        }
        if slot >= 16 {
            bail!("/host supports at most 16 files in v1");
        }
        let data = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let need = if data.is_empty() {
            1
        } else {
            u32::try_from(data.len().div_ceil(SECTOR_SIZE)).unwrap_or(u32::MAX)
        };
        if need > MAX_FILE_SECTORS {
            bail!("{} exceeds LERUXFS2 file cap", path.display());
        }
        let first = alloc_contiguous(&mut map, &sb, need)
            .with_context(|| format!("alloc {}", path.display()))?;
        for (i, chunk) in data.chunks(SECTOR_SIZE).enumerate() {
            let mut buf = [0u8; SECTOR_SIZE];
            buf[..chunk.len()].copy_from_slice(chunk);
            write_lba(&mut f, first + i as u32, &buf)?;
        }
        if data.is_empty() {
            write_lba(&mut f, first, &[0u8; SECTOR_SIZE])?;
        }
        encode_dir_entry(
            &mut host_dir,
            slot,
            &make_entry(name_bytes, first, data.len() as u32, false),
        );
        slot += 1;
        count += 1;
    }

    write_lba(&mut f, host_lba, &host_dir)?;
    write_lba(&mut f, ROOT_DIR_LBA, &root_dir)?;
    write_lba(&mut f, FREE_MAP_LBA, &map)?;

    println!(
        "==> seeded {} host file(s) under /host/ on {}",
        count,
        disk.display()
    );
    Ok(count)
}

fn write_lba(f: &mut std::fs::File, lba: u32, sector: &[u8; SECTOR_SIZE]) -> Result<()> {
    let off = (lba as u64) * (SECTOR_SIZE as u64);
    f.seek(SeekFrom::Start(off))
        .with_context(|| format!("seek LBA {lba}"))?;
    f.write_all(sector)
        .with_context(|| format!("write LBA {lba}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lerux_fs::{decode_dir_entry, is_formatted};
    use std::io::Read;

    #[test]
    fn seed_writes_host_hello() {
        let tmp = std::env::temp_dir().join(format!("lerux-fs-host-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let host = tmp.join("host");
        fs::create_dir_all(&host).unwrap();
        fs::write(host.join("hello.txt"), b"hello from host\n").unwrap();
        let disk = tmp.join("disk.img");
        fs::write(&disk, vec![0u8; 4 * 1024 * 1024]).unwrap();

        let n = seed_host_dir(&tmp, &host, Some(&disk)).expect("seed");
        assert_eq!(n, 1);

        let mut f = fs::File::open(&disk).unwrap();
        let mut sb = [0u8; SECTOR_SIZE];
        f.seek(SeekFrom::Start(SUPERBLOCK_LBA as u64 * SECTOR_SIZE as u64))
            .unwrap();
        f.read_exact(&mut sb).unwrap();
        assert!(is_formatted(&sb));

        let mut root = [0u8; SECTOR_SIZE];
        f.seek(SeekFrom::Start(ROOT_DIR_LBA as u64 * SECTOR_SIZE as u64))
            .unwrap();
        f.read_exact(&mut root).unwrap();
        let host_ent = decode_dir_entry(&root, 0);
        assert_eq!(host_ent.name_slice(), b"host");
        assert!(host_ent.is_dir());

        let mut host_dir = [0u8; SECTOR_SIZE];
        f.seek(SeekFrom::Start(
            u64::from(host_ent.first_lba) * SECTOR_SIZE as u64,
        ))
        .unwrap();
        f.read_exact(&mut host_dir).unwrap();
        let file = decode_dir_entry(&host_dir, 0);
        assert_eq!(file.name_slice(), b"hello.txt");
        assert_eq!(file.size, 16);

        let mut data = [0u8; SECTOR_SIZE];
        f.seek(SeekFrom::Start(
            u64::from(file.first_lba) * SECTOR_SIZE as u64,
        ))
        .unwrap();
        f.read_exact(&mut data).unwrap();
        assert_eq!(&data[..16], b"hello from host\n");

        let _ = fs::remove_dir_all(&tmp);
    }
}
