//! `skap start` – start a project's docker compose stack.

use anyhow::Result;

use crate::cli::ProjectArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: ProjectArgs) -> Result<()> {
    let (name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::Path::new(&entry.path);
    if !docker::has_compose(dir) {
        output::error("no docker-compose.yml in project. Try: skap add docker");
        return Ok(());
    }
    output::info(&format!("Starting {name}…"));
    docker::compose(dir, &["up", "-d"])?;
    output::success(&format!("{name} läuft"));
    Ok(())
}
