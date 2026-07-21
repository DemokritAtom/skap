//! `skap clone <url> [name]` – clone a git repo and register it as a
//! skap project. Honours `.skap.toml` if present, otherwise falls back
//! to stack autodetection.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::cli::CloneArgs;
use crate::commands::common::validate_project_name;
use crate::config::ports::PortRegistry;
use crate::config::registry::{ProjectEntry, Registry};
use crate::core::{detect, project_file::ProjectFile};
use crate::utils::output;

pub async fn run(args: CloneArgs) -> Result<()> {
    // Derive the target folder name.
    let derived_name = args.name.clone().unwrap_or_else(|| guess_name(&args.url));
    if derived_name.is_empty() {
        bail!("could not derive a project name from URL '{}'", args.url);
    }
    validate_project_name(&derived_name).with_context(|| {
        "could not derive a valid project name from the URL – pass one explicitly: `skap clone <url> <name>`"
    })?;
    let target = std::env::current_dir()?.join(&derived_name);
    if target.exists() {
        bail!("target directory already exists: {}", target.display());
    }

    // Refuse silent overwrites of existing registry entries; the user
    // must pick a different name explicitly.
    {
        let registry = Registry::load()?;
        if registry.get(&derived_name).is_some() {
            bail!(
                "a project named '{derived_name}' is already registered – pass a different name: `skap clone {} <name>`",
                args.url
            );
        }
    }

    output::step(&format!("git clone {} {}", args.url, derived_name));
    let status = Command::new("git")
        .args(["clone", &args.url, &derived_name])
        .status()
        .context("failed to invoke git")?;
    if !status.success() {
        bail!("git clone failed");
    }
    output::success("Repo geklont");

    let mut registry = Registry::load()?;
    let mut port_reg = PortRegistry::load()?;

    let (template, ports_map): (String, BTreeMap<String, u16>) =
        if let Some(pf) = ProjectFile::load(&target)? {
            output::info(&format!(
                ".skap.toml gefunden (template = {})",
                pf.project.template
            ));
            (pf.project.template, pf.ports)
        } else {
            // Fallback: detect.
            let det = detect::detect_stack(&target).unwrap_or("docker-only");
            output::info(&format!("Kein .skap.toml – erkannter Stack: {det}"));

            // Try to parse host ports from compose file.
            let mut map = BTreeMap::new();
            let compose_path = target.join("docker-compose.yml");
            if let Ok(raw) = std::fs::read_to_string(&compose_path) {
                for (i, p) in detect::parse_compose_ports(&raw).into_iter().enumerate() {
                    map.insert(format!("port{}", i + 1), p);
                }
            }
            (det.to_string(), map)
        };

    // Register every port from .skap.toml in the global port registry.
    for (k, v) in &ports_map {
        port_reg.reserve(&derived_name, k, *v);
    }

    let entry = ProjectEntry {
        path: target.to_string_lossy().into_owned(),
        template: template.clone(),
        created: Utc::now(),
        tags: Vec::new(),
        docker: target.join("docker-compose.yml").exists(),
        git: true,
        git_remote: args.url.clone(),
        ports: {
            let mut v: Vec<u16> = ports_map.values().copied().collect();
            v.sort();
            v
        },
        archived: false,
    };
    registry.insert(derived_name.clone(), entry);
    registry.save()?;
    port_reg.save()?;
    output::success(&format!(
        "Registriert: {derived_name} (template: {template})"
    ));
    output::info(&format!("Starten mit: skap start {derived_name}"));
    Ok(())
}

fn guess_name(url: &str) -> String {
    let last = url.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    let stripped = last.strip_suffix(".git").unwrap_or(last);
    stripped.to_string()
}

#[allow(dead_code)]
fn _silence(_: &PathBuf) {}
