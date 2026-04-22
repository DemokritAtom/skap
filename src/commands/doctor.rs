//! `creo doctor` – system & project diagnosis.

use std::collections::HashMap;
use std::process::Command;

use anyhow::Result;
use colored::Colorize;

use crate::config::ports::PortRegistry;
use crate::config::registry::Registry;
use crate::core::{docker, ports as port_core};

pub async fn run() -> Result<()> {
    section("SYSTEM");
    let tools = [
        ("Docker", "docker", &["--version"][..]),
        ("Git", "git", &["--version"]),
        ("Node", "node", &["--version"]),
        ("Python", "python3", &["--version"]),
        ("Go", "go", &["version"]),
        ("Rust", "rustc", &["--version"]),
    ];
    for (label, bin, args) in tools.iter() {
        match Command::new(bin).args(*args).output() {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                println!("  {}  {:<10} {}", "✓".green(), label, v.dimmed());
            }
            _ => println!("  {}  {:<10} not installed", "✗".red(), label),
        }
    }

    let registry = Registry::load()?;
    let port_reg = PortRegistry::load()?;
    let mut recommendations: Vec<String> = Vec::new();

    println!();
    section("PROJEKTE");
    if registry.projects.is_empty() {
        println!("  {}", "(none)".dimmed());
    }
    for (name, e) in &registry.projects {
        let path = std::path::Path::new(&e.path);
        if !path.exists() {
            println!("  {}  {:<15} path missing ({})", "✗".red(), name, e.path);
            recommendations.push(format!("creo archive {name}  (path missing)"));
            continue;
        }
        if e.docker {
            match docker::status(path) {
                docker::Status::Up => println!("  {}  {:<15} alles ok", "✓".green(), name),
                docker::Status::Down => println!("  {}  {:<15} stopped", "·".dimmed(), name),
                docker::Status::Error => {
                    println!("  {}  {:<15} compose error", "⚠".yellow(), name);
                    recommendations.push(format!("creo fix docker {name}"));
                }
                docker::Status::Unknown => println!("  {}  {:<15} no docker", "·".dimmed(), name),
            }
        } else {
            println!("  {}  {:<15} (no docker)", "·".dimmed(), name);
        }
    }

    // Port conflicts: find any reserved port that is also bound by another
    // process not started by creo's compose stacks.
    println!();
    section("PORT-KONFLIKTE");
    let mut by_port: HashMap<u16, Vec<String>> = HashMap::new();
    for (key, port) in &port_reg.ports {
        by_port.entry(*port).or_default().push(key.clone());
    }
    let mut had_conflict = false;
    for (port, owners) in &by_port {
        if owners.len() > 1 {
            had_conflict = true;
            println!(
                "  {}  Port {}  shared by: {}",
                "⚠".yellow(),
                port,
                owners.join(", ")
            );
        } else if !port_core::is_port_free(*port) {
            // Reserved by us but bound by something we can't see (could be
            // our own running container, which is fine).
            // Heuristic: if no project owns that port via docker status,
            // it's worth noting.
            // Skip noisy output here.
        }
    }
    if !had_conflict {
        println!("  {}", "keine".dimmed());
    }

    if !recommendations.is_empty() {
        println!();
        section("EMPFEHLUNGEN");
        for r in &recommendations {
            println!("  → {r}");
        }
    }
    Ok(())
}

fn section(title: &str) {
    println!("{}", title.bold());
}
