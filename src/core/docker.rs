//! Docker / `docker compose` integration.
//!
//! We shell out to the `docker` CLI rather than talking to the daemon
//! directly. This keeps the binary small, avoids permission issues with
//! `/var/run/docker.sock`, and matches what users expect when reading
//! skap's output.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

/// Container status as reported by `docker compose ps`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// At least one service is running.
    Up,
    /// Services exist but none are running.
    Down,
    /// `docker compose ps` returned an error.
    Error,
    /// No compose file in the project (or docker not installed).
    Unknown,
}

/// Returns true if the `docker` binary is on PATH.
pub fn is_installed() -> bool {
    which::which("docker").is_ok()
}

/// True iff the project directory has a docker compose configuration.
pub fn has_compose(project_dir: &Path) -> bool {
    ["docker-compose.yml", "docker-compose.yaml", "compose.yml"]
        .iter()
        .any(|f| project_dir.join(f).exists())
}

/// Probe the compose stack status.
pub fn status(project_dir: &Path) -> Status {
    if !has_compose(project_dir) || !is_installed() {
        return Status::Unknown;
    }
    let out = Command::new("docker")
        .args(["compose", "ps", "--format", "json"])
        .current_dir(project_dir)
        .stderr(Stdio::null())
        .output();
    let Ok(out) = out else { return Status::Error };
    if !out.status.success() {
        return Status::Error;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Newer compose emits one JSON object per line; older emits a JSON array.
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Status::Down;
    }
    let running = trimmed
        .lines()
        .any(|l| l.contains("\"State\":\"running\"") || l.contains("\"State\": \"running\""));
    if running {
        Status::Up
    } else {
        Status::Down
    }
}

/// Run `docker compose <args>` inside `project_dir` and inherit stdio so
/// the user sees compose's progress directly.
pub fn compose(project_dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("docker")
        .arg("compose")
        .args(args)
        .current_dir(project_dir)
        .status()
        .with_context(|| "failed to invoke `docker`. Is Docker installed?")?;
    if !status.success() {
        anyhow::bail!("docker compose {} failed", args.join(" "));
    }
    Ok(())
}

/// List the service names declared in the compose file.
pub fn services(project_dir: &Path) -> Result<Vec<String>> {
    let out = Command::new("docker")
        .args(["compose", "config", "--services"])
        .current_dir(project_dir)
        .output()
        .with_context(|| "failed to invoke `docker compose config`")?;
    if !out.status.success() {
        anyhow::bail!(
            "docker compose config --services failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}
