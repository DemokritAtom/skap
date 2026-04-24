//! Project registry (`~/.config/skap/registry.toml`).
//!
//! Tracks every project skap has created or adopted: its path, template,
//! creation time, tags, and which optional features are enabled.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::fs::write_atomic;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub projects: BTreeMap<String, ProjectEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub path: String,
    pub template: String,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub docker: bool,
    #[serde(default)]
    pub git: bool,
    #[serde(default)]
    pub git_remote: String,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub archived: bool,
}

impl Registry {
    pub fn path() -> Result<PathBuf> {
        Ok(super::config_dir()?.join("registry.toml"))
    }

    /// Load the registry, creating an empty one if missing.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            let r = Self::default();
            r.save()?;
            return Ok(r);
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let r: Self =
            toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(r)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let raw = toml::to_string_pretty(self).context("failed to serialize registry")?;
        write_atomic(&path, &raw)
    }

    /// Insert or replace a project entry.
    pub fn insert(&mut self, name: String, entry: ProjectEntry) {
        self.projects.insert(name, entry);
    }

    /// Remove a project from the registry.
    pub fn remove(&mut self, name: &str) -> Option<ProjectEntry> {
        self.projects.remove(name)
    }

    /// Locate a project by name.
    pub fn get(&self, name: &str) -> Option<&ProjectEntry> {
        self.projects.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ProjectEntry> {
        self.projects.get_mut(name)
    }

    /// Find the project whose path is, or contains, `cwd`. Used when the
    /// user omits the project name and we infer it from the working dir.
    pub fn find_by_path(&self, cwd: &Path) -> Option<(&String, &ProjectEntry)> {
        let cwd = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
        self.projects.iter().find(|(_, e)| {
            let p = std::fs::canonicalize(&e.path).unwrap_or_else(|_| PathBuf::from(&e.path));
            cwd == p || cwd.starts_with(&p)
        })
    }
}
