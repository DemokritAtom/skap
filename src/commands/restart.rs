//! `creo restart` – restart a project's docker compose stack.

use anyhow::Result;

use crate::cli::ProjectArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: ProjectArgs) -> Result<()> {
    let (name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::Path::new(&entry.path);
    if !docker::has_compose(dir) {
        output::error("no docker-compose.yml in project");
        return Ok(());
    }
    docker::compose(dir, &["restart"])?;
    output::success(&format!("{name} neu gestartet"));
    Ok(())
}
