//! Global skap configuration (`~/.config/skap/config.toml`).
//!
//! Lazily created on first read with sane defaults.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::utils::fs::write_atomic;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub ports: PortsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub editor: String,
    pub git: bool,
    pub docker: bool,
    pub license: String,
    pub git_provider: String,
    pub github_token: String,
    pub gitlab_token: String,
    pub gitlab_url: String,
    #[serde(default = "default_true")]
    pub emoji: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            editor: "code".into(),
            git: true,
            docker: true,
            license: "MIT".into(),
            git_provider: "github".into(),
            github_token: String::new(),
            gitlab_token: String::new(),
            gitlab_url: "https://gitlab.com".into(),
            emoji: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortsSection {
    pub base_port: u16,
}

impl Default for PortsSection {
    fn default() -> Self {
        Self { base_port: 3000 }
    }
}

impl GlobalConfig {
    /// Path to the global config file.
    pub fn path() -> Result<PathBuf> {
        Ok(super::config_dir()?.join("config.toml"))
    }

    /// Load the config, creating it with defaults if it does not exist.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let cfg: Self =
            toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(cfg)
    }

    /// Persist the config to disk atomically.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        let raw = toml::to_string_pretty(self).context("failed to serialize global config")?;
        write_atomic(&path, &raw)
    }

    /// Set a config value by dotted key, e.g. `editor`, `defaults.editor`,
    /// `ports.base_port`. Returns an error for unknown keys.
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        // Allow both `editor` and `defaults.editor` style.
        let key = key.strip_prefix("defaults.").unwrap_or(key);
        match key {
            "editor" => self.defaults.editor = value.into(),
            "git" => self.defaults.git = parse_bool(value)?,
            "docker" => self.defaults.docker = parse_bool(value)?,
            "license" | "default_license" => self.defaults.license = value.into(),
            "git_provider" => self.defaults.git_provider = value.into(),
            "github_token" => self.defaults.github_token = value.into(),
            "gitlab_token" => self.defaults.gitlab_token = value.into(),
            "gitlab_url" => self.defaults.gitlab_url = value.into(),
            "emoji" => self.defaults.emoji = parse_bool(value)?,
            "base_port" | "ports.base_port" => {
                self.ports.base_port = value
                    .parse()
                    .with_context(|| format!("invalid port: {value}"))?;
            }
            other => anyhow::bail!("unknown config key: {other}"),
        }
        Ok(())
    }

    /// Get a config value by dotted key.
    pub fn get(&self, key: &str) -> Result<String> {
        let key = key.strip_prefix("defaults.").unwrap_or(key);
        Ok(match key {
            "editor" => self.defaults.editor.clone(),
            "git" => self.defaults.git.to_string(),
            "docker" => self.defaults.docker.to_string(),
            "license" | "default_license" => self.defaults.license.clone(),
            "git_provider" => self.defaults.git_provider.clone(),
            "github_token" => self.defaults.github_token.clone(),
            "gitlab_token" => self.defaults.gitlab_token.clone(),
            "gitlab_url" => self.defaults.gitlab_url.clone(),
            "emoji" => self.defaults.emoji.to_string(),
            "base_port" | "ports.base_port" => self.ports.base_port.to_string(),
            other => anyhow::bail!("unknown config key: {other}"),
        })
    }

    /// All key/value pairs for `skap config list`.
    pub fn entries(&self) -> Vec<(&'static str, String)> {
        vec![
            ("editor", self.defaults.editor.clone()),
            ("git", self.defaults.git.to_string()),
            ("docker", self.defaults.docker.to_string()),
            ("license", self.defaults.license.clone()),
            ("git_provider", self.defaults.git_provider.clone()),
            ("github_token", redact(&self.defaults.github_token)),
            ("gitlab_token", redact(&self.defaults.gitlab_token)),
            ("gitlab_url", self.defaults.gitlab_url.clone()),
            ("emoji", self.defaults.emoji.to_string()),
            ("base_port", self.ports.base_port.to_string()),
        ]
    }
}

fn parse_bool(s: &str) -> Result<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        other => anyhow::bail!("expected boolean, got: {other}"),
    }
}

fn redact(secret: &str) -> String {
    if secret.is_empty() {
        String::new()
    } else if secret.len() <= 6 {
        "***".into()
    } else {
        format!("***{}", &secret[secret.len() - 4..])
    }
}
