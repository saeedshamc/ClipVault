//! ClipVault daemon entry point and CLI client.

mod app;
mod ipc;
mod service;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use clipvault_core::ipc::{Command, Request, Response, ResponseBody, ResponseResult};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "clipvault-daemon",
    version,
    about = "ClipVault background daemon and CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run the always-on daemon (default when no subcommand is given).
    Run,
    /// List recent clipboard items.
    List {
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Full-text search.
    Search {
        query: String,
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Delete one item by id.
    Delete { id: i64 },
    /// Clear the entire history.
    Clear,
    /// Pause clipboard capture.
    Pause,
    /// Resume clipboard capture.
    Resume,
    /// Show daemon status.
    Status,
    /// Ping the daemon.
    Ping,
}

fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => service::run_daemon(),
        Commands::List { limit, offset } => {
            print_response(client_cmd(Command::List { limit, offset })?)
        }
        Commands::Search { query, limit } => {
            print_response(client_cmd(Command::Search { query, limit })?)
        }
        Commands::Delete { id } => print_response(client_cmd(Command::Delete { id })?),
        Commands::Clear => print_response(client_cmd(Command::Clear)?),
        Commands::Pause => print_response(client_cmd(Command::Pause)?),
        Commands::Resume => print_response(client_cmd(Command::Resume)?),
        Commands::Status => print_response(client_cmd(Command::Status)?),
        Commands::Ping => print_response(client_cmd(Command::Ping)?),
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn client_cmd(cmd: Command) -> Result<Response> {
    let req = Request::new(1, cmd);
    ipc::call(&req).context("IPC request failed — is clipvault-daemon running?")
}

fn print_response(resp: Response) -> Result<()> {
    match resp.body {
        ResponseBody::Err { error } => bail!("{error}"),
        ResponseBody::Ok { result } => {
            match result {
                ResponseResult::Pong { version } => {
                    println!("pong clipvault-daemon {version}");
                }
                ResponseResult::Items { items } => {
                    for item in items {
                        println!(
                            "{}\t{}\t{}\t{}",
                            item.id,
                            item.kind.as_str(),
                            item.total_size,
                            item.searchable_text.replace('\n', " ")
                        );
                    }
                }
                ResponseResult::Cleared { deleted } => println!("cleared {deleted} items"),
                ResponseResult::Deleted { id } => println!("deleted {id}"),
                ResponseResult::Status {
                    paused,
                    item_count,
                    total_bytes,
                } => {
                    println!("paused={paused} items={item_count} total_bytes={total_bytes}");
                }
                ResponseResult::Empty => println!("ok"),
            }
            Ok(())
        }
    }
}
