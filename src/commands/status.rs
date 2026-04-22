//! `creo status` – show details of a single project.

use anyhow::Result;
use colored::Colorize;

use crate::cli::StatusArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: StatusArgs) -> Result<()> {
    let (name, e) = resolve_project(args.project.as_deref())?;
    let dir = std::path::Path::new(&e.path);

    println!("{:<11} {}", "Projekt:".bold(), name);
    println!("{:<11} {}", "Pfad:".bold(), e.path);
    println!("{:<11} {}", "Template:".bold(), e.template);
    println!(
        "{:<11} {}",
        "Erstellt:".bold(),
        e.created.format("%Y-%m-%d")
    );
    println!();

    // Git
    let git_str = if e.git {
        match git_branch(dir) {
            Some(branch) => {
                let dirty = if git_is_dirty(dir) { "dirty" } else { "sauber" };
                let remote = if e.git_remote.is_empty() {
                    "(no remote)".to_string()
                } else {
                    format!("remote: {}", e.git_remote)
                };
                format!("✓ {dirty}  (branch: {branch}, {remote})")
            }
            None => "✓ initialized".into(),
        }
    } else {
        "─ no git".into()
    };
    println!("{:<11} {}", "Git:".bold(), git_str);

    // Docker
    let dstr = if e.docker {
        output::status_symbol(docker::status(dir))
    } else {
        "─ no docker".into()
    };
    println!("{:<11} {}", "Docker:".bold(), dstr);

    // Ports
    let pstr = if e.ports.is_empty() {
        "─".into()
    } else {
        e.ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("{:<11} {}", "Ports:".bold(), pstr);

    // Tags
    let tstr = if e.tags.is_empty() {
        "─".into()
    } else {
        e.tags.join(", ")
    };
    println!("{:<11} {}", "Tags:".bold(), tstr);

    // Disk usage
    if let Some(size) = dir_size(dir) {
        println!();
        println!("{:<11} {}", "Disk Usage:".bold(), human_bytes(size));
    }

    Ok(())
}

fn git_branch(path: &std::path::Path) -> Option<String> {
    let repo = git2::Repository::open(path).ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(|s| s.to_string())
}

fn git_is_dirty(path: &std::path::Path) -> bool {
    let Ok(repo) = git2::Repository::open(path) else { return false };
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    !statuses.is_empty()
}

fn dir_size(path: &std::path::Path) -> Option<u64> {
    fn walk(p: &std::path::Path) -> u64 {
        let Ok(rd) = std::fs::read_dir(p) else { return 0 };
        let mut total = 0u64;
        for entry in rd.flatten() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() {
                total += walk(&entry.path());
            } else {
                total += meta.len();
            }
        }
        total
    }
    if path.exists() {
        Some(walk(path))
    } else {
        None
    }
}

fn human_bytes(b: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = b as f64;
    let mut idx = 0;
    while size >= 1024.0 && idx < UNITS.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }
    format!("{:.0} {}", size, UNITS[idx])
}
