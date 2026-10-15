//! `MICROKIT_PD_NAME_LENGTH` is supplied by a stub. LionsOS 0.4.0 uses Microkit
//! 2.3.0, which defines that length as 64. The stub exists so the driver-framework
//! headers can be parsed without linking seL4.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use lerux_sddf::{
    blk_client_config_t, blk_connection_resource_t, blk_driver_config_t, blk_queue_handle_t,
    blk_req_code_t::BLK_REQ_BARRIER, blk_req_queue_t, blk_req_t, blk_resp_queue_t,
    blk_resp_status_t::BLK_RESP_ERR_NO_DEVICE, blk_resp_t, blk_storage_info_t,
    blk_virt_config_client_t, blk_virt_config_driver_t, blk_virt_config_t,
    device_region_resource_t, fs_client_config_t, fs_cmd_t, fs_cmpl_t, fs_connection_resource_t,
    fs_msg_t, fs_queue_t, fs_server_config_t, fs_stat_t, gpu_events_t, gpu_formats_t,
    gpu_queue_handle_t, gpu_rect_t, gpu_req_code_t, gpu_req_get_display_info_t, gpu_req_queue_t,
    gpu_req_resource_attach_backing_t, gpu_req_resource_create_2d_t,
    gpu_req_resource_create_blob_t, gpu_req_resource_detach_backing_t, gpu_req_resource_flush_t,
    gpu_req_resource_unref_t, gpu_req_set_scanout_blob_t, gpu_req_set_scanout_t, gpu_req_t,
    gpu_req_transfer_to_2d_t, gpu_resp_get_display_info_t, gpu_resp_queue_t, gpu_resp_status_t,
    gpu_resp_t, gpu_scanout_t, mac_addr_t, net_buff_desc_t, net_client_config_t,
    net_connection_resource_t, net_copy_config_t, net_driver_config_t, net_queue_handle_t,
    net_queue_t, net_virt_rx_client_config_t, net_virt_rx_config_t, net_virt_tx_client_config_t,
    net_virt_tx_config_t, net_virt_tx_data_region_t, net_vswitch_config_t,
    net_vswitch_port_config_t, region_resource_t, serial_client_config_t,
    serial_connection_resource_t, serial_driver_config_t, serial_queue_handle_t, serial_queue_t,
    serial_virt_rx_config_t, serial_virt_tx_client_config_t, serial_virt_tx_config_t,
    timer_client_config_t, BLK_MAX_SERIAL_NUMBER, BLK_STORAGE_INFO_REGION_SIZE, BLK_TRANSFER_SIZE,
    FS_CMD_DIR_REWIND, FS_CMD_FILE_OPEN, FS_CMD_INITIALISE, FS_NUM_COMMANDS, FS_OPEN_FLAGS_CREATE,
    FS_QUEUE_CAPACITY, FS_STATUS_NOT_EMPTY, FS_STATUS_NO_FILE, FS_STATUS_NUM_STATUSES,
    FS_STATUS_SUCCESS, GPU_BPP_2D, GPU_DISABLE_SCANOUT_RESOURCE_ID, GPU_EVENTS_REGION_SIZE,
    GPU_MAX_RESOURCES, GPU_MAX_SCANOUTS, LIONS_FS_MAGIC, LIONS_FS_MAGIC_LEN, MAC802_BYTES,
    NET_BUFFER_SIZE, SDDF_BLK_MAGIC, SDDF_BLK_MAGIC_LEN, SDDF_BLK_MAX_CLIENTS, SDDF_NAME_LENGTH,
    SDDF_NET_MAGIC, SDDF_NET_MAGIC_LEN, SDDF_NET_MAX_CLIENTS, SDDF_SERIAL_BEGIN_STR_MAX_LEN,
    SDDF_SERIAL_MAGIC, SDDF_SERIAL_MAGIC_LEN, SDDF_SERIAL_MAX_CLIENTS, SDDF_TIMER_GET_TIME,
    SDDF_TIMER_MAGIC, SDDF_TIMER_MAGIC_LEN, SDDF_TIMER_MAX_CLIENTS, SDDF_TIMER_SET_TIMEOUT,
};

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct CReport {
    size: BTreeMap<String, usize>,
    align: BTreeMap<String, usize>,
    offset: BTreeMap<(String, String), usize>,
    constants: BTreeMap<String, u64>,
    images: BTreeMap<String, Vec<u8>>,
    magic: Vec<u8>,
    blk_magic: Vec<u8>,
    fs_magic: Vec<u8>,
    net_magic: Vec<u8>,
    timer_magic: Vec<u8>,
}

struct SnapshotStruct {
    name: String,
    size: usize,
    align: usize,
    fields: Vec<(String, usize)>,
}

#[test]
fn snapshot_matches_lerux_sddf() {
    let path = snapshot_path();
    println!("{}", path.display());
    println!("header lions/fs/protocol.h");
    let rust = rust_layout();
    for item in &rust {
        println!("size_of {} {}", item.name, item.size);
    }
    if env_flag("LERUX_LIONSOS_LAYOUT_BAD_SIZE") {
        let mut snapshot = load_snapshot(&path);
        apply_bad_serial_size(&mut snapshot);
        finish_compare(&rust, &snapshot);
        return;
    }
    if env_flag("LERUX_LIONSOS_LAYOUT") {
        match_lionsos_headers();
        if env_flag("LERUX_LIONSOS_LAYOUT_WRITE") {
            write_snapshot(&path, &rust);
        } else {
            finish_compare(&rust, &load_snapshot(&path));
            println!("no diff");
        }
        return;
    }
    finish_compare(&rust, &load_snapshot(&path));
}

