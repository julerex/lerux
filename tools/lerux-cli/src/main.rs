mod agent_sddf;
mod analyze;
mod bench;
mod board;
mod boot_volume;
mod build;
mod build_sdk;
mod channel_consts;
mod channels;
mod ci_lists;
mod clippy;
mod config_cmd;
mod deploy;
mod dev_image;
mod disk_img;
mod dns_one;
mod fat_esp;
mod fetch;
mod fetch_sddf;
mod fs_scaffold;
mod fs_sddf;
mod github_release;
mod gpu_sddf;
mod http_one;
mod https_one;
mod hw_lock;
mod image_digest;
mod image_sign;
mod install;
mod iso;
mod iso_host;
mod libclang;
mod net_sddf;
mod path;
mod process;
mod profile;
mod prog;
mod program_sddf;
mod qemu;
mod qos_check;
mod serial_sddf;
mod smoke_expects;
mod system;
mod tcp_echo;
mod test;

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::{
    board::{get_board, load_boards, print_board_field},
    build_sdk::{build_sdk, sdk_path},
    install::{install_tool, InstallTool},
    process::repo_root,
};

#[derive(Parser)]
#[command(name = "lerux", about = "lerux build and test tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Fetch,
    FetchSdk,
    BuildSdk,
    SdkPath,
    Board {
        name: String,
        field: Option<String>,
    },
    System {
        #[arg(long)]
        board: String,
        #[arg(short = 'o')]
        output: PathBuf,
    },
    HostPath,
    LibclangEnv,
    Install {
        tool: InstallTool,
    },
    DiskImg,
    /// Diff the shell-feature alias for a filesystem board copied from another.
    FsScaffold {
        /// Board whose shell feature and config writer are copied.
        #[arg(long)]
        from: String,
        /// New filesystem board. Its shell feature must enable `from`.
        #[arg(long)]
        board: String,
    },
    /// Cross-target clippy for PD and shared userspace crates (requires SDK).
    Clippy {
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    Build {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    BuildPd {
        crate_name: String,
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    Image {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    Run {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
        /// Phase 70: QEMU gdbstub on `tcp::1234` (`-s`).
        #[arg(long, default_value_t = false)]
        gdb: bool,
        /// Phase 70: QEMU `-snapshot` (do not persist disk.img writes).
        #[arg(long, default_value_t = false)]
        snapshot: bool,
    },
    Test {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
        /// Test driver: `auto` (default), `qemu`, or `hw-serial` (Phase 47).
        /// Also set via `LERUX_TEST_MODE`. hw-serial requires `LERUX_HW_SERIAL`.
        #[arg(long)]
        mode: Option<String>,
        #[arg(long, default_value_t = false)]
        gdb: bool,
        #[arg(long, default_value_t = false)]
        snapshot: bool,
    },
    TestAll {
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    /// Phase 49/57: run echo/blk/net microbenches on QEMU; write md+json summary.
    Bench {
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
        /// Output directory (default: `{build_dir}/bench`).
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Phase 57: fail if metrics miss `support/bench-thresholds.toml`.
        #[arg(long, default_value_t = false)]
        check: bool,
    },
    /// Phase 57: post-process a serial capture for faults/hangs/errors.
    ///
    /// Example: `lerux diagnose build/smoke-logs/qemu_virt_aarch64_workstation.serial.log`
    Diagnose {
        /// Path to serial log, or `-` for stdin.
        path: String,
    },
    TcpEcho {
        #[arg(default_value_t = 18080)]
        port: u16,
    },
    HttpOne {
        #[arg(default_value_t = 8081)]
        port: u16,
    },
    HttpsOne {
        #[arg(default_value_t = 8443)]
        port: u16,
    },
    /// Answer `host` on `127.0.0.1:53` for the browser smoke.
    DnsOne {
        /// Bind directly. A permission failure is returned instead of calling sudo.
        #[arg(long, hide = true, default_value_t = false)]
        root: bool,
    },
    /// Probe whether something is listening on the TCP echo port.
    TcpEchoProbe {
        #[arg(default_value_t = 18080)]
        port: u16,
    },
    /// Run a smoke test against an arbitrary command (test.py-compatible).
    Smoke {
        #[arg(long, action = clap::ArgAction::Append)]
        expect: Vec<String>,
        #[arg(long, num_args = 2, value_names = ["URL", "EXPECT"])]
        curl: Vec<String>,
        #[arg(long)]
        unordered: bool,
        #[arg(long, default_value_t = 60)]
        timeout: u64,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// System profile commands (named PD sets + templates from support/profiles/).
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
    /// ADR-010: compile `support/prog/smoke.rs` and write a signed LRW1 blob.
    Prog {
        #[command(subcommand)]
        command: ProgCommands,
    },
    /// Copy board boot images onto a mounted FAT boot directory.
    ///
    /// `RPi4`: `loader.img` + U-Boot helper. x86 PC: also `sel4.elf` + Multiboot 2 snippets.
    ///
    /// Example: `lerux deploy --board rpi4b_4gb --dest /media/$USER/boot`
    ///
    /// Phase 60 Track C: verifies SHA-256 sidecars before copy unless `--no-verify`.
    Deploy {
        #[arg(long, default_value = "rpi4b_4gb")]
        board: String,
        /// Absolute path to a mounted FAT boot directory (must exist; no `..` segments).
        #[arg(long, short = 'd')]
        dest: PathBuf,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
        /// Build the image if `loader.img` is missing.
        #[arg(long, default_value_t = true)]
        build: bool,
        /// Skip building even if loader.img is missing (error instead).
        #[arg(long, default_value_t = false)]
        no_build: bool,
        /// Verify SHA-256 sidecar before copy (default: true).
        #[arg(long, default_value_t = true)]
        verify: bool,
        /// Skip integrity check (not recommended for field media).
        #[arg(long, default_value_t = false)]
        no_verify: bool,
    },
    /// Build a UEFI ISO that Multiboot 2-boots an x86 board.
    ///
    /// Writes `build/<board>/lerux_<board>.iso` for an x86 board. Does not write a disk.
    Iso {
        #[arg(long, default_value = "x86_64_generic")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
        /// ISO path. Default: `build/<board>/lerux_<board>.iso`.
        /// Relative paths stay under the build directory. Absolute paths must not contain `..`.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Rebuild the Microkit image before packing, even when `loader.img` exists.
        #[arg(long, default_value_t = true)]
        build: bool,
        /// Pack the `loader.img` already on disk. A newer ISO can still hold an older image.
        #[arg(long, default_value_t = false)]
        no_build: bool,
        /// Verify SHA-256 sidecars before packing (default: true).
        #[arg(long, default_value_t = true)]
        verify: bool,
        /// Skip integrity check (not recommended for field media).
        #[arg(long, default_value_t = false)]
        no_verify: bool,
        /// After writing the ISO, boot it in QEMU as a raw disk under OVMF.
        #[arg(long, default_value_t = false)]
        boot_test: bool,
    },
    /// Phase 60 Track C: write `loader.img.sha256` next to an image.
    ///
    /// Also runs automatically after `lerux image`. Format is `sha256sum`-compatible.
    Digest {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        /// Explicit path to `loader.img` (overrides board layout).
        #[arg(long, short = 'p')]
        path: Option<PathBuf>,
    },
    /// Phase 60 Track C / 67: verify digest, and ed25519 sig when present.
    VerifyImage {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        /// Explicit path to `loader.img` (overrides board layout).
        #[arg(long, short = 'p')]
        path: Option<PathBuf>,
        /// Public key (32-byte raw). Required when a `.sig` is present or with `--require-sig`.
        #[arg(long)]
        key: Option<PathBuf>,
        /// Fail if `loader.img.sig` is missing.
        #[arg(long, default_value_t = false)]
        require_sig: bool,
    },
    /// Phase 67: write an ed25519 keypair (`path` + `path.pub`).
    Keygen {
        #[arg(long, default_value = "support/keys/smoke.ed25519")]
        out: PathBuf,
    },
    /// Phase 67: sign `loader.img` with an ed25519 secret key.
    Sign {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, short = 'p')]
        path: Option<PathBuf>,
        #[arg(long, default_value = "support/keys/smoke.ed25519")]
        key: PathBuf,
    },
    /// Phase 54: config schema and disk seed helpers.
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Print the Phase 54 key schema summary.
    Schema,
    /// Print QEMU default key=value pairs.
    Defaults,
    /// Ensure support/disk.img exists (raw MBR; no LERUXFS).
    SeedDisk {
        #[arg(long)]
        disk: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// List available system profiles (e.g. minimal, workstation).
    List,
    /// Build a loader.img for the named profile (uses `default_board` or --board override).
    Build {
        name: String,
        #[arg(long)]
        board: Option<String>,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    /// Diff two profiles (PD set, channels, and composed Microkit SDF).
    Diff {
        profile_a: String,
        profile_b: String,
    },
    /// Show one profile (PDs + structured channels).
    Show { name: String },
    /// Validate structured channel manifests (unique ends, known PDs).
    Validate {
        /// Profile name; omit to validate all profiles under support/profiles/.
        name: Option<String>,
    },
    /// Write the composed Microkit SDF for a profile (`-o` or stdout).
    Sdf {
        name: String,
        #[arg(long)]
        board: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
    /// Emit generated `channel_consts.rs` from the profile manifest.
    EmitChannels {
        name: String,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },
    /// Check PD `Channel::new` consts against the profile manifest.
    CheckChannels {
        /// Profile name; omit to check all profiles.
        name: Option<String>,
    },
    /// Phase 60: capability audit — trust class, PD domains, high-risk edges.
    Audit {
        /// Profile name; omit to audit all profiles under support/profiles/.
        name: Option<String>,
    },
    /// Phase 60 Track D: validate PD priorities and PPC edges (no MCS).
    ///
    /// Checks composed SDF: every `pp="true"` caller has strictly lower
    /// priority than its callee; admin/admin-core profiles also get
    /// service-class band checks (docs/qos.md).
    CheckQos {
        /// Profile name; omit to check all profiles with a `default_board`.
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProgCommands {
    /// Compile the smoke Rust program and sign the Wasm payload.
    Pack {
        /// Ed25519 secret (32 raw bytes). Relative paths are repo-rooted.
        #[arg(long)]
        key: PathBuf,
        /// Output `LRW1` blob. Relative paths are repo-rooted.
        #[arg(long, short = 'o')]
        out: PathBuf,
        /// Rust source. Default: `support/prog/smoke.rs`.
        #[arg(long)]
        src: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = repo_root()?;
    dispatch(&root, cli.command)
}

fn dispatch(root: &Path, command: Commands) -> Result<()> {
    match command {
        cmd @ (Commands::Fetch
        | Commands::FetchSdk
        | Commands::BuildSdk
        | Commands::SdkPath
        | Commands::Board { .. }
        | Commands::System { .. }
        | Commands::HostPath
        | Commands::LibclangEnv
        | Commands::Install { .. }
        | Commands::DiskImg
        | Commands::FsScaffold { .. }) => dispatch_source(root, cmd),
        cmd @ (Commands::Clippy { .. }
        | Commands::Build { .. }
        | Commands::BuildPd { .. }
        | Commands::Image { .. }
        | Commands::TestAll { .. }
        | Commands::Bench { .. }
        | Commands::Diagnose { .. }
        | Commands::TcpEcho { .. }
        | Commands::HttpOne { .. }
        | Commands::HttpsOne { .. }
        | Commands::DnsOne { .. }
        | Commands::TcpEchoProbe { .. }) => dispatch_host(root, cmd),
        cmd @ (Commands::Run { .. }
        | Commands::Test { .. }
        | Commands::Smoke { .. }
        | Commands::Profile { .. }
        | Commands::Prog { .. }) => dispatch_session(root, cmd),
        cmd @ (Commands::Deploy { .. }
        | Commands::Iso { .. }
        | Commands::Digest { .. }
        | Commands::VerifyImage { .. }
        | Commands::Keygen { .. }
        | Commands::Sign { .. }
        | Commands::Config { .. }) => dispatch_ship(root, cmd),
    }
}

fn dispatch_source(root: &Path, command: Commands) -> Result<()> {
    match command {
        Commands::Fetch => fetch::fetch(root),
        Commands::FetchSdk => fetch::fetch_sdk(root),
        Commands::BuildSdk => build_sdk(root),
        Commands::SdkPath => print_sdk_path(root),
        Commands::Board { name, field } => show_board(root, &name, field.as_deref()),
        Commands::System { board, output } => system::generate_system(root, &board, &output),
        Commands::HostPath => {
            print_host_path(root);
            Ok(())
        }
        Commands::LibclangEnv => {
            libclang::print_libclang_env(root);
            Ok(())
        }
        Commands::Install { tool } => print_install(root, tool),
        Commands::DiskImg => disk_img::disk_img(root),
        Commands::FsScaffold { from, board } => print_fs_scaffold(root, &from, &board),
        _ => unreachable!("source command"),
    }
}

fn dispatch_host(root: &Path, command: Commands) -> Result<()> {
    match command {
        Commands::Clippy { build_dir, config } => {
            clippy::clippy_workspace(root, &build_dir, &config)
        }
        Commands::Build {
            board,
            build_dir,
            config,
        } => build::build(root, &board, &build_dir, &config),
        Commands::BuildPd {
            crate_name,
            board,
            build_dir,
            config,
        } => build::build_pd(root, &board, &build_dir, &config, &crate_name),
        Commands::Image {
            board,
            build_dir,
            config,
        } => build::image(root, &board, &build_dir, &config),
        Commands::TestAll { build_dir, config } => build::test_all(root, &build_dir, &config),
        Commands::Bench {
            build_dir,
            config,
            out_dir,
            check,
        } => bench::run_bench(root, &build_dir, &config, out_dir.as_deref(), check),
        Commands::Diagnose { path } => analyze::analyze_log(&path),
        Commands::TcpEcho { port } => tcp_echo::tcp_echo(port),
        Commands::HttpOne { port } => http_one::http_one(port),
        Commands::HttpsOne { port } => https_one::https_one(port),
        Commands::DnsOne { root } => dns_one::dns_one(root),
        Commands::TcpEchoProbe { port } => probe_tcp_echo(port),
        _ => unreachable!("host command"),
    }
}

fn dispatch_session(root: &Path, command: Commands) -> Result<()> {
    match command {
        Commands::Run {
            board,
            build_dir,
            config,
            gdb,
            snapshot,
        } => run_command(root, &board, &build_dir, &config, gdb, snapshot),
        Commands::Test {
            board,
            build_dir,
            config,
            mode,
            gdb,
            snapshot,
        } => test_command(
            root,
            &board,
            &build_dir,
            &config,
            mode.as_deref(),
            gdb,
            snapshot,
        ),
        Commands::Smoke {
            expect,
            curl,
            unordered,
            timeout,
            cmd,
        } => smoke_command(expect, &curl, unordered, timeout, cmd),
        Commands::Profile { command } => profile_command(root, command),
        Commands::Prog { command } => prog_command(command),
        _ => unreachable!("session command"),
    }
}

fn dispatch_ship(root: &Path, command: Commands) -> Result<()> {
    match command {
        cmd @ (Commands::Deploy { .. } | Commands::Iso { .. } | Commands::Digest { .. }) => {
            dispatch_ship_boot(root, cmd)
        }
        cmd => dispatch_ship_keys(root, cmd),
    }
}

fn dispatch_ship_boot(root: &Path, command: Commands) -> Result<()> {
    match command {
        cmd @ Commands::Deploy { .. } => deploy_from(root, cmd),
        cmd @ Commands::Iso { .. } => iso_from(root, cmd),
        Commands::Digest {
            board,
            build_dir,
            path,
        } => digest_command(root, &board, &build_dir, path.as_deref()),
        _ => unreachable!("ship boot command"),
    }
}

fn deploy_from(root: &Path, command: Commands) -> Result<()> {
    let Commands::Deploy {
        board,
        dest,
        build_dir,
        config,
        build,
        no_build,
        verify,
        no_verify,
    } = command
    else {
        unreachable!("deploy command");
    };
    deploy_command(
        root,
        &board,
        &dest,
        &build_dir,
        &config,
        &ImageFlags {
            build_if_missing: build && !no_build,
            verify: verify && !no_verify,
        },
    )
}

fn iso_from(root: &Path, command: Commands) -> Result<()> {
    let Commands::Iso {
        board,
        build_dir,
        config,
        output,
        build,
        no_build,
        verify,
        no_verify,
        boot_test,
    } = command
    else {
        unreachable!("iso command");
    };
    iso_command(
        root,
        &board,
        &build_dir,
        &config,
        output.as_deref(),
        &ImageFlags {
            build_if_missing: build && !no_build,
            verify: verify && !no_verify,
        },
        boot_test,
    )
}

fn dispatch_ship_keys(root: &Path, command: Commands) -> Result<()> {
    match command {
        Commands::VerifyImage {
            board,
            build_dir,
            path,
            key,
            require_sig,
        } => verify_image_command(root, &board, &build_dir, path.as_deref(), key, require_sig),
        Commands::Keygen { out } => keygen_command(root, out),
        Commands::Sign {
            board,
            build_dir,
            path,
            key,
        } => sign_command(root, &board, &build_dir, path.as_deref(), key),
        Commands::Config { command } => config_command(root, command),
        _ => unreachable!("ship command"),
    }
}

fn print_sdk_path(root: &Path) -> Result<()> {
    println!("{}", sdk_path(root)?);
    Ok(())
}

fn show_board(root: &Path, name: &str, field: Option<&str>) -> Result<()> {
    let boards = load_boards(root)?;
    let board = get_board(&boards, name)?;
    print_board_field(board, field)
}

fn print_host_path(root: &Path) {
    println!("{}", path::host_path(root));
}

fn print_install(root: &Path, tool: InstallTool) -> Result<()> {
    let bin = install_tool(root, tool)?;
    println!("{}", bin.display());
    Ok(())
}

fn print_fs_scaffold(root: &Path, from: &str, board: &str) -> Result<()> {
    print!("{}", fs_scaffold::report(root, from, board)?);
    Ok(())
}

fn probe_tcp_echo(port: u16) -> Result<()> {
    let code = i32::from(!tcp_echo::port_is_listening(port));
    std::process::exit(code);
}

fn apply_qemu_session_flags(gdb: bool, snapshot: bool) {
    if gdb {
        // SAFETY: CLI is single-threaded here.
        unsafe { std::env::set_var("LERUX_QEMU_GDB", "1") };
    }
    if snapshot {
        // SAFETY: CLI is single-threaded here.
        unsafe { std::env::set_var("LERUX_QEMU_SNAPSHOT", "1") };
    }
}

fn run_command(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    gdb: bool,
    snapshot: bool,
) -> Result<()> {
    apply_qemu_session_flags(gdb, snapshot);
    build::run(root, board, build_dir, config)
}

fn test_command(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    mode: Option<&str>,
    gdb: bool,
    snapshot: bool,
) -> Result<()> {
    apply_qemu_session_flags(gdb, snapshot);
    let mode = test::TestMode::from_env_or_flag(mode)?;
    build::image(root, board, build_dir, config)?;
    test::run_board_test_with_mode(root, board, build_dir, config, mode)
}

fn smoke_curls(curl: &[String]) -> Vec<(String, String)> {
    curl.chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some((c[0].clone(), c[1].clone()))
            } else {
                None
            }
        })
        .collect()
}

fn smoke_command(
    expect: Vec<String>,
    curl: &[String],
    unordered: bool,
    timeout: u64,
    mut cmd: Vec<String>,
) -> Result<()> {
    if cmd.first().is_some_and(|a| a == "--") {
        cmd.remove(0);
    }
    let (program, program_args) = cmd
        .split_first()
        .map_or_else(|| ("true".into(), vec![]), |(p, a)| (p.clone(), a.to_vec()));
    let mut command = Command::new(program);
    command.args(program_args);
    let smoke = test::SmokeTest {
        expects: if expect.is_empty() {
            vec!["lerux: Hello from Rust on seL4 Microkit!".into()]
        } else {
            expect
        },
        curls: smoke_curls(curl),
        unordered,
        timeout_secs: timeout,
        script: Vec::new(),
        script_timeout_secs: 30,
        qmp_keys: Vec::new(),
        qmp_expect: None,
    };
    test::run_smoke(command, &smoke)
}

fn profile_command(root: &Path, command: ProfileCommands) -> Result<()> {
    let profiles = crate::profile::load_profiles(root)?;
    match command {
        ProfileCommands::List => {
            crate::profile::list_profiles(&profiles);
            Ok(())
        }
        ProfileCommands::Build {
            name,
            board,
            build_dir,
            config,
        } => profile_build(
            root,
            &profiles,
            &name,
            board.as_deref(),
            &build_dir,
            &config,
        ),
        ProfileCommands::Diff {
            profile_a,
            profile_b,
        } => profile_diff(root, &profiles, &profile_a, &profile_b),
        ProfileCommands::Show { name } => profile_show(&profiles, &name),
        ProfileCommands::Validate { name } => profile_validate(&profiles, name),
        ProfileCommands::Sdf {
            name,
            board,
            output,
        } => profile_sdf(root, &profiles, &name, board.as_deref(), output),
        ProfileCommands::EmitChannels { name, output } => {
            profile_emit_channels(&profiles, &name, output)
        }
        ProfileCommands::CheckChannels { name } => profile_check_channels(root, &profiles, name),
        ProfileCommands::Audit { name } => {
            crate::profile::audit_profiles(&profiles, name.as_deref())
        }
        ProfileCommands::CheckQos { name } => {
            crate::qos_check::check_profiles_qos(root, &profiles, name.as_deref())
        }
    }
}

fn profile_build(
    root: &Path,
    profiles: &crate::profile::Profiles,
    name: &str,
    board: Option<&str>,
    build_dir: &str,
    config: &str,
) -> Result<()> {
    let profile = crate::profile::get_profile(profiles, name)?;
    let board_name = crate::profile::resolve_board_for_profile(profiles, name, board)?;
    let sdf = crate::system::render_profile_system(root, name, profile, board)?;
    let out = crate::system::system_file(root, &board_name, build_dir);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&out, sdf).with_context(|| format!("write {}", out.display()))?;
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, &board_name)?;
    build::link_image(root, &board_name, build_dir, config, &board_cfg.pds)?;
    println!("profile {name} -> board {board_name}: loader.img ready");
    Ok(())
}

fn profile_diff(
    root: &Path,
    profiles: &crate::profile::Profiles,
    profile_a: &str,
    profile_b: &str,
) -> Result<()> {
    let pa = crate::profile::get_profile(profiles, profile_a)?;
    let pb = crate::profile::get_profile(profiles, profile_b)?;
    crate::profile::diff_profiles_with_sdf(root, profile_a, pa, profile_b, pb)
}

fn profile_show(profiles: &crate::profile::Profiles, name: &str) -> Result<()> {
    let p = crate::profile::get_profile(profiles, name)?;
    crate::profile::show_profile(name, p);
    Ok(())
}

fn profile_validate(profiles: &crate::profile::Profiles, name: Option<String>) -> Result<()> {
    if let Some(name) = name {
        let p = crate::profile::get_profile(profiles, &name)?;
        crate::profile::validate_profile(&name, p)?;
    } else {
        crate::profile::validate_all_profiles(profiles)?;
    }
    Ok(())
}

fn profile_sdf(
    root: &Path,
    profiles: &crate::profile::Profiles,
    name: &str,
    board: Option<&str>,
    output: Option<PathBuf>,
) -> Result<()> {
    let p = crate::profile::get_profile(profiles, name)?;
    let sdf = crate::system::render_profile_system(root, name, p, board)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &sdf)?;
        println!("wrote {} ({} bytes)", path.display(), sdf.len());
    } else {
        print!("{sdf}");
    }
    Ok(())
}

fn profile_emit_channels(
    profiles: &crate::profile::Profiles,
    name: &str,
    output: Option<PathBuf>,
) -> Result<()> {
    let p = crate::profile::get_profile(profiles, name)?;
    let body = crate::channel_consts::emit_channel_consts_rs(name, &p.channel);
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &body)?;
        println!("wrote {}", path.display());
    } else {
        print!("{body}");
    }
    Ok(())
}

