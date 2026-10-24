mod agent_sddf;
mod analyze;
mod bench;
mod board;
mod build;
mod build_sdk;
mod channel_consts;
mod channels;
mod ci_lists;
mod clippy;
mod config_cmd;
mod deploy;
mod disk_img;
mod fetch;
mod fetch_sddf;
mod fs_scaffold;
mod fs_sddf;
mod gpu_sddf;
mod http_one;
mod https_one;
mod hw_lock;
mod image_digest;
mod image_sign;
mod install;
mod iso;
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

use std::{path::PathBuf, process::Command};

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
        /// Phase 70: QEMU gdbstub on tcp::1234 (`-s`).
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
    /// RPi4: `loader.img` + U-Boot helper. x86 PC: also `sel4_32.elf` + Multiboot 2 snippets.
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
    /// Build a hybrid BIOS+UEFI ISO that Multiboot 2-boots an x86 board.
    ///
    /// Writes `build/<board>/lerux.iso` for an x86 board. Does not write a disk.
    Iso {
        #[arg(long, default_value = "x86_64_generic")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
        /// ISO path. Default: `build/<board>/lerux.iso`.
        /// Relative paths stay under the build directory. Absolute paths must not contain `..`.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Build the image if `loader.img` or `sel4_32.elf` is missing.
        #[arg(long, default_value_t = true)]
        build: bool,
        /// Skip building even if the boot files are missing (error instead).
        #[arg(long, default_value_t = false)]
        no_build: bool,
        /// Verify SHA-256 sidecars before packing (default: true).
        #[arg(long, default_value_t = true)]
        verify: bool,
        /// Skip integrity check (not recommended for field media).
        #[arg(long, default_value_t = false)]
        no_verify: bool,
        /// After writing the ISO, boot it in QEMU as a raw disk under SeaBIOS.
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
    /// Build a loader.img for the named profile (uses default_board or --board override).
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
        /// Profile name; omit to check all profiles with a default_board.
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

    match cli.command {
        Commands::Fetch => fetch::fetch(&root)?,
        Commands::FetchSdk => fetch::fetch_sdk(&root)?,
        Commands::BuildSdk => build_sdk(&root)?,
        Commands::SdkPath => println!("{}", sdk_path(&root)?),
        Commands::Board { name, field } => {
            let boards = load_boards(&root)?;
            let board = get_board(&boards, &name)?;
            print_board_field(board, field.as_deref())?;
        }
        Commands::System { board, output } => {
            system::generate_system(&root, &board, &output)?;
        }
        Commands::HostPath => println!("{}", path::host_path(&root)),
        Commands::LibclangEnv => libclang::print_libclang_env(&root),
        Commands::Install { tool } => {
            let bin = install_tool(&root, tool)?;
            println!("{}", bin.display());
        }
        Commands::DiskImg => disk_img::disk_img(&root)?,
        Commands::FsScaffold { from, board } => {
            print!("{}", fs_scaffold::report(&root, &from, &board)?);
        }
        Commands::Clippy { build_dir, config } => {
            clippy::clippy_workspace(&root, &build_dir, &config)?;
        }
        Commands::Build {
            board,
            build_dir,
            config,
        } => {
            build::build(&root, &board, &build_dir, &config)?;
        }
        Commands::BuildPd {
            crate_name,
            board,
            build_dir,
            config,
        } => build::build_pd(&root, &board, &build_dir, &config, &crate_name)?,
        Commands::Image {
            board,
            build_dir,
            config,
        } => {
            build::image(&root, &board, &build_dir, &config)?;
        }
        Commands::Run {
            board,
            build_dir,
            config,
            gdb,
            snapshot,
        } => {
            if gdb {
                // SAFETY: CLI is single-threaded here.
                unsafe { std::env::set_var("LERUX_QEMU_GDB", "1") };
            }
            if snapshot {
                // SAFETY: CLI is single-threaded here.
                unsafe { std::env::set_var("LERUX_QEMU_SNAPSHOT", "1") };
            }
            build::run(&root, &board, &build_dir, &config)?;
        }
        Commands::Test {
            board,
            build_dir,
            config,
            mode,
            gdb,
            snapshot,
        } => {
            if gdb {
                // SAFETY: CLI is single-threaded here.
                unsafe { std::env::set_var("LERUX_QEMU_GDB", "1") };
            }
            if snapshot {
                // SAFETY: CLI is single-threaded here.
                unsafe { std::env::set_var("LERUX_QEMU_SNAPSHOT", "1") };
            }
            let mode = test::TestMode::from_env_or_flag(mode.as_deref())?;
            build::image(&root, &board, &build_dir, &config)?;
            test::run_board_test_with_mode(&root, &board, &build_dir, &config, mode)?;
        }
        Commands::TestAll { build_dir, config } => {
            build::test_all(&root, &build_dir, &config)?;
        }
        Commands::Bench {
            build_dir,
            config,
            out_dir,
            check,
        } => {
            bench::run_bench(&root, &build_dir, &config, out_dir.as_deref(), check)?;
        }
        Commands::Diagnose { path } => {
            analyze::analyze_log(&path)?;
        }
        Commands::TcpEcho { port } => tcp_echo::tcp_echo(port)?,
        Commands::HttpOne { port } => http_one::http_one(port)?,
        Commands::HttpsOne { port } => https_one::https_one(port)?,
        Commands::TcpEchoProbe { port } => {
            let code = if tcp_echo::port_is_listening(port) {
                0
            } else {
                1
            };
            std::process::exit(code);
        }
        Commands::Smoke {
            expect,
            curl,
            unordered,
            timeout,
            cmd,
        } => {
            let mut args = cmd;
            if args.first().is_some_and(|a| a == "--") {
                args.remove(0);
            }
            let (program, program_args) = args
                .split_first()
                .map(|(p, a)| (p.clone(), a.to_vec()))
                .unwrap_or_else(|| ("true".into(), vec![]));
            let mut command = Command::new(program);
            command.args(program_args);
            let curls: Vec<(String, String)> = curl
                .chunks(2)
                .filter_map(|c| {
                    if c.len() == 2 {
                        Some((c[0].clone(), c[1].clone()))
                    } else {
                        None
                    }
                })
                .collect();
            let smoke = test::SmokeTest {
                expects: if expect.is_empty() {
                    vec!["lerux: Hello from Rust on seL4 Microkit!".into()]
                } else {
                    expect
                },
                curls,
                unordered,
                timeout_secs: timeout,
                script: Vec::new(),
                script_timeout_secs: 30,
                qmp_keys: Vec::new(),
                qmp_expect: None,
            };
            test::run_smoke(command, &smoke)?;
        }
        Commands::Profile { command } => {
            let profiles = crate::profile::load_profiles(&root)?;
            match command {
                ProfileCommands::List => {
                    crate::profile::list_profiles(&profiles);
                }
                ProfileCommands::Build {
                    name,
                    board,
                    build_dir,
                    config,
                } => {
                    let profile = crate::profile::get_profile(&profiles, &name)?;
                    let board_name = crate::profile::resolve_board_for_profile(
                        &profiles,
                        &name,
                        board.as_deref(),
                    )?;
                    let sdf = crate::system::render_profile_system(
                        &root,
                        &name,
                        profile,
                        board.as_deref(),
                    )?;
                    let out = crate::system::system_file(&root, &board_name, &build_dir);
                    if let Some(parent) = out.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("mkdir {}", parent.display()))?;
                    }
                    std::fs::write(&out, sdf)
                        .with_context(|| format!("write {}", out.display()))?;
                    let boards = load_boards(&root)?;
                    let board_cfg = get_board(&boards, &board_name)?;
                    build::link_image(&root, &board_name, &build_dir, &config, &board_cfg.pds)?;
                    println!("profile {name} -> board {board_name}: loader.img ready");
                }
                ProfileCommands::Diff {
                    profile_a,
                    profile_b,
                } => {
                    let pa = crate::profile::get_profile(&profiles, &profile_a)?;
                    let pb = crate::profile::get_profile(&profiles, &profile_b)?;
                    crate::profile::diff_profiles_with_sdf(&root, &profile_a, pa, &profile_b, pb)?;
                }
                ProfileCommands::Show { name } => {
                    let p = crate::profile::get_profile(&profiles, &name)?;
                    crate::profile::show_profile(&name, p);
                }
                ProfileCommands::Validate { name } => {
                    if let Some(name) = name {
                        let p = crate::profile::get_profile(&profiles, &name)?;
                        crate::profile::validate_profile(&name, p)?;
                    } else {
                        crate::profile::validate_all_profiles(&profiles)?;
                    }
                }
                ProfileCommands::Sdf {
                    name,
                    board,
                    output,
                } => {
                    let p = crate::profile::get_profile(&profiles, &name)?;
                    let sdf =
                        crate::system::render_profile_system(&root, &name, p, board.as_deref())?;
                    if let Some(path) = output {
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&path, &sdf)?;
                        println!("wrote {} ({} bytes)", path.display(), sdf.len());
                    } else {
                        print!("{sdf}");
                    }
                }
                ProfileCommands::EmitChannels { name, output } => {
                    let p = crate::profile::get_profile(&profiles, &name)?;
                    let body = crate::channel_consts::emit_channel_consts_rs(&name, &p.channel);
                    if let Some(path) = output {
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&path, &body)?;
                        println!("wrote {}", path.display());
                    } else {
                        print!("{body}");
                    }
                }
                ProfileCommands::CheckChannels { name } => {
                    if let Some(name) = name {
                        let p = crate::profile::get_profile(&profiles, &name)?;
                        crate::channel_consts::check_profile_channels(&root, &name, p)?;
                        println!("profile {name}: channel consts ok");
                    } else {
                        for (name, p) in &profiles {
                            crate::channel_consts::check_profile_channels(&root, name, p)?;
                            println!("profile {name}: channel consts ok");
                        }
                    }
                }
                ProfileCommands::Audit { name } => {
                    crate::profile::audit_profiles(&profiles, name.as_deref())?;
                }
                ProfileCommands::CheckQos { name } => {
                    crate::qos_check::check_profiles_qos(&root, &profiles, name.as_deref())?;
                }
            }
        }
        Commands::Prog { command } => match command {
            ProgCommands::Pack { key, out, src } => {
                prog::pack_cli(&key, &out, src.as_deref())?;
            }
        },
        Commands::Deploy {
            board,
            dest,
            build_dir,
            config,
            build,
            no_build,
            verify,
            no_verify,
        } => {
            let build_if_missing = build && !no_build;
            let do_verify = verify && !no_verify;
            crate::deploy::deploy_loader(
                &root,
                &board,
                &build_dir,
                &config,
                &dest,
                build_if_missing,
                do_verify,
            )?;
        }
        Commands::Iso {
            board,
            build_dir,
            config,
            output,
            build,
            no_build,
            verify,
            no_verify,
            boot_test,
        } => {
            let build_if_missing = build && !no_build;
            let do_verify = verify && !no_verify;
            let iso = crate::iso::build_iso(
                &root,
                &board,
                &build_dir,
                &config,
                output.as_deref(),
                build_if_missing,
                do_verify,
            )?;
            if boot_test {
                crate::iso::boot_test(&root, &iso, &board, &build_dir)?;
            }
        }
        Commands::Digest {
            board,
            build_dir,
            path,
        } => {
            let loader =
                crate::image_digest::resolve_loader(&root, &board, &build_dir, path.as_deref())?;
            crate::image_digest::write_sidecar(&loader)?;
        }
        Commands::VerifyImage {
            board,
            build_dir,
            path,
            key,
            require_sig,
        } => {
            let loader =
                crate::image_digest::resolve_loader(&root, &board, &build_dir, path.as_deref())?;
            let key = key.map(|k| if k.is_absolute() { k } else { root.join(k) });
            crate::image_sign::verify_image(&loader, key.as_deref(), require_sig)?;
        }
        Commands::Keygen { out } => {
            let out = if out.is_absolute() {
                out
            } else {
                root.join(out)
            };
            crate::image_sign::keygen(&out)?;
        }
        Commands::Sign {
            board,
            build_dir,
            path,
            key,
        } => {
            let loader =
                crate::image_digest::resolve_loader(&root, &board, &build_dir, path.as_deref())?;
            let key = if key.is_absolute() {
                key
            } else {
                root.join(key)
            };
            crate::image_sign::sign(&loader, &key)?;
        }
        Commands::Config { command } => match command {
            ConfigCommands::Schema => crate::config_cmd::print_schema(),
            ConfigCommands::Defaults => crate::config_cmd::print_defaults(),
            ConfigCommands::SeedDisk { disk } => {
                crate::config_cmd::seed_disk(&root, disk.as_deref())?;
            }
        },
    }

    Ok(())
}
