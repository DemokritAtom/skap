//! `creo archive` – mark a project as archived (hide & stop).

use anyhow::Result;

use crate::cli::ProjectArgs;
use crate::commands::common::resolve_project;
use crate::config::registry::Registry;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: ProjectArgs) -> Result<()> {
    let (name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::PathBuf::from(&entry.path);
    if entry.docker && dir.exists() && docker::has_compose(&dir) {
        let _ = docker::compose(&dir, &["down"]);
    }
    let mut reg = Registry::load()?;
    if let Some(e) = reg.get_mut(&name) {
        e.archived = true;
    }
    reg.save()?;
    output::success(&format!("{name} archiviert"));
    Ok(())
}
