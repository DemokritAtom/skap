//! `skap shell` – open a shell in a running container.

use anyhow::{bail, Result};
use dialoguer::{Confirm, Select};

use crate::cli::ShellArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;
use crate::utils::output;

pub async fn run(args: ShellArgs) -> Result<()> {
    let (name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::Path::new(&entry.path);
    if !docker::has_compose(dir) {
        bail!("project '{name}' has no docker-compose.yml");
    }

    // If no container is running, offer to start the stack first.
    if docker::status(dir) != docker::Status::Up {
        output::warn(&format!("Container von '{name}' laufen aktuell nicht."));
        let start = Confirm::new()
            .with_prompt("Jetzt starten?")
            .default(true)
            .interact()
            .unwrap_or(false);
        if !start {
            output::info("Abgebrochen.");
            return Ok(());
        }
        output::step("docker compose up -d …");
        docker::compose(dir, &["up", "-d"])?;
    }

    let services = docker::services(dir)?;
    if services.is_empty() {
        bail!("no services declared in docker-compose.yml");
    }
    let svc = match args.service {
        Some(s) => s,
        None if services.len() == 1 => services[0].clone(),
        None => {
            let pick = Select::new()
                .with_prompt("Service")
                .items(&services)
                .default(0)
                .interact()?;
            services[pick].clone()
        }
    };
    // Prefer bash, fall back to sh.
    docker::compose(
        dir,
        &[
            "exec",
            &svc,
            "sh",
            "-c",
            "command -v bash >/dev/null && exec bash || exec sh",
        ],
    )?;
    Ok(())
}
