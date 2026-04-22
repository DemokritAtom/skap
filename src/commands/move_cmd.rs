//! `creo move <project> <new-path>` – move a project to a different
//! filesystem location and update the registry accordingly.

use anyhow::{bail, Context, Result};

use crate::cli::MoveArgs;
use crate::config::registry::Registry;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: MoveArgs) -> Result<()> {
    let mut registry = Registry::load()?;
    let entry = registry
        .get(&args.project)
        .cloned()
        .with_context(|| format!("project '{}' is not registered", args.project))?;

    let old_path = std::path::PathBuf::from(&entry.path);
    let new_path = std::path::PathBuf::from(&args.new_path);
    if new_path.exists() {
        bail!("target path already exists: {}", new_path.display());
    }
    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let was_running = entry.docker
        && old_path.exists()
        && docker::has_compose(&old_path)
        && docker::status(&old_path) == docker::Status::Up;
    if was_running {
        output::step("Container stoppen…");
        let _ = docker::compose(&old_path, &["down"]);
    }

    std::fs::rename(&old_path, &new_path).with_context(|| {
        format!(
            "failed to move {} -> {}",
            old_path.display(),
            new_path.display()
        )
    })?;
    output::success(&format!("Verschoben nach {}", new_path.display()));

    if let Some(e) = registry.get_mut(&args.project) {
        e.path = new_path.to_string_lossy().into_owned();
    }
    registry.save()?;
    output::success("Registry aktualisiert");

    if was_running {
        let _ = docker::compose(&new_path, &["up", "-d"]);
    }
    output::info(&format!("Starten mit: creo start {}", args.project));
    Ok(())
}
