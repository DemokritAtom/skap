//! Thin wrappers around `git2` for the operations creo needs:
//! init, initial commit, remote configuration.

use std::path::Path;

use anyhow::{Context, Result};
use git2::{Repository, Signature};

/// Initialize a new git repo at `path` and create an initial commit
/// containing every file in the working tree.
pub fn init_with_initial_commit(path: &Path, message: &str) -> Result<()> {
    let repo = Repository::init(path).with_context(|| format!("git init at {}", path.display()))?;

    // Stage everything.
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Build a signature, falling back to a generic one if git config is empty.
    let sig = author_signature(&repo)?;
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?;
    Ok(())
}

/// Add (or replace) the `origin` remote on the repo at `path`.
pub fn set_remote_origin(path: &Path, url: &str) -> Result<()> {
    let repo = Repository::open(path)?;
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", url)?;
    } else {
        repo.remote("origin", url)?;
    }
    Ok(())
}

/// Returns the configured author name from local or global git config,
/// falling back to "creo".
pub fn detect_author_name() -> String {
    if let Ok(cfg) = git2::Config::open_default() {
        if let Ok(name) = cfg.get_string("user.name") {
            if !name.is_empty() {
                return name;
            }
        }
    }
    "creo".to_string()
}

fn author_signature(repo: &Repository) -> Result<Signature<'static>> {
    let cfg = repo.config().ok();
    let name = cfg
        .as_ref()
        .and_then(|c| c.get_string("user.name").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "creo".to_string());
    let email = cfg
        .as_ref()
        .and_then(|c| c.get_string("user.email").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "creo@local".to_string());
    Ok(Signature::now(&name, &email)?)
}
