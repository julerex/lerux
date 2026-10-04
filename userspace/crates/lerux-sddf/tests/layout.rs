//! Compare `lerux-sddf` with the LionsOS 0.4.0 headers.
//!
//! The host C compiler is the oracle. When `deps/workspace/lionsos` is absent
//! the comparison is skipped and this test passes, so `just check` can run
//! before `lerux fetch`. After fetch, a size, alignment, or field-offset
//! mismatch fails the build.
//!
//! `MICROKIT_PD_NAME_LENGTH` is supplied by a stub. LionsOS 0.4.0 uses Microkit
//! 2.3.0, which defines that length as 64. The stub exists so the driver-framework
//! headers can be parsed without linking seL4.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use lerux_sddf::{
    blk_client_config_t, blk_connection_resource_t, blk_driver_config_t, blk_queue_handle_t,
    blk_req_code_t::BLK_REQ_BARRIER, blk_req_queue_t, blk_req_t, blk_resp_queue_t,
    blk_resp_status_t::BLK_RESP_ERR_NO_DEVICE, blk_resp_t, blk_storage_info_t,
    blk_virt_config_client_t, blk_virt_config_driver_t, blk_virt_config_t,
    device_region_resource_t, fs_client_config_t, fs_cmd_t, fs_cmpl_t, fs_connection_resource_t,
    fs_msg_t, fs_queue_t, fs_server_config_t, fs_stat_t, net_buff_desc_t, net_queue_handle_t,
    net_queue_t, region_resource_t, serial_client_config_t, serial_connection_resource_t,
    serial_driver_config_t, serial_queue_handle_t, serial_queue_t, serial_virt_rx_config_t,
    serial_virt_tx_client_config_t, serial_virt_tx_config_t, BLK_MAX_SERIAL_NUMBER,
    BLK_STORAGE_INFO_REGION_SIZE, BLK_TRANSFER_SIZE, FS_CMD_DIR_REWIND, FS_CMD_FILE_OPEN,
    FS_CMD_INITIALISE, FS_NUM_COMMANDS, FS_OPEN_FLAGS_CREATE, FS_QUEUE_CAPACITY,
    FS_STATUS_NOT_EMPTY, FS_STATUS_NO_FILE, FS_STATUS_NUM_STATUSES, FS_STATUS_SUCCESS,
    LIONS_FS_MAGIC, LIONS_FS_MAGIC_LEN, NET_BUFFER_SIZE, SDDF_BLK_MAGIC, SDDF_BLK_MAGIC_LEN,
    SDDF_BLK_MAX_CLIENTS, SDDF_NAME_LENGTH, SDDF_SERIAL_BEGIN_STR_MAX_LEN, SDDF_SERIAL_MAGIC,
    SDDF_SERIAL_MAGIC_LEN, SDDF_SERIAL_MAX_CLIENTS,
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
}

#[test]
fn rust_layouts_match_lionsos_headers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let protocol = root.join("deps/workspace/lionsos/include/lions/fs/protocol.h");
    let serial_queue = root.join("deps/workspace/lionsos/dep/sddf/include/sddf/serial/queue.h");
    if !protocol.is_file() || !serial_queue.is_file() {
        eprintln!("lerux fetch is required before the LionsOS layout comparison is meaningful");
        eprintln!("missing {}", protocol.display());
        eprintln!("missing {}", serial_queue.display());
        return;
    }

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
    ];
    fields.push(("fs_cmd_t", "type", core::mem::offset_of!(fs_cmd_t, r#type)));
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
#include <sddf/blk/queue.h>
#include <sddf/blk/storage_info.h>
#include <sddf/network/queue.h>
#include <sddf/resources/device.h>
#include <sddf/serial/config.h>
#include <sddf/serial/queue.h>

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
    return 0;
}
"#;
