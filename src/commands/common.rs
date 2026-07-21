//! Shared helpers used by multiple commands.

use anyhow::{bail, Result};

use crate::config::registry::{ProjectEntry, Registry};

/// Validate a project name used as a directory name, Docker
/// `container_name`, and port-registry key.
///
/// Docker container names only allow `[a-zA-Z0-9][a-zA-Z0-9_.-]*`, and a
/// name containing `/` would escape the intended target directory. We
/// reject anything outside that safe set up front so problems surface
/// immediately at `skap new`/`clone`/`rename` time instead of as a
/// confusing `docker compose` error later.
pub fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("project name must not be empty");
    }
    if name == "." || name == ".." {
        bail!("invalid project name: '{name}'");
    }
    let first_ok = name
        .chars()
        .next()
        .map(|c| c.is_ascii_alphanumeric())
        .unwrap_or(false);
    let rest_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !first_ok || !rest_ok {
        bail!(
            "invalid project name '{name}': only letters, digits, '-', '_' and '.' are allowed, and it must start with a letter or digit"
        );
    }
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_names() {
        for name in ["myapp", "my-app", "my_app", "app2", "App.v2"] {
            assert!(
                validate_project_name(name).is_ok(),
                "expected {name} to be valid"
            );
        }
    }

    #[test]
    fn rejects_empty_and_dots() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name(".").is_err());
        assert!(validate_project_name("..").is_err());
    }

    #[test]
    fn rejects_spaces_and_path_separators() {
        for name in ["my app", "my/app", "../evil", "/etc/passwd", "app/"] {
            assert!(
                validate_project_name(name).is_err(),
                "expected {name} to be rejected"
            );
        }
    }

    #[test]
    fn rejects_leading_punctuation() {
        assert!(validate_project_name("-app").is_err());
        assert!(validate_project_name("_app").is_err());
        assert!(validate_project_name(".app").is_err());
    }
}
