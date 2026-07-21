//! `skap rename <old> <new>` – rename a project everywhere.
//!
//! Renames the on-disk directory, the registry entry, every port
//! reservation belonging to the project, and the `container_name`
//! entries in its `docker-compose.yml`. If the project's containers
//! were running, they are stopped before the rename and restarted
//! afterwards.

use anyhow::{bail, Context, Result};

use crate::cli::RenameArgs;
use crate::commands::common::validate_project_name;
use crate::config::ports::PortRegistry;
use crate::config::registry::Registry;
use crate::core::docker;
use crate::core::project_file::ProjectFile;
use crate::utils::output;

pub async fn run(args: RenameArgs) -> Result<()> {
    validate_project_name(&args.new_name)?;
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
    crate::utils::fs::move_dir(&old_path, &new_path)?;
    output::success("Ordner umbenannt");

    // Update registry: remove old, insert new with patched path.
    let mut new_entry = entry.clone();
    new_entry.path = new_path.to_string_lossy().into_owned();
    registry.remove(&args.old_name);
    registry.insert(args.new_name.clone(), new_entry);
    registry.save()?;
    output::success("Registry aktualisiert");

    // Update port registry: move every reservation from old_name to new_name.
    port_reg.rename_project(&args.old_name, &args.new_name);
    port_reg.save()?;
    output::success("Ports aktualisiert");

    // Keep container names in the compose file in sync with the new
    // project name (they were rendered as literal `<old_name>-<service>`
    // strings at creation time and don't update themselves).
    let compose_path = new_path.join("docker-compose.yml");
    if let Ok(raw) = std::fs::read_to_string(&compose_path) {
        let old_prefix = format!("container_name: {}-", args.old_name);
        let new_prefix = format!("container_name: {}-", args.new_name);
        if raw.contains(&old_prefix) {
            let updated = raw.replace(&old_prefix, &new_prefix);
            if crate::utils::fs::write_atomic(&compose_path, &updated).is_ok() {
                output::success("Container-Namen in docker-compose.yml aktualisiert");
            }
        }
    }

    // Update .skap.toml inside the project.
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
