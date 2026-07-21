//! Port registry (`~/.config/skap/ports.toml`).
//!
//! Tracks which ports skap has handed out so that subsequent
//! projects/services don't accidentally collide with each other.
//!
//! Ports are stored per project (`project -> service -> port`) rather
//! than as a single flat `"<project>-<service>"` string key. A flat key
//! is ambiguous under prefix matching: deleting/renaming project "api"
//! would also match reservations belonging to an unrelated project
//! named "api-gateway". Keying by the exact project name removes that
//! ambiguity entirely.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::utils::fs::write_atomic;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortRegistry {
    #[serde(default)]
    pub ports: BTreeMap<String, BTreeMap<String, u16>>,
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
        let r: Self =
            toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(r)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let raw = toml::to_string_pretty(self).context("failed to serialize port registry")?;
        write_atomic(&path, &raw)
    }

    /// Reserve `port` for `service` under `project`.
    pub fn reserve(&mut self, project: &str, service: &str, port: u16) {
        self.ports
            .entry(project.to_string())
            .or_default()
            .insert(service.to_string(), port);
    }

    /// Release a single service's reservation.
    #[allow(dead_code)]
    pub fn release(&mut self, project: &str, service: &str) {
        if let Some(services) = self.ports.get_mut(project) {
            services.remove(service);
            if services.is_empty() {
                self.ports.remove(project);
            }
        }
    }

    /// All ports currently reserved for any project/service.
    pub fn reserved(&self) -> Vec<u16> {
        self.ports
            .values()
            .flat_map(|m| m.values().copied())
            .collect()
    }

    /// Drop every reservation belonging to the given project. Used when a
    /// project is deleted or its ports are re-assigned.
    pub fn release_project(&mut self, project: &str) {
        self.ports.remove(project);
    }

    /// Move every reservation from `old` to `new` (used by `skap rename`).
    pub fn rename_project(&mut self, old: &str, new: &str) {
        if let Some(services) = self.ports.remove(old) {
            self.ports.insert(new.to_string(), services);
        }
    }

    /// Flattened `(display_key, port)` pairs for listing, sorted by
    /// project then service.
    pub fn entries(&self) -> Vec<(String, u16)> {
        self.ports
            .iter()
            .flat_map(|(project, services)| {
                services
                    .iter()
                    .map(move |(service, port)| (format!("{project}-{service}"), *port))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test: deleting/releasing a project whose name is a
    /// string-prefix of another project's name must not touch the other
    /// project's reservations. This used to break when reservations were
    /// keyed by a flat `"<project>-<service>"` string and matched via
    /// `starts_with("<project>-")`.
    #[test]
    fn release_project_does_not_affect_prefix_colliding_project() {
        let mut reg = PortRegistry::default();
        reg.reserve("api", "app", 3000);
        reg.reserve("api-gateway", "app", 3001);

        reg.release_project("api");

        assert_eq!(reg.reserved(), vec![3001]);
        assert!(reg.ports.contains_key("api-gateway"));
        assert!(!reg.ports.contains_key("api"));
    }

    #[test]
    fn rename_project_does_not_affect_prefix_colliding_project() {
        let mut reg = PortRegistry::default();
        reg.reserve("api", "app", 3000);
        reg.reserve("api-gateway", "app", 3001);

        reg.rename_project("api", "svc");

        assert!(!reg.ports.contains_key("api"));
        assert_eq!(*reg.ports.get("svc").unwrap().get("app").unwrap(), 3000);
        assert_eq!(
            *reg.ports.get("api-gateway").unwrap().get("app").unwrap(),
            3001
        );
    }

    #[test]
    fn release_single_service_drops_empty_project() {
        let mut reg = PortRegistry::default();
        reg.reserve("proj", "app", 3000);
        reg.release("proj", "app");
        assert!(!reg.ports.contains_key("proj"));
    }

    #[test]
    fn entries_are_flattened_and_prefixed() {
        let mut reg = PortRegistry::default();
        reg.reserve("proj", "app", 3000);
        reg.reserve("proj", "db", 5432);
        let mut entries = reg.entries();
        entries.sort();
        assert_eq!(
            entries,
            vec![
                ("proj-app".to_string(), 3000),
                ("proj-db".to_string(), 5432),
            ]
        );
    }
}
