//! `creo config` – read or modify global creo config.

use anyhow::Result;
use dialoguer::Confirm;

use crate::cli::{ConfigAction, ConfigArgs};
use crate::config::global::GlobalConfig;
use crate::utils::output;

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.action {
        ConfigAction::Set { key, value } => {
            let mut cfg = GlobalConfig::load()?;
            cfg.set(&key, &value)?;
            cfg.save()?;
            output::success(&format!("set {key} = {value}"));
        }
        ConfigAction::Get { key } => {
            let cfg = GlobalConfig::load()?;
            let value = cfg.get(&key)?;
            println!("{value}");
        }
        ConfigAction::List => {
            let cfg = GlobalConfig::load()?;
            for (k, v) in cfg.entries() {
                println!("{k} = {v}");
            }
        }
        ConfigAction::Init { force } => init(force)?,
    }
    Ok(())
}

fn init(force: bool) -> Result<()> {
    let path = GlobalConfig::path()?;
    if path.exists() && !force {
        let proceed = Confirm::new()
            .with_prompt(format!(
                "{} existiert bereits. Mit Defaults überschreiben?",
                path.display()
            ))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !proceed {
            output::info("Abgebrochen – config.toml unverändert.");
            return Ok(());
        }
    }
    let cfg = GlobalConfig::default();
    cfg.save()?;
    output::success(&format!("config.toml geschrieben: {}", path.display()));
    output::info("registry.toml und ports.toml wurden NICHT verändert.");
    Ok(())
}
