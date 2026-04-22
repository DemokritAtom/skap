//! Shared helpers used by multiple commands.

use anyhow::{bail, Result};

use crate::config::registry::{ProjectEntry, Registry};

/// Resolve `(name, entry)` for a project. If `requested` is `Some`, looks
/// it up in the registry. Otherwise infers the project from the current
/// working directory.
pub fn resolve_project(requested: Option<&str>) -> Result<(String, ProjectEntry)> {
    let registry = Registry::load()?;
    if let Some(name) = requested {
        match registry.get(name) {
            Some(e) => Ok((name.to_string(), e.clone())),
            None => bail!("project '{name}' is not registered"),
        }
    } else {
        let cwd = std::env::current_dir()?;
        match registry.find_by_path(&cwd) {
            Some((name, e)) => Ok((name.clone(), e.clone())),
            None => bail!(
                "could not infer project from current directory ({}). Pass a project name.",
                cwd.display()
            ),
        }
    }
}
