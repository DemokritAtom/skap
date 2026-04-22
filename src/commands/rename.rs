//! `creo rename <old> <new>` – rename a project everywhere.
//!
//! Renames the on-disk directory, the registry key + path, and every
//! key in the port registry that starts with `<old>-`. If the project's
//! containers were running, they are stopped before the rename and
//! restarted afterwards.

use anyhow::{bail, Context, Result};

use crate::cli::RenameArgs;
use crate::config::ports::PortRegistry;
use crate::config::registry::Registry;
use crate::core::docker;
use crate::core::project_file::ProjectFile;
use crate::utils::output;

pub async fn run(args: RenameArgs) -> Result<()> {
    let mut registry = Registry::load()?;
    let mut port_reg = PortRegistry::load()?;

    let entry = registry
        .get(&args.old_name)
        .cloned()
        .with_context(|| format!("project '{}' is not registered", args.old_name))?;
    if registry.get(&args.new_name).is_some() {
        bail!("a project named '{}' already exists", args.new_name);
    }

    let old_path = std::path::PathBuf::from(&entry.path);
    let new_path = old_path
        .parent()
        .map(|p| p.join(&args.new_name))
        .unwrap_or_else(|| std::path::PathBuf::from(&args.new_name));
    if new_path.exists() {
        bail!("target path already exists: {}", new_path.display());
    }

    // Stop running stack first.
    let was_running = entry.docker
        && old_path.exists()
        && docker::has_compose(&old_path)
        && docker::status(&old_path) == docker::Status::Up;
    if was_running {
        output::step("Container stoppen…");
        let _ = docker::compose(&old_path, &["down"]);
    }

    // Move the directory.
    std::fs::rename(&old_path, &new_path).with_context(|| {
        format!(
            "failed to rename {} -> {}",
            old_path.display(),
            new_path.display()
        )
    })?;
    output::success("Ordner umbenannt");

    // Update registry: remove old, insert new with patched path.
    let mut new_entry = entry.clone();
    new_entry.path = new_path.to_string_lossy().into_owned();
    registry.remove(&args.old_name);
    registry.insert(args.new_name.clone(), new_entry);
    registry.save()?;
    output::success("Registry aktualisiert");

    // Update port registry keys.
    let prefix_old = format!("{}-", args.old_name);
    let prefix_new = format!("{}-", args.new_name);
    let to_rename: Vec<(String, u16)> = port_reg
        .ports
        .iter()
        .filter(|(k, _)| k.starts_with(&prefix_old))
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    for (k, _) in &to_rename {
        port_reg.release(k);
    }
    for (k, v) in &to_rename {
        let renamed = k.replacen(&prefix_old, &prefix_new, 1);
        port_reg.reserve(renamed, *v);
    }
    port_reg.save()?;
    output::success("Ports aktualisiert");

    // Update .creo.toml inside the project.
    if let Ok(Some(mut pf)) = ProjectFile::load(&new_path) {
        pf.project.name = args.new_name.clone();
        pf.save(&new_path).ok();
    }

    // Restart containers if they were up.
    if was_running {
        let _ = docker::compose(&new_path, &["up", "-d"]);
    }

    output::success(&format!(
        "Projekt \"{}\" → \"{}\"",
        args.old_name, args.new_name
    ));
    Ok(())
}
