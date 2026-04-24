//! Stack detection in existing projects.
//!
//! Used by `skap clone` to pick a sensible template when a `.skap.toml`
//! is not present in the cloned repository, and by `skap doctor` to
//! sanity-check registered projects.
//!
//! The order of checks matters – more specific signals are tested first.

use std::path::Path;

/// Return the name of the closest matching built-in template, or `None`
/// if no recognisable stack was detected.
pub fn detect_stack(path: &Path) -> Option<&'static str> {
    // Next.js – distinctive config files.
    if path.join("next.config.js").exists()
        || path.join("next.config.ts").exists()
        || path.join("next.config.mjs").exists()
    {
        return Some("next");
    }

    // Vite-based React / Vue / Svelte.
    if path.join("vite.config.ts").exists() || path.join("vite.config.js").exists() {
        // Try to differentiate by package.json content if present.
        if let Ok(pkg) = std::fs::read_to_string(path.join("package.json")) {
            if pkg.contains("\"vue\"") {
                return Some("vue");
            }
            if pkg.contains("\"svelte\"") {
                return Some("svelte");
            }
        }
        return Some("react");
    }

    if path.join("nuxt.config.ts").exists() || path.join("nuxt.config.js").exists() {
        return Some("vue");
    }
    if path.join("svelte.config.js").exists() {
        return Some("svelte");
    }

    // Python.
    if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        if path.join("manage.py").exists() {
            return Some("django");
        }
        return Some("fastapi");
    }

    // Rust.
    if path.join("Cargo.toml").exists() {
        // If a `Dockerfile` exists assume axum-style server, else CLI.
        if path.join("Dockerfile").exists() {
            return Some("axum");
        }
        return Some("rust-cli");
    }

    // Go.
    if path.join("go.mod").exists() {
        if path.join("Dockerfile").exists() {
            return Some("go-api");
        }
        return Some("go-cli");
    }

    // Plain Node project.
    if path.join("package.json").exists() {
        return Some("express");
    }

    // Last resort: just docker.
    if path.join("docker-compose.yml").exists() || path.join("docker-compose.yaml").exists() {
        return Some("docker-only");
    }

    None
}

/// Try to read host-side ports from a `docker-compose.yml`. Returns the
/// raw list of declared host ports, deduplicated.
pub fn parse_compose_ports(compose: &str) -> Vec<u16> {
    let mut hits = Vec::new();
    for line in compose.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("- ") && !trimmed.starts_with('"') {
            continue;
        }
        let mut buf = String::new();
        for c in trimmed.chars() {
            if c.is_ascii_digit() {
                buf.push(c);
            } else if !buf.is_empty() && c == ':' {
                if let Ok(p) = buf.parse::<u16>() {
                    hits.push(p);
                }
                buf.clear();
            } else {
                buf.clear();
            }
        }
    }
    hits.sort();
    hits.dedup();
    hits
}