fn profile_check_channels(
    root: &Path,
    profiles: &crate::profile::Profiles,
    name: Option<String>,
) -> Result<()> {
    if let Some(name) = name {
        let p = crate::profile::get_profile(profiles, &name)?;
        crate::channel_consts::check_profile_channels(root, &name, p)?;
        println!("profile {name}: channel consts ok");
    } else {
        for (name, p) in profiles {
            crate::channel_consts::check_profile_channels(root, name, p)?;
            println!("profile {name}: channel consts ok");
        }
    }
    Ok(())
}

fn prog_command(command: ProgCommands) -> Result<()> {
    match command {
        ProgCommands::Pack { key, out, src } => prog::pack_cli(&key, &out, src.as_deref()),
    }
}

struct ImageFlags {
    build_if_missing: bool,
    verify: bool,
}

fn deploy_command(
    root: &Path,
    board: &str,
    dest: &Path,
    build_dir: &str,
    config: &str,
    flags: &ImageFlags,
) -> Result<()> {
    crate::deploy::deploy_loader(
        root,
        board,
        build_dir,
        config,
        dest,
        flags.build_if_missing,
        flags.verify,
    )?;
    Ok(())
}

fn iso_command(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    output: Option<&Path>,
    flags: &ImageFlags,
    boot_test: bool,
) -> Result<()> {
    crate::iso_host::require_iso_host(root, board, config, flags.build_if_missing, boot_test)?;
    let iso = crate::iso::build_iso(
        root,
        board,
        build_dir,
        config,
        output,
        flags.build_if_missing,
        flags.verify,
    )?;
    if boot_test {
        crate::iso::boot_test(root, &iso, board, build_dir)?;
    }
    Ok(())
}

