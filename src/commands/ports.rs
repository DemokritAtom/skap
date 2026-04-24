//! `skap ports list [--used|--free]` – inspect every port reserved by
//! skap and show whether it is currently bound on this machine.

use anyhow::Result;
use colored::Colorize;

use crate::cli::{PortsAction, PortsArgs, PortsListArgs};
use crate::config::ports::PortRegistry;
use crate::core::ports as port_core;
use crate::utils::output;

pub async fn run(args: PortsArgs) -> Result<()> {
    match args.action {
        PortsAction::List(a) => list(a).await,
    }
}

async fn list(args: PortsListArgs) -> Result<()> {
    let reg = PortRegistry::load()?;

    println!(
        "{:<35} {:<8} {}",
        "Service".bold(),
        "Port".bold(),
        "Status".bold()
    );
    println!("{}", "─".repeat(60).dimmed());

    if reg.ports.is_empty() {
        output::info("Keine Ports reserviert.");
        return Ok(());
    }

    let mut shown = 0usize;
    for (key, port) in &reg.ports {
        let in_use = !port_core::is_port_free(*port);
        if args.used && !in_use {
            continue;
        }
        if args.free && in_use {
            continue;
        }
        println!(
            "{:<35} {:<8} {}",
            key,
            port,
            output::port_status_symbol(in_use)
        );
        shown += 1;
    }
    if shown == 0 {
        output::info("Keine Einträge entsprechen dem Filter.");
    }
    Ok(())
}
