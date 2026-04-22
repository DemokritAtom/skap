//! `creo clean` – remove docker artifacts of a project.

use anyhow::Result;

use crate::cli::CleanArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: CleanArgs) -> Result<()> {
    let (name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::PathBuf::from(&entry.path);
    let images = args.images || args.all;
    let volumes = args.volumes || args.all;

    if !images && !volumes {
        output::warn("nothing to do (use --images, --volumes or --all)");
        return Ok(());
    }
    let mut argv: Vec<&str> = vec!["down"];
    if images {
        argv.push("--rmi");
        argv.push("local");
    }
    if volumes {
        argv.push("-v");
    }
    docker::compose(&dir, &argv)?;
    output::success(&format!("{name} cleaned"));
    Ok(())
}
