//! Port registry (`~/.config/creo/ports.toml`).
//!
//! Tracks which ports creo has handed out so that subsequent
//! projects/services don't accidentally collide with each other.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::utils::fs::write_atomic;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortRegistry {
    #[serde(default)]
    pub ports: BTreeMap<String, u16>,
}

impl PortRegistry {
    pub fn path() -> Result<PathBuf> {
        Ok(super::config_dir()?.join("ports.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            let r = Self::default();
            r.save()?;
            return Ok(r);
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let r: Self = toml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(r)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let raw = toml::to_string_pretty(self).context("failed to serialize port registry")?;
        write_atomic(&path, &raw)
    }

    pub fn reserve(&mut self, key: impl Into<String>, port: u16) {
        self.ports.insert(key.into(), port);
    }

    pub fn release(&mut self, key: &str) {
        self.ports.remove(key);
    }

    /// All ports currently reserved for any project/service.
    pub fn reserved(&self) -> Vec<u16> {
        self.ports.values().copied().collect()
    }

    /// Drop every reservation belonging to the given project (prefix match
    /// `<project>-`). Used when a project is deleted or its ports are
    /// re-assigned.
    pub fn release_project(&mut self, project: &str) {
        let prefix = format!("{project}-");
        self.ports.retain(|k, _| !k.starts_with(&prefix));
    }
}
