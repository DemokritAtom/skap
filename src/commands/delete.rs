//! `skap delete` – remove a project and all traces from registry and ports.

use crate::cli::DeleteArgs;
use crate::commands::common::resolve_project;
use crate::config::ports::PortRegistry;
use crate::config::registry::Registry;
use crate::core::docker;
use crate::utils::output;
use anyhow::{Context, Result};
use dialoguer::Select;

pub async fn run(args: DeleteArgs) -> Result<()> {
    let (name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::PathBuf::from(&entry.path);
    let mut reg = Registry::load()?;
    let mut port_reg = PortRegistry::load()?;

    // Stop running containers if dockerized
    if entry.docker && dir.exists() && docker::has_compose(&dir) {
        let _ = docker::compose(&dir, &["down"]);
    }

    // If not --yes, ask for confirmation
    if !args.yes && !args.keep_files {
        println!("\n⚠ Projekt \"{}\" löschen?", name);
        println!("  Pfad:   {}", entry.path);
        println!(
            "  Ports:  {}",
            entry
                .ports
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        let options = vec![
            "Alles löschen (Ordner + Registry + Ports)",
            "Nur aus Registry entfernen (Ordner bleibt)",
            "Abbrechen",
        ];
        let sel = Select::new()
            .with_prompt("Wähle eine Option:")
            .items(&options)
            .default(0)
            .interact()?;
        match sel {
            0 => {} // proceed
            1 => return remove_registry_only(name, &mut reg, &mut port_reg),
            _ => {
                output::info("Abgebrochen");
                return Ok(());
            }
        }
    }

    // Remove from registry and ports
    reg.remove(&name);
    port_reg.release_project(&name);
    reg.save()?;
    port_reg.save()?;
    output::success(&format!("{} aus Registry entfernt", name));

    // Remove project folder unless --keep-files
    if !args.keep_files && dir.exists() {
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("Ordner konnte nicht gelöscht werden: {}", dir.display()))?;
        output::success(&format!("Ordner {} gelöscht", dir.display()));
    }
    Ok(())
}

fn remove_registry_only(
    name: String,
    reg: &mut Registry,
    port_reg: &mut PortRegistry,
) -> Result<()> {
    reg.remove(&name);
    port_reg.release_project(&name);
    reg.save()?;
    port_reg.save()?;
    output::success(&format!("{} nur aus Registry entfernt", name));
    Ok(())
}