fn digest_command(root: &Path, board: &str, build_dir: &str, path: Option<&Path>) -> Result<()> {
    let loader = crate::image_digest::resolve_loader(root, board, build_dir, path);
    crate::image_digest::write_sidecar(&loader)?;
    Ok(())
}

fn verify_image_command(
    root: &Path,
    board: &str,
    build_dir: &str,
    path: Option<&Path>,
    key: Option<PathBuf>,
    require_sig: bool,
) -> Result<()> {
    let loader = crate::image_digest::resolve_loader(root, board, build_dir, path);
    let key = key.map(|k| if k.is_absolute() { k } else { root.join(k) });
    crate::image_sign::verify_image(&loader, key.as_deref(), require_sig)
}

fn keygen_command(root: &Path, out: PathBuf) -> Result<()> {
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    crate::image_sign::keygen(&out)
}

fn sign_command(
    root: &Path,
    board: &str,
    build_dir: &str,
    path: Option<&Path>,
    key: PathBuf,
) -> Result<()> {
    let loader = crate::image_digest::resolve_loader(root, board, build_dir, path);
    let key = if key.is_absolute() {
        key
    } else {
        root.join(key)
    };
    crate::image_sign::sign(&loader, &key)
}

fn config_command(root: &Path, command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Schema => {
            crate::config_cmd::print_schema();
            Ok(())
        }
        ConfigCommands::Defaults => {
            crate::config_cmd::print_defaults();
            Ok(())
        }
        ConfigCommands::SeedDisk { disk } => crate::config_cmd::seed_disk(root, disk.as_deref()),
    }
}
