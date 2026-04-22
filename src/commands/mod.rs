//! Command dispatcher: maps parsed CLI input to the matching command module.
//!
//! Each command lives in its own submodule. In Step 1 they are stubs that
//! print a not-yet-implemented notice; subsequent steps fill them in.

use anyhow::Result;

use crate::cli::{Cli, Command};

pub mod add;
pub mod archive;
pub mod clean;
pub mod clone;
pub mod common;
pub mod config;
pub mod doctor;
pub mod fix;
pub mod list;
pub mod logs;
pub mod move_cmd;
pub mod new;
pub mod open;
pub mod ports;
pub mod rename;
pub mod restart;
pub mod run;
pub mod shell;
pub mod start;
pub mod status;
pub mod stop;
pub mod tag;
pub mod update;

/// Dispatch the parsed CLI to the corresponding command implementation.
pub async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::New(args) => new::run(args).await,
        Command::Add(args) => add::run(args).await,
        Command::Fix(args) => fix::run(args).await,
        Command::List(args) => list::run(args).await,
        Command::Status(args) => status::run(args).await,
        Command::Open(args) => open::run(args).await,
        Command::Start(args) => start::run(args).await,
        Command::Stop(args) => stop::run(args).await,
        Command::Restart(args) => restart::run(args).await,
        Command::Logs(args) => logs::run(args).await,
        Command::Shell(args) => shell::run(args).await,
        Command::Run(args) => run::run(args).await,
        Command::Doctor => doctor::run().await,
        Command::Clean(args) => clean::run(args).await,
        Command::Archive(args) => archive::run(args).await,
        Command::Config(args) => config::run(args).await,
        Command::Update => update::run().await,
        Command::Rename(args) => rename::run(args).await,
        Command::Move(args) => move_cmd::run(args).await,
        Command::Ports(args) => ports::run(args).await,
        Command::Clone(args) => clone::run(args).await,
        Command::Tag(args) => tag::run(args).await,
    }
}
