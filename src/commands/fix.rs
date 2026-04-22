//! `creo fix` – diagnose & repair common issues.
//!
//! Currently implements `ports`, `env`, `git`, `docker`, `permissions`,
//! `deps` and `all` (running every check in sequence).

use std::path::Path;

use anyhow::{bail, Result};
use dialoguer::Confirm;

use crate::cli::FixArgs;
use crate::commands::common::resolve_project;
use crate::config::ports::PortRegistry;
use crate::config::registry::Registry;
use crate::core::{docker, ports as port_core};
use crate::utils::output;

pub async fn run(args: FixArgs) -> Result<()> {
    let problem = args.problem.to_ascii_lowercase();
    let (name, mut entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::PathBuf::from(&entry.path);

    match problem.as_str() {
        "ports" => fix_ports(&name, &dir, &mut entry)?,
        "env" => fix_env(&dir)?,
        "git" => fix_git(&dir)?,
        "docker" => fix_docker(&dir)?,
        "permissions" => fix_permissions(&dir)?,
        "deps" => fix_deps(&dir)?,
        "all" => {
            fix_ports(&name, &dir, &mut entry).ok();
            fix_env(&dir).ok();
            fix_git(&dir).ok();
            fix_docker(&dir).ok();
            fix_permissions(&dir).ok();
            fix_deps(&dir).ok();
        }
        other => bail!("unknown fix '{other}'"),
    }

    let mut reg = Registry::load()?;
    reg.insert(name, entry);
    reg.save()?;
    Ok(())
}

fn fix_ports(
    project: &str,
    dir: &Path,
    entry: &mut crate::config::registry::ProjectEntry,
) -> Result<()> {
    let compose_path = dir.join("docker-compose.yml");
    if !compose_path.exists() {
        output::warn("no docker-compose.yml – nothing to fix");
        return Ok(());
    }
    let raw = std::fs::read_to_string(&compose_path)?;
    let conflicts = scan_conflicts(&raw);
    if conflicts.is_empty() {
        output::success("no port conflicts found");
        return Ok(());
    }

    println!();
    output::warn(&format!("Konflikte gefunden auf {} Port(s):", conflicts.len()));
    for c in &conflicts {
        println!("  · {}", c);
    }

    let mut port_reg = PortRegistry::load()?;
    let mut new_raw = raw.clone();
    let mut taken = Vec::new();
    let mut changes: Vec<(u16, u16)> = Vec::new();
    for old in &conflicts {
        let candidate = port_core::find_free_port_excluding(*old + 1, &port_reg, &taken);
        taken.push(candidate);
        // Replace `"<old>:` with `"<candidate>:` (host port mappings).
        new_raw = new_raw.replace(&format!("\"{old}:"), &format!("\"{candidate}:"));
        new_raw = new_raw.replace(&format!("- {old}:"), &format!("- {candidate}:"));
        changes.push((*old, candidate));
    }

    println!();
    for (o, n) in &changes {
        println!("  {o} → {n}");
    }
    let confirm = Confirm::new()
        .with_prompt("Ports ändern?")
        .default(true)
        .interact()?;
    if !confirm {
        output::info("abgebrochen");
        return Ok(());
    }
    crate::utils::fs::write_atomic(&compose_path, &new_raw)?;

    // Update registry entries.
    entry.ports = changes.iter().map(|(_, n)| *n).collect();
    entry.ports.sort();
    port_reg.release_project(project);
    for (i, (_, n)) in changes.iter().enumerate() {
        port_reg.reserve(format!("{project}-svc{i}"), *n);
    }
    port_reg.save()?;

    // Restart if it was running.
    if docker::status(dir) == docker::Status::Up {
        output::info("Container neu starten…");
        docker::compose(dir, &["up", "-d"]).ok();
    }
    output::success("Ports aktualisiert");
    Ok(())
}

/// Returns the host-side ports declared in a compose file that are
/// currently bound by another process.
fn scan_conflicts(compose: &str) -> Vec<u16> {
    let mut hits = Vec::new();
    for line in compose.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("- ") && !trimmed.starts_with('"') {
            continue;
        }
        // Find the first `<digits>:` segment on the line.
        let mut chars = trimmed.chars().peekable();
        let mut buf = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                buf.push(c);
                chars.next();
            } else if !buf.is_empty() && c == ':' {
                if let Ok(p) = buf.parse::<u16>() {
                    if !port_core::is_port_free(p) {
                        hits.push(p);
                    }
                }
                buf.clear();
                chars.next();
            } else {
                buf.clear();
                chars.next();
            }
        }
    }
    hits.sort();
    hits.dedup();
    hits
}

