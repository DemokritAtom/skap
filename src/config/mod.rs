//! Configuration sub-system: global config, project registry, port registry.
//!
//! All configuration files live under `~/.config/creo/` and are lazily
//! created on first read with sane defaults.

pub mod global;
pub mod ports;
pub mod registry;

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Returns the directory holding all creo configuration files
/// (`~/.config/creo` on Linux). Creates it if missing.
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine user config directory")?;
    let dir = base.join("creo");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("could not create config directory {}", dir.display()))?;
    Ok(dir)
}
