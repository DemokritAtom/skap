//! skap – a lean, fast Linux CLI for managing dev projects.
//!
//! Entry point: parses the CLI arguments and dispatches to the matching
//! command module. All real logic lives in `commands::*`.

use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod config;
mod core;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    // Initialize emoji mode based on the CLI flag and the persisted config
    // (best-effort – if the config can't be read yet, fall back to defaults).
    let cfg_emoji = config::global::GlobalConfig::load()
        .map(|c| c.defaults.emoji)
        .unwrap_or(true);
    utils::output::init_emoji(cli.no_emoji, cfg_emoji);
    commands::dispatch(cli).await
}