#[test]
fn bad_size_fixture() {
    let mut snapshot = load_snapshot(&snapshot_path());
    apply_bad_serial_size(&mut snapshot);
    let errors = compare_snapshot(&rust_layout(), &snapshot);
    assert!(
        errors
            .iter()
            .any(|error| error.starts_with("serial_queue_t:")),
        "bad size must name serial_queue_t, got {errors:?}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn snapshot_path() -> PathBuf {
    repo_root().join("support/lionsos-layout.toml")
}

fn env_flag(name: &str) -> bool {
    env::var(name).ok().as_deref() == Some("1")
}

fn rust_layout() -> Vec<SnapshotStruct> {
    let fields = rust_fields();
    rust_types()
        .into_iter()
        .map(|(name, size, align)| SnapshotStruct {
            name: name.to_string(),
            size,
            align,
            fields: fields
                .iter()
                .filter(|(struct_name, _, _)| *struct_name == name)
                .map(|(_, field, offset)| ((*field).to_string(), *offset))
                .collect(),
        })
        .collect()
}

fn apply_bad_serial_size(snapshot: &mut [SnapshotStruct]) {
    let serial = snapshot
        .iter_mut()
        .find(|item| item.name == "serial_queue_t")
        .expect("serial_queue_t");
    serial.size += 1;
}

fn finish_compare(rust: &[SnapshotStruct], snapshot: &[SnapshotStruct]) {
    let errors = compare_snapshot(rust, snapshot);
    assert!(
        errors.is_empty(),
        "LionsOS layout snapshot disagrees with lerux-sddf:\n{}",
        errors.join("\n")
    );
}

fn compare_snapshot(rust: &[SnapshotStruct], snapshot: &[SnapshotStruct]) -> Vec<String> {
    let mut errors = Vec::new();
    for item in rust {
        let Some(snap) = snapshot
            .iter()
            .find(|candidate| candidate.name == item.name)
        else {
            errors.push(format!("{}: snapshot missing", item.name));
            continue;
        };
        if snap.size != item.size {
            errors.push(format!(
                "{}: snapshot size {}, Rust size {}",
                item.name, snap.size, item.size
            ));
        }
        if snap.align != item.align {
            errors.push(format!(
                "{}: snapshot align {}, Rust align {}",
                item.name, snap.align, item.align
            ));
        }
        for (field, offset) in &item.fields {
            match snap.fields.iter().find(|(name, _)| name == field) {
                Some((_, snap_offset)) if snap_offset != offset => errors.push(format!(
                    "{}.{}: snapshot offset {snap_offset}, Rust offset {offset}",
                    item.name, field
                )),
                Some(_) => {}
                None => errors.push(format!("{}.{}: snapshot missing", item.name, field)),
            }
        }
        for (field, _) in &snap.fields {
            if !item.fields.iter().any(|(name, _)| name == field) {
                errors.push(format!(
                    "{}.{}: snapshot has an extra field",
                    item.name, field
                ));
            }
        }
    }
    for snap in snapshot {
        if !rust.iter().any(|item| item.name == snap.name) {
            errors.push(format!("{}: snapshot has an extra struct", snap.name));
        }
    }
    errors
}

fn match_lionsos_headers() {
    let root = repo_root();
    require_headers(&root);
    let report = compile_and_run(&root);
    let mut errors = Vec::new();
    check_types(&report, &mut errors);
    check_fields(&report, &mut errors);
    check_constants(&report, &mut errors);
    check_images(&report, &mut errors);
    check_magic(&report, &mut errors);
    assert!(
        errors.is_empty(),
        "LionsOS header layout disagrees with lerux-sddf:\n{}",
        errors.join("\n")
    );
}

fn write_snapshot(path: &Path, rust: &[SnapshotStruct]) {
    let rendered = render_snapshot(rust);
    let previous = fs::read_to_string(path).unwrap_or_default();
    if previous == rendered {
        println!("no diff");
        return;
    }
    fs::write(path, &rendered).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
    println!("wrote {}", path.display());
}

fn require_headers(root: &Path) {
    let protocol = root.join("deps/workspace/lionsos/include/lions/fs/protocol.h");
    let serial_queue = root.join("deps/workspace/lionsos/dep/sddf/include/sddf/serial/queue.h");
    if protocol.is_file() && serial_queue.is_file() {
        return;
    }
    panic!(
        "deps/workspace/lionsos is missing ({}; {})",
        protocol.display(),
        serial_queue.display()
    );
}

fn load_snapshot(path: &Path) -> Vec<SnapshotStruct> {
    let text =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    parse_snapshot(&text)
}

fn parse_snapshot(text: &str) -> Vec<SnapshotStruct> {
    struct RawStruct {
        name: String,
        size: Option<usize>,
        align: Option<usize>,
        fields: Vec<(String, usize)>,
    }

    let mut structs: Vec<RawStruct> = Vec::new();
    let mut current: Option<usize> = None;
    let mut in_fields = false;
    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line_number = line_no + 1;
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            if let Some(name) = header.strip_suffix(".field") {
                let index = structs
                    .iter()
                    .position(|item| item.name == name)
                    .unwrap_or_else(|| {
                        panic!("snapshot field table {name} has no struct at line {line_number}")
                    });
                current = Some(index);
                in_fields = true;
            } else if structs.iter().any(|item| item.name == header) {
                panic!("duplicate snapshot struct {header} at line {line_number}");
            } else {
                structs.push(RawStruct {
                    name: header.to_string(),
                    size: None,
                    align: None,
                    fields: Vec::new(),
                });
                current = Some(structs.len() - 1);
                in_fields = false;
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            panic!("bad snapshot line {line_number}");
        };
        let key = key.trim();
        let value: usize = value
            .trim()
            .parse()
            .unwrap_or_else(|err| panic!("bad snapshot number on line {line_number}: {err}"));
        let Some(index) = current else {
            panic!("snapshot assignment before a struct at line {line_number}");
        };
        if in_fields {
            if structs[index].fields.iter().any(|(name, _)| name == key) {
                panic!("duplicate snapshot field {key} at line {line_number}");
            }
            structs[index].fields.push((key.to_string(), value));
        } else if key == "size" {
            structs[index].size = Some(value);
        } else if key == "align" {
            structs[index].align = Some(value);
        } else {
            panic!("unknown snapshot key {key} at line {line_number}");
        }
    }
    structs
        .into_iter()
        .map(|item| SnapshotStruct {
            size: item
                .size
                .unwrap_or_else(|| panic!("{}: snapshot size is required", item.name)),
            align: item
                .align
                .unwrap_or_else(|| panic!("{}: snapshot align is required", item.name)),
            name: item.name,
            fields: item.fields,
        })
        .collect()
}

fn render_snapshot(items: &[SnapshotStruct]) -> String {
    let mut out = String::new();
    for item in items {
        out.push_str(&format!(
            "[{}]\nsize = {}\nalign = {}\n\n[{}.field]\n",
            item.name, item.size, item.align, item.name
        ));
        for (field, offset) in &item.fields {
            out.push_str(&format!("{field} = {offset}\n"));
        }
        out.push('\n');
    }
    out
}

