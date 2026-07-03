mod board;
mod build;
mod build_sdk;
mod clippy;
mod disk_img;
mod fetch;
mod http_one;
mod install;
mod libclang;
mod path;
mod process;
mod profile;
mod qemu;
mod system;
mod tcp_echo;
mod test;

use std::{path::PathBuf, process::Command};

use anyhow::Result;
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
    },
    Test {
        #[arg(long, default_value = "qemu_virt_aarch64")]
        board: String,
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    TestAll {
        #[arg(long, default_value = "build")]
        build_dir: String,
        #[arg(long, default_value = "debug")]
        config: String,
    },
    TcpEcho {
        #[arg(default_value_t = 18080)]
        port: u16,
    },
    HttpOne {
        #[arg(default_value_t = 8081)]
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
    /// Diff two profiles (PD set, template, channel manifest).
    Diff {
        profile_a: String,
        profile_b: String,
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
        } => {
            build::run(&root, &board, &build_dir, &config)?;
        }
        Commands::Test {
            board,
            build_dir,
            config,
        } => {
            build::image(&root, &board, &build_dir, &config)?;
            test::run_board_test(&root, &board, &build_dir, &config)?;
        }
        Commands::TestAll { build_dir, config } => {
            build::test_all(&root, &build_dir, &config)?;
        }
        Commands::TcpEcho { port } => tcp_echo::tcp_echo(port)?,
        Commands::HttpOne { port } => http_one::http_one(port)?,
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
                    let board_name = crate::profile::resolve_board_for_profile(
                        &profiles,
                        &name,
                        board.as_deref(),
                    )?;
                    build::image(&root, &board_name, &build_dir, &config)?;
                    println!("profile {name} -> board {board_name}: loader.img ready");
                }
                ProfileCommands::Diff {
                    profile_a,
                    profile_b,
                } => {
                    let pa = crate::profile::get_profile(&profiles, &profile_a)?;
                    let pb = crate::profile::get_profile(&profiles, &profile_b)?;
                    crate::profile::diff_profiles(&profile_a, pa, &profile_b, pb);
                }
            }
        }
    }

    Ok(())
}