fn fix_env(dir: &Path) -> Result<()> {
    let example = dir.join(".env.example");
    let real = dir.join(".env");
    if !example.exists() {
        output::step(".env.example missing – nothing to do");
        return Ok(());
    }
    if !real.exists() {
        std::fs::copy(&example, &real)?;
        output::success(".env created from .env.example");
        return Ok(());
    }
    let example_keys = parse_env_keys(&std::fs::read_to_string(&example)?);
    let mut real_text = std::fs::read_to_string(&real)?;
    let real_keys = parse_env_keys(&real_text);
    let missing: Vec<&String> = example_keys.iter().filter(|k| !real_keys.contains(k)).collect();
    if missing.is_empty() {
        output::success(".env is up to date");
        return Ok(());
    }
    if !real_text.ends_with('\n') {
        real_text.push('\n');
    }
    for k in &missing {
        real_text.push_str(&format!("{k}=\n"));
    }
    std::fs::write(&real, real_text)?;
    output::success(&format!("added {} missing key(s) to .env", missing.len()));
    Ok(())
}

fn parse_env_keys(s: &str) -> Vec<String> {
    s.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.starts_with('#') || l.is_empty() {
                None
            } else {
                l.split_once('=').map(|(k, _)| k.trim().to_string())
            }
        })
        .collect()
}

fn fix_git(dir: &Path) -> Result<()> {
    if !dir.join(".git").exists() {
        output::warn("no git repository – run `creo add git`");
        return Ok(());
    }
    // Verify the repo opens; if locked, suggest manual cleanup.
    match git2::Repository::open(dir) {
        Ok(_) => output::success("git state ok"),
        Err(e) => output::error(&format!("git error: {e}")),
    }
    Ok(())
}

fn fix_docker(dir: &Path) -> Result<()> {
    if !docker::has_compose(dir) {
        output::warn("no compose file");
        return Ok(());
    }
    output::info("Rebuilding compose stack…");
    docker::compose(dir, &["down"])?;
    docker::compose(dir, &["up", "-d", "--build"])?;
    output::success("docker stack rebuilt");
    Ok(())
}

fn fix_permissions(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in walkdir(dir) {
            if let Ok(meta) = entry.metadata() {
                let mode = meta.permissions().mode();
                // Drop world-write on regular files.
                if meta.is_file() && (mode & 0o002) != 0 {
                    let mut p = meta.permissions();
                    p.set_mode(mode & !0o002);
                    let _ = std::fs::set_permissions(entry.path(), p);
                }
            }
        }
        output::success("permissions normalised");
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        output::warn("permission fix is only available on Unix");
    }
    Ok(())
}

fn walkdir(root: &Path) -> Vec<std::fs::DirEntry> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && !p.ends_with(".git") && !p.ends_with("node_modules") && !p.ends_with("target") {
                    stack.push(p);
                }
                out.push(e);
            }
        }
    }
    out
}

fn fix_deps(dir: &Path) -> Result<()> {
    if dir.join("package.json").exists() && !dir.join("node_modules").exists() {
        output::info("Running npm install…");
        let _ = std::process::Command::new("npm").arg("install").current_dir(dir).status();
    }
    if dir.join("requirements.txt").exists() {
        output::info("Running pip install -r requirements.txt…");
        let _ = std::process::Command::new("pip")
            .args(["install", "-r", "requirements.txt"])
            .current_dir(dir)
            .status();
    }
    if dir.join("Cargo.toml").exists() {
        output::info("Running cargo fetch…");
        let _ = std::process::Command::new("cargo").arg("fetch").current_dir(dir).status();
    }
    if dir.join("go.mod").exists() {
        output::info("Running go mod tidy…");
        let _ = std::process::Command::new("go").args(["mod", "tidy"]).current_dir(dir).status();
    }
    output::success("deps updated");
    Ok(())
}