fn compile_and_run(root: &Path) -> CReport {
    let dir = std::env::temp_dir().join(format!("lerux-sddf-layout-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create layout probe directory");
    let _guard = TempDir(dir.clone());
    let stub = dir.join("stub");
    fs::create_dir_all(stub.join("sel4")).expect("create sel4 stub directory");
    fs::write(stub.join("microkit.h"), MICROKIT_STUB).expect("write microkit stub");
    fs::write(stub.join("sel4/sel4.h"), SEL4_STUB).expect("write sel4 stub");
    let source = dir.join("layout.c");
    fs::write(&source, LAYOUT_C).expect("write layout probe");
    let binary = dir.join("layout");

    let sddf = root.join("deps/workspace/lionsos/dep/sddf/include");
    let lions = root.join("deps/workspace/lionsos/include");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let compiled = Command::new(&cc)
        .arg("-std=c11")
        .arg("-DCONFIG_ARCH_X86_64")
        .arg("-I")
        .arg(&stub)
        .arg("-I")
        .arg(sddf.join("microkit"))
        .arg("-I")
        .arg(&sddf)
        .arg("-I")
        .arg(&lions)
        .arg("-o")
        .arg(&binary)
        .arg(&source)
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn {cc}: {err}"));
    if !compiled.status.success() {
        panic!(
            "{cc} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&compiled.stdout),
            String::from_utf8_lossy(&compiled.stderr)
        );
    }

    let ran = Command::new(&binary)
        .output()
        .unwrap_or_else(|err| panic!("failed to run layout probe: {err}"));
    if !ran.status.success() {
        panic!(
            "layout probe failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        );
    }
    parse_report(&String::from_utf8_lossy(&ran.stdout))
}

fn parse_report(text: &str) -> CReport {
    let mut report = CReport {
        size: BTreeMap::new(),
        align: BTreeMap::new(),
        offset: BTreeMap::new(),
        constants: BTreeMap::new(),
        images: BTreeMap::new(),
        magic: Vec::new(),
        blk_magic: Vec::new(),
        fs_magic: Vec::new(),
        net_magic: Vec::new(),
        timer_magic: Vec::new(),
    };
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let kind = parts.next().unwrap_or_else(|| panic!("empty probe line"));
        match kind {
            "size" => {
                let name = part(&mut parts, line);
                report.size.insert(name, number(&mut parts, line));
            }
            "align" => {
                let name = part(&mut parts, line);
                report.align.insert(name, number(&mut parts, line));
            }
            "off" => {
                let name = part(&mut parts, line);
                let field = part(&mut parts, line);
                report
                    .offset
                    .insert((name, field), number(&mut parts, line));
            }
            "const" => {
                let name = part(&mut parts, line);
                report.constants.insert(name, number_u64(&mut parts, line));
            }
            "image" => {
                let name = part(&mut parts, line);
                report.images.insert(name, hex_bytes(parts, line));
            }
            "magic" => report.magic = hex_bytes(parts, line),
            "blkmagic" => report.blk_magic = hex_bytes(parts, line),
            "fsmagic" => report.fs_magic = hex_bytes(parts, line),
            "netmagic" => report.net_magic = hex_bytes(parts, line),
            "timermagic" => report.timer_magic = hex_bytes(parts, line),
            other => panic!("unknown probe line kind {other} in {line}"),
        }
    }
    report
}

fn part(parts: &mut std::str::SplitWhitespace<'_>, line: &str) -> String {
    parts
        .next()
        .unwrap_or_else(|| panic!("short probe line: {line}"))
        .to_string()
}

fn number(parts: &mut std::str::SplitWhitespace<'_>, line: &str) -> usize {
    part(parts, line)
        .parse()
        .unwrap_or_else(|err| panic!("bad number in {line}: {err}"))
}

fn number_u64(parts: &mut std::str::SplitWhitespace<'_>, line: &str) -> u64 {
    part(parts, line)
        .parse()
        .unwrap_or_else(|err| panic!("bad number in {line}: {err}"))
}

fn hex_bytes(parts: std::str::SplitWhitespace<'_>, line: &str) -> Vec<u8> {
    parts
        .map(|byte| {
            u8::from_str_radix(byte, 16).unwrap_or_else(|err| panic!("bad hex in {line}: {err}"))
        })
        .collect()
}

fn check_types(report: &CReport, errors: &mut Vec<String>) {
    for (name, size, align) in rust_types() {
        match (report.size.get(name), report.align.get(name)) {
            (Some(&c_size), Some(&c_align)) => {
                if c_size != size || c_align != align {
                    errors.push(format!(
                        "{name}: C size {c_size} align {c_align}, Rust size {size} align {align}"
                    ));
                }
            }
            _ => errors.push(format!("{name}: C probe did not report size and alignment")),
        }
    }
}

fn check_fields(report: &CReport, errors: &mut Vec<String>) {
    for (name, field, offset) in rust_fields() {
        match report.offset.get(&(name.to_string(), field.to_string())) {
            Some(&c_offset) if c_offset != offset => {
                errors.push(format!(
                    "{name}.{field}: C offset {c_offset}, Rust offset {offset}"
                ));
            }
            Some(_) => {}
            None => errors.push(format!("{name}.{field}: C probe did not report the offset")),
        }
    }
}

fn check_constants(report: &CReport, errors: &mut Vec<String>) {
    for (name, value) in rust_constants() {
        expect_const(report, name, value, errors);
    }
    expect_const(report, "BLK_REQ_BARRIER", BLK_REQ_BARRIER as u64, errors);
    expect_const(
        report,
        "BLK_RESP_ERR_NO_DEVICE",
        BLK_RESP_ERR_NO_DEVICE as u64,
        errors,
    );
}

fn expect_const(report: &CReport, name: &str, value: u64, errors: &mut Vec<String>) {
    match report.constants.get(name) {
        Some(&c_value) if c_value != value => {
            errors.push(format!("{name}: C {c_value}, Rust {value}"));
        }
        Some(_) => {}
        None => errors.push(format!("{name}: C probe did not report the constant")),
    }
}

fn check_images(report: &CReport, errors: &mut Vec<String>) {
    expect_image(report, "fit", 0x2a, errors);
    expect_image(report, "wide", 0xff, errors);
}

fn expect_image(report: &CReport, name: &str, oid: u8, errors: &mut Vec<String>) {
    let rust_bytes = desc_bytes(oid);
    match report.images.get(name) {
        Some(c_bytes) if c_bytes != &rust_bytes => {
            errors.push(format!(
                "net_buff_desc_t {name}: C {c_bytes:02x?}, Rust {rust_bytes:02x?}"
            ));
        }
        Some(_) => {}
        None => errors.push(format!(
            "net_buff_desc_t {name}: C probe did not report the image"
        )),
    }
}

fn desc_bytes(oid: u8) -> Vec<u8> {
    let desc = net_buff_desc_t::new(0x0102_0304_0506_0708, 0xaabb, oid);
    let mut bytes = vec![0; core::mem::size_of::<net_buff_desc_t>()];
    // SAFETY: `desc` is a fully initialised descriptor and `bytes` has its size.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(&desc).cast::<u8>(),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    bytes
}

fn check_magic(report: &CReport, errors: &mut Vec<String>) {
    if report.magic != SDDF_SERIAL_MAGIC {
        errors.push(format!(
            "SDDF_SERIAL_MAGIC: C {:02x?}, Rust {:02x?}",
            report.magic, SDDF_SERIAL_MAGIC
        ));
    }
    if report.blk_magic != SDDF_BLK_MAGIC {
        errors.push(format!(
            "SDDF_BLK_MAGIC: C {:02x?}, Rust {:02x?}",
            report.blk_magic, SDDF_BLK_MAGIC
        ));
    }
    if report.fs_magic != LIONS_FS_MAGIC {
        errors.push(format!(
            "LIONS_FS_MAGIC: C {:02x?}, Rust {:02x?}",
            report.fs_magic, LIONS_FS_MAGIC
        ));
    }
    if report.net_magic != SDDF_NET_MAGIC {
        errors.push(format!(
            "SDDF_NET_MAGIC: C {:02x?}, Rust {:02x?}",
            report.net_magic, SDDF_NET_MAGIC
        ));
    }
    if report.timer_magic != SDDF_TIMER_MAGIC {
        errors.push(format!(
            "SDDF_TIMER_MAGIC: C {:02x?}, Rust {:02x?}",
            report.timer_magic, SDDF_TIMER_MAGIC
        ));
    }
}

fn rust_types() -> Vec<(&'static str, usize, usize)> {
    macro_rules! ty {
        ($($name:ty),* $(,)?) => {
            vec![
                $((
                    stringify!($name),
                    core::mem::size_of::<$name>(),
                    core::mem::align_of::<$name>(),
                ),)*
            ]
        };
    }
    ty![
        serial_queue_t,
        serial_queue_handle_t,
        region_resource_t,
        serial_connection_resource_t,
        serial_driver_config_t,
        serial_virt_rx_config_t,
        serial_virt_tx_client_config_t,
        serial_virt_tx_config_t,
        serial_client_config_t,
        net_buff_desc_t,
        net_queue_t,
        net_queue_handle_t,
        mac_addr_t,
        net_connection_resource_t,
        net_driver_config_t,
        net_virt_tx_data_region_t,
        net_virt_tx_client_config_t,
        net_virt_tx_config_t,
        net_virt_rx_client_config_t,
        net_virt_rx_config_t,
        net_copy_config_t,
        net_client_config_t,
        net_vswitch_port_config_t,
        net_vswitch_config_t,
        blk_req_t,
        blk_resp_t,
        blk_req_queue_t,
        blk_resp_queue_t,
        blk_queue_handle_t,
        device_region_resource_t,
        blk_connection_resource_t,
        blk_driver_config_t,
        blk_virt_config_driver_t,
        blk_virt_config_client_t,
        blk_virt_config_t,
        blk_client_config_t,
        blk_storage_info_t,
        fs_connection_resource_t,
        fs_server_config_t,
        fs_client_config_t,
        fs_stat_t,
        fs_cmd_t,
        fs_cmpl_t,
        fs_msg_t,
        fs_queue_t,
        timer_client_config_t,
        gpu_rect_t,
        gpu_scanout_t,
        gpu_events_t,
        gpu_req_get_display_info_t,
        gpu_resp_get_display_info_t,
        gpu_req_resource_create_blob_t,
        gpu_req_set_scanout_blob_t,
        gpu_req_resource_create_2d_t,
        gpu_req_resource_attach_backing_t,
        gpu_req_resource_detach_backing_t,
        gpu_req_set_scanout_t,
        gpu_req_transfer_to_2d_t,
        gpu_req_resource_flush_t,
        gpu_req_resource_unref_t,
        gpu_req_t,
        gpu_resp_t,
        gpu_req_queue_t,
        gpu_resp_queue_t,
        gpu_queue_handle_t,
    ]
}

fn rust_fields() -> Vec<(&'static str, &'static str, usize)> {
    macro_rules! field {
        ($($name:ty, $field:ident);* $(;)?) => {
            vec![
                $((
                    stringify!($name),
                    stringify!($field),
                    core::mem::offset_of!($name, $field),
                ),)*
            ]
        };
    }
    let mut fields = field![
        serial_queue_t, tail;
        serial_queue_t, head;
        serial_queue_t, producer_signalled;
        serial_queue_handle_t, queue;
        serial_queue_handle_t, capacity;
        serial_queue_handle_t, data_region;
        region_resource_t, vaddr;
        region_resource_t, size;
        serial_connection_resource_t, queue;
        serial_connection_resource_t, data;
        serial_connection_resource_t, id;
        serial_driver_config_t, magic;
        serial_driver_config_t, rx;
        serial_driver_config_t, tx;
        serial_driver_config_t, default_baud;
        serial_driver_config_t, rx_enabled;
        serial_virt_rx_config_t, magic;
        serial_virt_rx_config_t, driver;
        serial_virt_rx_config_t, clients;
        serial_virt_rx_config_t, num_clients;
        serial_virt_rx_config_t, switch_char;
        serial_virt_rx_config_t, terminate_num_char;
        serial_virt_tx_client_config_t, conn;
        serial_virt_tx_client_config_t, name;
        serial_virt_tx_config_t, magic;
        serial_virt_tx_config_t, driver;
        serial_virt_tx_config_t, clients;
        serial_virt_tx_config_t, num_clients;
        serial_virt_tx_config_t, begin_str;
        serial_virt_tx_config_t, enable_colour;
        serial_virt_tx_config_t, enable_rx;
        serial_client_config_t, magic;
        serial_client_config_t, rx;
        serial_client_config_t, tx;
        net_buff_desc_t, io_or_offset;
        net_buff_desc_t, len;
        net_queue_t, tail;
        net_queue_t, head;
        net_queue_t, consumer_signalled;
        net_queue_handle_t, free;
        net_queue_handle_t, active;
        net_queue_handle_t, capacity;
        mac_addr_t, addr;
        net_connection_resource_t, free_queue;
        net_connection_resource_t, active_queue;
        net_connection_resource_t, num_buffers;
        net_connection_resource_t, id;
        net_driver_config_t, magic;
        net_driver_config_t, virt_rx;
        net_driver_config_t, virt_tx;
        net_virt_tx_data_region_t, data;
        net_virt_tx_data_region_t, num_buffers;
        net_virt_tx_client_config_t, conn;
        net_virt_tx_client_config_t, regions;
        net_virt_tx_client_config_t, num_regions;
        net_virt_tx_config_t, magic;
        net_virt_tx_config_t, driver;
        net_virt_tx_config_t, clients;
        net_virt_tx_config_t, num_clients;
        net_virt_rx_client_config_t, conn;
        net_virt_rx_client_config_t, mac_addrs;
        net_virt_rx_client_config_t, num_macs;
        net_virt_rx_config_t, magic;
        net_virt_rx_config_t, driver;
        net_virt_rx_config_t, data;
        net_virt_rx_config_t, buffer_metadata;
        net_virt_rx_config_t, clients;
        net_virt_rx_config_t, num_clients;
        net_copy_config_t, magic;
        net_copy_config_t, rx;
        net_copy_config_t, rx_data;
        net_copy_config_t, client;
        net_copy_config_t, client_data;
        net_client_config_t, magic;
        net_client_config_t, rx;
        net_client_config_t, rx_data;
        net_client_config_t, tx;
        net_client_config_t, tx_data;
        net_client_config_t, mac_addr;
        net_vswitch_port_config_t, rx;
        net_vswitch_port_config_t, tx;
        net_vswitch_port_config_t, tx_data;
        net_vswitch_port_config_t, mac_addr;
        net_vswitch_port_config_t, acl;
        net_vswitch_config_t, magic;
        net_vswitch_config_t, ports;
        net_vswitch_config_t, num_ports;
        net_vswitch_config_t, buffer_metadata;
        blk_req_t, code;
        blk_req_t, io_or_offset;
        blk_req_t, block_number;
        blk_req_t, count;
        blk_req_t, id;
        blk_resp_t, status;
        blk_resp_t, success_count;
        blk_resp_t, id;
        blk_req_queue_t, head;
        blk_req_queue_t, tail;
        blk_req_queue_t, plugged;
        blk_resp_queue_t, head;
        blk_resp_queue_t, tail;
        blk_queue_handle_t, req_queue;
        blk_queue_handle_t, resp_queue;
        blk_queue_handle_t, capacity;
        device_region_resource_t, region;
        device_region_resource_t, io_addr;
        blk_connection_resource_t, storage_info;
        blk_connection_resource_t, req_queue;
        blk_connection_resource_t, resp_queue;
        blk_connection_resource_t, num_buffers;
        blk_connection_resource_t, id;
        blk_driver_config_t, magic;
        blk_driver_config_t, virt;
        blk_virt_config_driver_t, conn;
        blk_virt_config_driver_t, data;
        blk_virt_config_client_t, conn;
        blk_virt_config_client_t, data;
        blk_virt_config_client_t, partition;
        blk_virt_config_t, magic;
        blk_virt_config_t, num_clients;
        blk_virt_config_t, driver;
        blk_virt_config_t, clients;
        blk_client_config_t, magic;
        blk_client_config_t, virt;
        blk_client_config_t, data;
        blk_storage_info_t, serial_number;
        blk_storage_info_t, read_only;
        blk_storage_info_t, ready;
        blk_storage_info_t, sector_size;
        blk_storage_info_t, block_size;
        blk_storage_info_t, queue_depth;
        blk_storage_info_t, cylinders;
        blk_storage_info_t, heads;
        blk_storage_info_t, blocks;
        blk_storage_info_t, capacity;
        fs_connection_resource_t, command_queue;
        fs_connection_resource_t, completion_queue;
        fs_connection_resource_t, share;
        fs_connection_resource_t, queue_len;
        fs_connection_resource_t, id;
        fs_server_config_t, magic;
        fs_server_config_t, client;
        fs_client_config_t, magic;
        fs_client_config_t, server;
        fs_stat_t, dev;
        fs_stat_t, used;
        fs_cmd_t, id;
        fs_cmd_t, params;
        fs_cmpl_t, id;
        fs_cmpl_t, status;
        fs_cmpl_t, data;
        fs_msg_t, cmd;
        fs_msg_t, cmpl;
        fs_queue_t, head;
        fs_queue_t, tail;
        fs_queue_t, padding;
        fs_queue_t, buffer;
        gpu_rect_t, x;
        gpu_rect_t, y;
        gpu_rect_t, width;
        gpu_rect_t, height;
        gpu_scanout_t, rect;
        gpu_scanout_t, enabled;
        gpu_events_t, display_info;
        gpu_req_get_display_info_t, mem_offset;
        gpu_resp_get_display_info_t, scanouts;
        gpu_resp_get_display_info_t, num_scanouts;
        gpu_req_resource_create_blob_t, resource_id;
        gpu_req_resource_create_blob_t, mem_offset;
        gpu_req_resource_create_blob_t, mem_size;
        gpu_req_set_scanout_blob_t, resource_id;
        gpu_req_set_scanout_blob_t, scanout_id;
        gpu_req_set_scanout_blob_t, width;
        gpu_req_set_scanout_blob_t, height;
        gpu_req_set_scanout_blob_t, format;
        gpu_req_set_scanout_blob_t, stride;
        gpu_req_set_scanout_blob_t, offset;
        gpu_req_set_scanout_blob_t, rect;
        gpu_req_resource_create_2d_t, resource_id;
        gpu_req_resource_create_2d_t, width;
        gpu_req_resource_create_2d_t, height;
        gpu_req_resource_create_2d_t, format;
        gpu_req_resource_attach_backing_t, resource_id;
        gpu_req_resource_attach_backing_t, mem_offset;
        gpu_req_resource_attach_backing_t, mem_size;
        gpu_req_resource_detach_backing_t, resource_id;
        gpu_req_set_scanout_t, resource_id;
        gpu_req_set_scanout_t, scanout_id;
        gpu_req_set_scanout_t, rect;
        gpu_req_transfer_to_2d_t, resource_id;
        gpu_req_transfer_to_2d_t, rect;
        gpu_req_transfer_to_2d_t, mem_offset;
        gpu_req_resource_flush_t, resource_id;
        gpu_req_resource_flush_t, rect;
        gpu_req_resource_unref_t, resource_id;
        gpu_req_t, code;
        gpu_req_t, id;
        gpu_resp_t, status;
        gpu_resp_t, id;
        gpu_req_queue_t, head;
        gpu_req_queue_t, tail;
        gpu_resp_queue_t, head;
        gpu_resp_queue_t, tail;
        gpu_queue_handle_t, req_queue;
        gpu_queue_handle_t, resp_queue;
        gpu_queue_handle_t, capacity;
    ];
    fields.push(("fs_cmd_t", "type", core::mem::offset_of!(fs_cmd_t, r#type)));
    fields.push((
        "timer_client_config_t",
        "magic",
        core::mem::offset_of!(timer_client_config_t, magic),
    ));
    fields.push((
        "timer_client_config_t",
        "driver_id",
        core::mem::offset_of!(timer_client_config_t, driver_id),
    ));
    for (field, offset) in [
        (
            "get_display_info",
            core::mem::offset_of!(gpu_req_t, payload.get_display_info),
        ),
        (
            "resource_create_blob",
            core::mem::offset_of!(gpu_req_t, payload.resource_create_blob),
        ),
        (
            "set_scanout_blob",
            core::mem::offset_of!(gpu_req_t, payload.set_scanout_blob),
        ),
        (
            "resource_create_2d",
            core::mem::offset_of!(gpu_req_t, payload.resource_create_2d),
        ),
        (
            "resource_attach_backing",
            core::mem::offset_of!(gpu_req_t, payload.resource_attach_backing),
        ),
        (
            "resource_detach_backing",
            core::mem::offset_of!(gpu_req_t, payload.resource_detach_backing),
        ),
        (
            "set_scanout",
            core::mem::offset_of!(gpu_req_t, payload.set_scanout),
        ),
        (
            "transfer_to_2d",
            core::mem::offset_of!(gpu_req_t, payload.transfer_to_2d),
        ),
        (
            "resource_flush",
            core::mem::offset_of!(gpu_req_t, payload.resource_flush),
        ),
        (
            "resource_unref",
            core::mem::offset_of!(gpu_req_t, payload.resource_unref),
        ),
    ] {
        fields.push(("gpu_req_t", field, offset));
    }
    fields
}

fn rust_constants() -> Vec<(&'static str, u64)> {
    vec![
        ("FS_QUEUE_CAPACITY", FS_QUEUE_CAPACITY as u64),
        ("FS_STATUS_SUCCESS", FS_STATUS_SUCCESS),
        ("FS_STATUS_NO_FILE", FS_STATUS_NO_FILE),
        ("FS_STATUS_NOT_EMPTY", FS_STATUS_NOT_EMPTY),
        ("FS_STATUS_NUM_STATUSES", FS_STATUS_NUM_STATUSES),
        ("FS_CMD_INITIALISE", FS_CMD_INITIALISE),
        ("FS_CMD_FILE_OPEN", FS_CMD_FILE_OPEN),
        ("FS_CMD_DIR_REWIND", FS_CMD_DIR_REWIND),
        ("FS_NUM_COMMANDS", FS_NUM_COMMANDS),
        ("FS_OPEN_FLAGS_CREATE", FS_OPEN_FLAGS_CREATE),
        ("BLK_TRANSFER_SIZE", u64::from(BLK_TRANSFER_SIZE)),
        ("NET_BUFFER_SIZE", u64::from(NET_BUFFER_SIZE)),
        ("SDDF_NET_MAX_CLIENTS", SDDF_NET_MAX_CLIENTS as u64),
        ("SDDF_NET_MAGIC_LEN", SDDF_NET_MAGIC_LEN as u64),
        ("MAC802_BYTES", MAC802_BYTES as u64),
        ("SDDF_SERIAL_MAX_CLIENTS", SDDF_SERIAL_MAX_CLIENTS as u64),
        ("SDDF_NAME_LENGTH", SDDF_NAME_LENGTH as u64),
        (
            "SDDF_SERIAL_BEGIN_STR_MAX_LEN",
            SDDF_SERIAL_BEGIN_STR_MAX_LEN as u64,
        ),
        ("SDDF_SERIAL_MAGIC_LEN", SDDF_SERIAL_MAGIC_LEN as u64),
        ("SDDF_BLK_MAX_CLIENTS", SDDF_BLK_MAX_CLIENTS as u64),
        ("SDDF_BLK_MAGIC_LEN", SDDF_BLK_MAGIC_LEN as u64),
        (
            "BLK_STORAGE_INFO_REGION_SIZE",
            BLK_STORAGE_INFO_REGION_SIZE as u64,
        ),
        ("BLK_MAX_SERIAL_NUMBER", BLK_MAX_SERIAL_NUMBER as u64),
        ("LIONS_FS_MAGIC_LEN", LIONS_FS_MAGIC_LEN as u64),
        ("SDDF_TIMER_GET_TIME", SDDF_TIMER_GET_TIME),
        ("SDDF_TIMER_SET_TIMEOUT", SDDF_TIMER_SET_TIMEOUT),
        ("SDDF_TIMER_MAGIC_LEN", SDDF_TIMER_MAGIC_LEN as u64),
        ("SDDF_TIMER_MAX_CLIENTS", SDDF_TIMER_MAX_CLIENTS as u64),
        ("GPU_MAX_RESOURCES", GPU_MAX_RESOURCES as u64),
        (
            "GPU_DISABLE_SCANOUT_RESOURCE_ID",
            u64::from(GPU_DISABLE_SCANOUT_RESOURCE_ID),
        ),
        ("GPU_BPP_2D", u64::from(GPU_BPP_2D)),
        ("GPU_MAX_SCANOUTS", GPU_MAX_SCANOUTS as u64),
        ("GPU_EVENTS_REGION_SIZE", GPU_EVENTS_REGION_SIZE as u64),
        (
            "GPU_REQ_GET_DISPLAY_INFO",
            gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO as u64,
        ),
        (
            "GPU_REQ_RESOURCE_UNREF",
            gpu_req_code_t::GPU_REQ_RESOURCE_UNREF as u64,
        ),
        ("GPU_RESP_OK", gpu_resp_status_t::GPU_RESP_OK as u64),
        (
            "GPU_RESP_ERR_INVALID_PARAMETER",
            gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER as u64,
        ),
        (
            "GPU_FORMAT_B8G8R8A8_UNORM",
            gpu_formats_t::GPU_FORMAT_B8G8R8A8_UNORM as u64,
        ),
        (
            "GPU_FORMAT_R8G8B8X8_UNORM",
            gpu_formats_t::GPU_FORMAT_R8G8B8X8_UNORM as u64,
        ),
    ]
}

const MICROKIT_STUB: &str = r#"
#pragma once
#include <stdint.h>
/* Microkit 2.3.0, the kit LionsOS 0.4.0 builds with. */
#define MICROKIT_PD_NAME_LENGTH 64
#define BASE_OUTPUT_NOTIFICATION_CAP 10
typedef unsigned int microkit_channel;
typedef struct { uint64_t raw; } microkit_msginfo;
extern char microkit_name[MICROKIT_PD_NAME_LENGTH];
extern int microkit_have_signal;
extern unsigned int microkit_signal_cap;
static inline void microkit_irq_ack(microkit_channel ch) { (void)ch; }
static inline void microkit_deferred_irq_ack(microkit_channel ch) { (void)ch; }
static inline void microkit_notify(microkit_channel ch) { (void)ch; }
static inline void microkit_deferred_notify(microkit_channel ch) { (void)ch; }
static inline microkit_msginfo microkit_ppcall(microkit_channel ch, microkit_msginfo msginfo) {
    (void)ch;
    return msginfo;
}
"#;

const SEL4_STUB: &str = r#"
#pragma once
#include <stdint.h>
static inline uint64_t seL4_GetMR(unsigned int n) { (void)n; return 0; }
static inline void seL4_SetMR(unsigned int n, uint64_t val) { (void)n; (void)val; }
"#;

const LAYOUT_C: &str = r#"
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <lions/fs/config.h>
#include <lions/fs/protocol.h>
#include <sddf/blk/config.h>
#include <sddf/gpu/events.h>
#include <sddf/gpu/gpu.h>
#include <sddf/gpu/queue.h>
#include <sddf/blk/queue.h>
#include <sddf/blk/storage_info.h>
#include <sddf/network/config.h>
#include <sddf/network/queue.h>
#include <sddf/resources/device.h>
#include <sddf/serial/config.h>
#include <sddf/serial/queue.h>
#include <sddf/timer/config.h>
#include <sddf/timer/protocol.h>

#define REPORT_TYPE(t) \
    do { \
        printf("size %s %zu\n", #t, sizeof(t)); \
        printf("align %s %zu\n", #t, _Alignof(t)); \
    } while (0)

#define REPORT_FIELD(t, f) printf("off %s %s %zu\n", #t, #f, offsetof(t, f))

static void print_desc(const char *name, uint8_t oid)
{
    net_buff_desc_t desc;
    unsigned char *bytes;
    size_t i;
    memset(&desc, 0, sizeof desc);
    desc.io_or_offset = 0x0102030405060708ull;
    desc.len = 0xaabb;
    desc.oid = oid;
    bytes = (unsigned char *)&desc;
    printf("image %s", name);
    for (i = 0; i < sizeof desc; i++) {
        printf(" %02x", bytes[i]);
    }
    printf("\n");
}

int main(void)
{
    int i;

    REPORT_TYPE(serial_queue_t);
    REPORT_FIELD(serial_queue_t, tail);
    REPORT_FIELD(serial_queue_t, head);
    REPORT_FIELD(serial_queue_t, producer_signalled);
    REPORT_TYPE(serial_queue_handle_t);
    REPORT_FIELD(serial_queue_handle_t, queue);
    REPORT_FIELD(serial_queue_handle_t, capacity);
    REPORT_FIELD(serial_queue_handle_t, data_region);
    REPORT_TYPE(region_resource_t);
    REPORT_FIELD(region_resource_t, vaddr);
    REPORT_FIELD(region_resource_t, size);
    REPORT_TYPE(serial_connection_resource_t);
    REPORT_FIELD(serial_connection_resource_t, queue);
    REPORT_FIELD(serial_connection_resource_t, data);
    REPORT_FIELD(serial_connection_resource_t, id);
    REPORT_TYPE(serial_driver_config_t);
    REPORT_FIELD(serial_driver_config_t, magic);
    REPORT_FIELD(serial_driver_config_t, rx);
    REPORT_FIELD(serial_driver_config_t, tx);
    REPORT_FIELD(serial_driver_config_t, default_baud);
    REPORT_FIELD(serial_driver_config_t, rx_enabled);
    REPORT_TYPE(serial_virt_rx_config_t);
    REPORT_FIELD(serial_virt_rx_config_t, magic);
    REPORT_FIELD(serial_virt_rx_config_t, driver);
    REPORT_FIELD(serial_virt_rx_config_t, clients);
    REPORT_FIELD(serial_virt_rx_config_t, num_clients);
    REPORT_FIELD(serial_virt_rx_config_t, switch_char);
    REPORT_FIELD(serial_virt_rx_config_t, terminate_num_char);
    REPORT_TYPE(serial_virt_tx_client_config_t);
    REPORT_FIELD(serial_virt_tx_client_config_t, conn);
    REPORT_FIELD(serial_virt_tx_client_config_t, name);
    REPORT_TYPE(serial_virt_tx_config_t);
    REPORT_FIELD(serial_virt_tx_config_t, magic);
    REPORT_FIELD(serial_virt_tx_config_t, driver);
    REPORT_FIELD(serial_virt_tx_config_t, clients);
    REPORT_FIELD(serial_virt_tx_config_t, num_clients);
    REPORT_FIELD(serial_virt_tx_config_t, begin_str);
    REPORT_FIELD(serial_virt_tx_config_t, enable_colour);
    REPORT_FIELD(serial_virt_tx_config_t, enable_rx);
    REPORT_TYPE(serial_client_config_t);
    REPORT_FIELD(serial_client_config_t, magic);
    REPORT_FIELD(serial_client_config_t, rx);
    REPORT_FIELD(serial_client_config_t, tx);
    REPORT_TYPE(net_buff_desc_t);
    REPORT_FIELD(net_buff_desc_t, io_or_offset);
    REPORT_FIELD(net_buff_desc_t, len);
    REPORT_TYPE(net_queue_t);
    REPORT_FIELD(net_queue_t, tail);
    REPORT_FIELD(net_queue_t, head);
    REPORT_FIELD(net_queue_t, consumer_signalled);
    REPORT_TYPE(net_queue_handle_t);
    REPORT_FIELD(net_queue_handle_t, free);
    REPORT_FIELD(net_queue_handle_t, active);
    REPORT_FIELD(net_queue_handle_t, capacity);
    REPORT_TYPE(mac_addr_t);
    REPORT_FIELD(mac_addr_t, addr);
    REPORT_TYPE(net_connection_resource_t);
    REPORT_FIELD(net_connection_resource_t, free_queue);
    REPORT_FIELD(net_connection_resource_t, active_queue);
    REPORT_FIELD(net_connection_resource_t, num_buffers);
    REPORT_FIELD(net_connection_resource_t, id);
    REPORT_TYPE(net_driver_config_t);
    REPORT_FIELD(net_driver_config_t, magic);
    REPORT_FIELD(net_driver_config_t, virt_rx);
    REPORT_FIELD(net_driver_config_t, virt_tx);
    REPORT_TYPE(net_virt_tx_data_region_t);
    REPORT_FIELD(net_virt_tx_data_region_t, data);
    REPORT_FIELD(net_virt_tx_data_region_t, num_buffers);
    REPORT_TYPE(net_virt_tx_client_config_t);
    REPORT_FIELD(net_virt_tx_client_config_t, conn);
    REPORT_FIELD(net_virt_tx_client_config_t, regions);
    REPORT_FIELD(net_virt_tx_client_config_t, num_regions);
    REPORT_TYPE(net_virt_tx_config_t);
    REPORT_FIELD(net_virt_tx_config_t, magic);
    REPORT_FIELD(net_virt_tx_config_t, driver);
    REPORT_FIELD(net_virt_tx_config_t, clients);
    REPORT_FIELD(net_virt_tx_config_t, num_clients);
    REPORT_TYPE(net_virt_rx_client_config_t);
    REPORT_FIELD(net_virt_rx_client_config_t, conn);
    REPORT_FIELD(net_virt_rx_client_config_t, mac_addrs);
    REPORT_FIELD(net_virt_rx_client_config_t, num_macs);
    REPORT_TYPE(net_virt_rx_config_t);
    REPORT_FIELD(net_virt_rx_config_t, magic);
    REPORT_FIELD(net_virt_rx_config_t, driver);
    REPORT_FIELD(net_virt_rx_config_t, data);
    REPORT_FIELD(net_virt_rx_config_t, buffer_metadata);
    REPORT_FIELD(net_virt_rx_config_t, clients);
    REPORT_FIELD(net_virt_rx_config_t, num_clients);
    REPORT_TYPE(net_copy_config_t);
    REPORT_FIELD(net_copy_config_t, magic);
    REPORT_FIELD(net_copy_config_t, rx);
    REPORT_FIELD(net_copy_config_t, rx_data);
    REPORT_FIELD(net_copy_config_t, client);
    REPORT_FIELD(net_copy_config_t, client_data);
    REPORT_TYPE(net_client_config_t);
    REPORT_FIELD(net_client_config_t, magic);
    REPORT_FIELD(net_client_config_t, rx);
    REPORT_FIELD(net_client_config_t, rx_data);
    REPORT_FIELD(net_client_config_t, tx);
    REPORT_FIELD(net_client_config_t, tx_data);
    REPORT_FIELD(net_client_config_t, mac_addr);
    REPORT_TYPE(net_vswitch_port_config_t);
    REPORT_FIELD(net_vswitch_port_config_t, rx);
    REPORT_FIELD(net_vswitch_port_config_t, tx);
    REPORT_FIELD(net_vswitch_port_config_t, tx_data);
    REPORT_FIELD(net_vswitch_port_config_t, mac_addr);
    REPORT_FIELD(net_vswitch_port_config_t, acl);
    REPORT_TYPE(net_vswitch_config_t);
    REPORT_FIELD(net_vswitch_config_t, magic);
    REPORT_FIELD(net_vswitch_config_t, ports);
    REPORT_FIELD(net_vswitch_config_t, num_ports);
    REPORT_FIELD(net_vswitch_config_t, buffer_metadata);
    REPORT_TYPE(blk_req_t);
    REPORT_FIELD(blk_req_t, code);
    REPORT_FIELD(blk_req_t, io_or_offset);
    REPORT_FIELD(blk_req_t, block_number);
    REPORT_FIELD(blk_req_t, count);
    REPORT_FIELD(blk_req_t, id);
    REPORT_TYPE(blk_resp_t);
    REPORT_FIELD(blk_resp_t, status);
    REPORT_FIELD(blk_resp_t, success_count);
    REPORT_FIELD(blk_resp_t, id);
    REPORT_TYPE(blk_req_queue_t);
    REPORT_FIELD(blk_req_queue_t, head);
    REPORT_FIELD(blk_req_queue_t, tail);
    REPORT_FIELD(blk_req_queue_t, plugged);
    REPORT_TYPE(blk_resp_queue_t);
    REPORT_FIELD(blk_resp_queue_t, head);
    REPORT_FIELD(blk_resp_queue_t, tail);
    REPORT_TYPE(blk_queue_handle_t);
    REPORT_FIELD(blk_queue_handle_t, req_queue);
    REPORT_FIELD(blk_queue_handle_t, resp_queue);
    REPORT_FIELD(blk_queue_handle_t, capacity);
    REPORT_TYPE(device_region_resource_t);
    REPORT_FIELD(device_region_resource_t, region);
    REPORT_FIELD(device_region_resource_t, io_addr);
    REPORT_TYPE(blk_connection_resource_t);
    REPORT_FIELD(blk_connection_resource_t, storage_info);
    REPORT_FIELD(blk_connection_resource_t, req_queue);
    REPORT_FIELD(blk_connection_resource_t, resp_queue);
    REPORT_FIELD(blk_connection_resource_t, num_buffers);
    REPORT_FIELD(blk_connection_resource_t, id);
    REPORT_TYPE(blk_driver_config_t);
    REPORT_FIELD(blk_driver_config_t, magic);
    REPORT_FIELD(blk_driver_config_t, virt);
    REPORT_TYPE(blk_virt_config_driver_t);
    REPORT_FIELD(blk_virt_config_driver_t, conn);
    REPORT_FIELD(blk_virt_config_driver_t, data);
    REPORT_TYPE(blk_virt_config_client_t);
    REPORT_FIELD(blk_virt_config_client_t, conn);
    REPORT_FIELD(blk_virt_config_client_t, data);
    REPORT_FIELD(blk_virt_config_client_t, partition);
    REPORT_TYPE(blk_virt_config_t);
    REPORT_FIELD(blk_virt_config_t, magic);
    REPORT_FIELD(blk_virt_config_t, num_clients);
    REPORT_FIELD(blk_virt_config_t, driver);
    REPORT_FIELD(blk_virt_config_t, clients);
    REPORT_TYPE(blk_client_config_t);
    REPORT_FIELD(blk_client_config_t, magic);
    REPORT_FIELD(blk_client_config_t, virt);
    REPORT_FIELD(blk_client_config_t, data);
    REPORT_TYPE(blk_storage_info_t);
    REPORT_FIELD(blk_storage_info_t, serial_number);
    REPORT_FIELD(blk_storage_info_t, read_only);
    REPORT_FIELD(blk_storage_info_t, ready);
    REPORT_FIELD(blk_storage_info_t, sector_size);
    REPORT_FIELD(blk_storage_info_t, block_size);
    REPORT_FIELD(blk_storage_info_t, queue_depth);
    REPORT_FIELD(blk_storage_info_t, cylinders);
    REPORT_FIELD(blk_storage_info_t, heads);
    REPORT_FIELD(blk_storage_info_t, blocks);
    REPORT_FIELD(blk_storage_info_t, capacity);
    REPORT_TYPE(fs_connection_resource_t);
    REPORT_FIELD(fs_connection_resource_t, command_queue);
    REPORT_FIELD(fs_connection_resource_t, completion_queue);
    REPORT_FIELD(fs_connection_resource_t, share);
    REPORT_FIELD(fs_connection_resource_t, queue_len);
    REPORT_FIELD(fs_connection_resource_t, id);
    REPORT_TYPE(fs_server_config_t);
    REPORT_FIELD(fs_server_config_t, magic);
    REPORT_FIELD(fs_server_config_t, client);
    REPORT_TYPE(fs_client_config_t);
    REPORT_FIELD(fs_client_config_t, magic);
    REPORT_FIELD(fs_client_config_t, server);
    REPORT_TYPE(fs_stat_t);
    REPORT_FIELD(fs_stat_t, dev);
    REPORT_FIELD(fs_stat_t, used);
    REPORT_TYPE(fs_cmd_t);
    REPORT_FIELD(fs_cmd_t, id);
    REPORT_FIELD(fs_cmd_t, type);
    REPORT_FIELD(fs_cmd_t, params);
    REPORT_TYPE(fs_cmpl_t);
    REPORT_FIELD(fs_cmpl_t, id);
    REPORT_FIELD(fs_cmpl_t, status);
    REPORT_FIELD(fs_cmpl_t, data);
    REPORT_TYPE(fs_msg_t);
    REPORT_FIELD(fs_msg_t, cmd);
    REPORT_FIELD(fs_msg_t, cmpl);
    REPORT_TYPE(fs_queue_t);
    REPORT_FIELD(fs_queue_t, head);
    REPORT_FIELD(fs_queue_t, tail);
    REPORT_FIELD(fs_queue_t, padding);
    REPORT_FIELD(fs_queue_t, buffer);
    REPORT_TYPE(timer_client_config_t);
    REPORT_FIELD(timer_client_config_t, magic);
    REPORT_FIELD(timer_client_config_t, driver_id);
    REPORT_TYPE(gpu_rect_t);
    REPORT_FIELD(gpu_rect_t, x);
    REPORT_FIELD(gpu_rect_t, y);
    REPORT_FIELD(gpu_rect_t, width);
    REPORT_FIELD(gpu_rect_t, height);
    REPORT_TYPE(gpu_scanout_t);
    REPORT_FIELD(gpu_scanout_t, rect);
    REPORT_FIELD(gpu_scanout_t, enabled);
    REPORT_TYPE(gpu_events_t);
    REPORT_FIELD(gpu_events_t, display_info);
    REPORT_TYPE(gpu_req_get_display_info_t);
    REPORT_FIELD(gpu_req_get_display_info_t, mem_offset);
    REPORT_TYPE(gpu_resp_get_display_info_t);
    REPORT_FIELD(gpu_resp_get_display_info_t, scanouts);
    REPORT_FIELD(gpu_resp_get_display_info_t, num_scanouts);
    REPORT_TYPE(gpu_req_resource_create_blob_t);
    REPORT_FIELD(gpu_req_resource_create_blob_t, resource_id);
    REPORT_FIELD(gpu_req_resource_create_blob_t, mem_offset);
    REPORT_FIELD(gpu_req_resource_create_blob_t, mem_size);
    REPORT_TYPE(gpu_req_set_scanout_blob_t);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, resource_id);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, scanout_id);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, width);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, height);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, format);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, stride);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, offset);
    REPORT_FIELD(gpu_req_set_scanout_blob_t, rect);
    REPORT_TYPE(gpu_req_resource_create_2d_t);
    REPORT_FIELD(gpu_req_resource_create_2d_t, resource_id);
    REPORT_FIELD(gpu_req_resource_create_2d_t, width);
    REPORT_FIELD(gpu_req_resource_create_2d_t, height);
    REPORT_FIELD(gpu_req_resource_create_2d_t, format);
    REPORT_TYPE(gpu_req_resource_attach_backing_t);
    REPORT_FIELD(gpu_req_resource_attach_backing_t, resource_id);
    REPORT_FIELD(gpu_req_resource_attach_backing_t, mem_offset);
    REPORT_FIELD(gpu_req_resource_attach_backing_t, mem_size);
    REPORT_TYPE(gpu_req_resource_detach_backing_t);
    REPORT_FIELD(gpu_req_resource_detach_backing_t, resource_id);
    REPORT_TYPE(gpu_req_set_scanout_t);
    REPORT_FIELD(gpu_req_set_scanout_t, resource_id);
    REPORT_FIELD(gpu_req_set_scanout_t, scanout_id);
    REPORT_FIELD(gpu_req_set_scanout_t, rect);
    REPORT_TYPE(gpu_req_transfer_to_2d_t);
    REPORT_FIELD(gpu_req_transfer_to_2d_t, resource_id);
    REPORT_FIELD(gpu_req_transfer_to_2d_t, rect);
    REPORT_FIELD(gpu_req_transfer_to_2d_t, mem_offset);
    REPORT_TYPE(gpu_req_resource_flush_t);
    REPORT_FIELD(gpu_req_resource_flush_t, resource_id);
    REPORT_FIELD(gpu_req_resource_flush_t, rect);
    REPORT_TYPE(gpu_req_resource_unref_t);
    REPORT_FIELD(gpu_req_resource_unref_t, resource_id);
    REPORT_TYPE(gpu_req_t);
    REPORT_FIELD(gpu_req_t, code);
    REPORT_FIELD(gpu_req_t, id);
    REPORT_FIELD(gpu_req_t, get_display_info);
    REPORT_FIELD(gpu_req_t, resource_create_blob);
    REPORT_FIELD(gpu_req_t, set_scanout_blob);
    REPORT_FIELD(gpu_req_t, resource_create_2d);
    REPORT_FIELD(gpu_req_t, resource_attach_backing);
    REPORT_FIELD(gpu_req_t, resource_detach_backing);
    REPORT_FIELD(gpu_req_t, set_scanout);
    REPORT_FIELD(gpu_req_t, transfer_to_2d);
    REPORT_FIELD(gpu_req_t, resource_flush);
    REPORT_FIELD(gpu_req_t, resource_unref);
    REPORT_TYPE(gpu_resp_t);
    REPORT_FIELD(gpu_resp_t, status);
    REPORT_FIELD(gpu_resp_t, id);
    REPORT_TYPE(gpu_req_queue_t);
    REPORT_FIELD(gpu_req_queue_t, head);
    REPORT_FIELD(gpu_req_queue_t, tail);
    REPORT_TYPE(gpu_resp_queue_t);
    REPORT_FIELD(gpu_resp_queue_t, head);
    REPORT_FIELD(gpu_resp_queue_t, tail);
    REPORT_TYPE(gpu_queue_handle_t);
    REPORT_FIELD(gpu_queue_handle_t, req_queue);
    REPORT_FIELD(gpu_queue_handle_t, resp_queue);
    REPORT_FIELD(gpu_queue_handle_t, capacity);

    printf("const FS_QUEUE_CAPACITY %d\n", FS_QUEUE_CAPACITY);
    printf("const FS_STATUS_SUCCESS %d\n", FS_STATUS_SUCCESS);
    printf("const FS_STATUS_NO_FILE %d\n", FS_STATUS_NO_FILE);
    printf("const FS_STATUS_NOT_EMPTY %d\n", FS_STATUS_NOT_EMPTY);
    printf("const FS_STATUS_NUM_STATUSES %d\n", FS_STATUS_NUM_STATUSES);
    printf("const FS_CMD_INITIALISE %d\n", FS_CMD_INITIALISE);
    printf("const FS_CMD_FILE_OPEN %d\n", FS_CMD_FILE_OPEN);
    printf("const FS_CMD_DIR_REWIND %d\n", FS_CMD_DIR_REWIND);
    printf("const FS_NUM_COMMANDS %d\n", FS_NUM_COMMANDS);
    printf("const FS_OPEN_FLAGS_CREATE %d\n", FS_OPEN_FLAGS_CREATE);
    printf("const BLK_TRANSFER_SIZE %d\n", BLK_TRANSFER_SIZE);
    printf("const NET_BUFFER_SIZE %d\n", NET_BUFFER_SIZE);
    printf("const SDDF_NET_MAX_CLIENTS %d\n", SDDF_NET_MAX_CLIENTS);
    printf("const SDDF_NET_MAGIC_LEN %d\n", SDDF_NET_MAGIC_LEN);
    printf("const MAC802_BYTES %d\n", MAC802_BYTES);
    printf("const SDDF_SERIAL_MAX_CLIENTS %d\n", SDDF_SERIAL_MAX_CLIENTS);
    printf("const SDDF_NAME_LENGTH %d\n", SDDF_NAME_LENGTH);
    printf("const SDDF_SERIAL_BEGIN_STR_MAX_LEN %d\n", SDDF_SERIAL_BEGIN_STR_MAX_LEN);
    printf("const SDDF_SERIAL_MAGIC_LEN %d\n", SDDF_SERIAL_MAGIC_LEN);
    printf("const SDDF_BLK_MAX_CLIENTS %d\n", SDDF_BLK_MAX_CLIENTS);
    printf("const SDDF_BLK_MAGIC_LEN %d\n", SDDF_BLK_MAGIC_LEN);
    printf("const BLK_STORAGE_INFO_REGION_SIZE %d\n", BLK_STORAGE_INFO_REGION_SIZE);
    printf("const BLK_MAX_SERIAL_NUMBER %d\n", BLK_MAX_SERIAL_NUMBER);
    printf("const LIONS_FS_MAGIC_LEN %d\n", LIONS_FS_MAGIC_LEN);
    printf("const BLK_REQ_BARRIER %d\n", BLK_REQ_BARRIER);
    printf("const BLK_RESP_ERR_NO_DEVICE %d\n", BLK_RESP_ERR_NO_DEVICE);
    printf("const SDDF_TIMER_GET_TIME %d\n", SDDF_TIMER_GET_TIME);
    printf("const SDDF_TIMER_SET_TIMEOUT %d\n", SDDF_TIMER_SET_TIMEOUT);
    printf("const SDDF_TIMER_MAGIC_LEN %d\n", SDDF_TIMER_MAGIC_LEN);
    printf("const SDDF_TIMER_MAX_CLIENTS %d\n", SDDF_TIMER_MAX_CLIENTS);
    printf("const GPU_MAX_RESOURCES %d\n", GPU_MAX_RESOURCES);
    printf("const GPU_DISABLE_SCANOUT_RESOURCE_ID %d\n", GPU_DISABLE_SCANOUT_RESOURCE_ID);
    printf("const GPU_BPP_2D %d\n", GPU_BPP_2D);
    printf("const GPU_MAX_SCANOUTS %d\n", GPU_MAX_SCANOUTS);
    printf("const GPU_EVENTS_REGION_SIZE %d\n", GPU_EVENTS_REGION_SIZE);
    printf("const GPU_REQ_GET_DISPLAY_INFO %d\n", GPU_REQ_GET_DISPLAY_INFO);
    printf("const GPU_REQ_RESOURCE_UNREF %d\n", GPU_REQ_RESOURCE_UNREF);
    printf("const GPU_RESP_OK %d\n", GPU_RESP_OK);
    printf("const GPU_RESP_ERR_INVALID_PARAMETER %d\n", GPU_RESP_ERR_INVALID_PARAMETER);
    printf("const GPU_FORMAT_B8G8R8A8_UNORM %d\n", GPU_FORMAT_B8G8R8A8_UNORM);
    printf("const GPU_FORMAT_R8G8B8X8_UNORM %d\n", GPU_FORMAT_R8G8B8X8_UNORM);

    print_desc("fit", 0x2a);
    print_desc("wide", 0xff);

    printf("magic");
    for (i = 0; i < SDDF_SERIAL_MAGIC_LEN; i++) {
        printf(" %02x", (unsigned char)SDDF_SERIAL_MAGIC[i]);
    }
    printf("\n");
    printf("blkmagic");
    for (i = 0; i < SDDF_BLK_MAGIC_LEN; i++) {
        printf(" %02x", (unsigned char)SDDF_BLK_MAGIC[i]);
    }
    printf("\n");
    printf("fsmagic");
    for (i = 0; i < LIONS_FS_MAGIC_LEN; i++) {
        printf(" %02x", (unsigned char)LIONS_FS_MAGIC[i]);
    }
    printf("\n");
    printf("netmagic");
    for (i = 0; i < SDDF_NET_MAGIC_LEN; i++) {
        printf(" %02x", (unsigned char)SDDF_NET_MAGIC[i]);
    }
    printf("\n");
    printf("timermagic");
    for (i = 0; i < SDDF_TIMER_MAGIC_LEN; i++) {
        printf(" %02x", (unsigned char)SDDF_TIMER_MAGIC[i]);
    }
    printf("\n");
    return 0;
}
"#;
