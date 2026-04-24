//! `skap run <project> <cmd...>` – run a command in a project's context.
//!
//! If the project has a running compose stack with a single service, the
//! command is executed inside that container via `docker compose exec`.
//! Otherwise it runs in the project directory on the host – with an
//! explicit warning so the user understands which environment was used.

use anyhow::{bail, Result};

use crate::cli::RunArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: RunArgs) -> Result<()> {
    let (_name, entry) = resolve_project(Some(&args.project))?;
    if args.cmd.is_empty() {
        bail!("no command given");
    }
    let dir = std::path::PathBuf::from(&entry.path);

    if entry.docker && docker::has_compose(&dir) && docker::status(&dir) == docker::Status::Up {
        let services = docker::services(&dir).unwrap_or_default();
        if services.len() == 1 {
            let svc = &services[0];
            let mut argv: Vec<&str> = vec!["exec", svc];
            for a in &args.cmd {
                argv.push(a);
            }
            return docker::compose(&dir, &argv);
        }
    }

    // Fall back to host execution. Be loud about it when docker was
    // expected but unavailable so the user isn't surprised.
    if entry.docker {
        output::warn("Container läuft nicht – führe Befehl direkt im Projektverzeichnis aus.");
    }
    let status = std::process::Command::new(&args.cmd[0])
        .args(&args.cmd[1..])
        .current_dir(&dir)
        .status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
