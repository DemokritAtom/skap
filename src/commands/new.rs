//! `creo new` – create a new project from a template.
//!
//! Workflow:
//!  1. Resolve template (positional/flag, default `docker-only`).
//!  2. Determine target directory (cwd / `<name>`); refuse if it exists.
//!  3. Auto-assign free ports for every service of the template (or honour
//!     `--port` as the new base port).
//!  4. Render the template into the target directory.
//!  5. Optionally `git init` + initial commit.
//!  6. Optionally create a license file.
//!  7. Register the project in the registry.
//!  8. Optionally create a remote on the configured git provider
//!     (todo: handled in step 11 / `creo add github` for now).
//!  9. Print a clean summary.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::cli::NewArgs;
use crate::config::global::GlobalConfig;
use crate::config::ports::PortRegistry;
use crate::config::registry::{ProjectEntry, Registry};
use crate::core::git as creo_git;
use crate::core::ports as port_core;
use crate::core::templates::Template;
use crate::utils::output;

pub async fn run(args: NewArgs) -> Result<()> {
    let cfg = GlobalConfig::load()?;
    let mut registry = Registry::load()?;
    let mut port_reg = PortRegistry::load()?;

    if registry.get(&args.name).is_some() {
        bail!("a project named '{}' is already registered", args.name);
    }

    // 1. resolve template
    let template_name = args
        .template_flag
        .clone()
        .or_else(|| args.template.clone())
        .unwrap_or_else(|| "docker-only".to_string());
    let template = Template::load_builtin(&template_name)?;

    // 2. target dir
    let target_dir: PathBuf = std::env::current_dir()?.join(&args.name);
    if target_dir.exists() {
        bail!("target directory already exists: {}", target_dir.display());
    }

    // 3. assign ports
    let base_port = args.port.unwrap_or(cfg.ports.base_port);
    let services: Vec<(String, u16)> = template
        .meta
        .services
        .iter()
        .map(|s| (s.name.clone(), s.port_offset))
        .collect();
    let assigned_ports = port_core::assign_ports(&args.name, &services, &mut port_reg, base_port);

    // Build deterministic ports BTreeMap for the template context.
    let mut ports_for_ctx: BTreeMap<String, u16> = BTreeMap::new();
    for (svc, port) in &assigned_ports {
        ports_for_ctx.insert(svc.clone(), *port);
    }

    // 4. render
    let license = args
        .license
        .clone()
        .unwrap_or_else(|| cfg.defaults.license.clone());
    let author = creo_git::detect_author_name();
    let ctx = crate::core::templates::make_context(&args.name, &license, &author, &ports_for_ctx);
    template
        .render_to(&target_dir, &ctx)
        .with_context(|| format!("failed to render template '{template_name}'"))?;

    // Bootstrap a .env from .env.example so docker compose env_file works
    // out of the box. Users can then edit .env without touching the
    // committed example.
    let example = target_dir.join(".env.example");
    let dotenv = target_dir.join(".env");
    if example.exists() && !dotenv.exists() {
        let _ = std::fs::copy(&example, &dotenv);
    }

    output::success(&format!(
        "Projekt \"{}\" erstellt  ({})",
        args.name,
        target_dir.display()
    ));
    output::success(&format!("Template: {template_name}"));

    // 5. optional license file
    let want_license = !license.eq_ignore_ascii_case("none");
    if want_license {
        write_license(&target_dir, &license, &author)?;
        output::step(&format!("Lizenzdatei: {license}"));
    }

    // 6. git init
    let want_git = !args.no_git && cfg.defaults.git;
    if want_git {
        creo_git::init_with_initial_commit(&target_dir, "Initial commit (creo)")?;
        output::success("Git initialisiert (initial commit)");
    }

    // 7. docker note
    let want_docker = !args.no_docker && cfg.defaults.docker;
    if want_docker {
        let port_str = ordered_port_summary(&assigned_ports);
        output::success(&format!("Docker Compose generiert (Ports: {port_str})"));
    } else {
        // If user opted out, drop the freshly written compose file/Dockerfile
        // for a cleaner project skeleton.
        for f in ["docker-compose.yml", "Dockerfile"] {
            let p = target_dir.join(f);
            if p.exists() {
                let _ = std::fs::remove_file(&p);
            }
        }
        // Also release reservations again.
        port_reg.release_project(&args.name);
        output::step("Docker übersprungen (--no-docker)");
    }

    // 8. registry
    let entry = ProjectEntry {
        path: target_dir.to_string_lossy().into_owned(),
        template: template_name.clone(),
        created: Utc::now(),
        tags: args.tags.clone(),
        docker: want_docker,
        git: want_git,
        git_remote: String::new(),
        ports: if want_docker {
            let mut v: Vec<u16> = assigned_ports.values().copied().collect();
            v.sort();
            v
        } else {
            Vec::new()
        },
        archived: false,
    };
    registry.insert(args.name.clone(), entry);
    registry.save()?;
    port_reg.save()?;
    output::step("Registriert in creo registry");

    // Per-project marker file (.creo.toml). Always written so that other
    // contributors who clone the repo can run `creo doctor` immediately.
    let pf_ports: BTreeMap<String, u16> = if want_docker {
        ports_for_ctx.clone()
    } else {
        BTreeMap::new()
    };
    let pf = crate::core::project_file::ProjectFile::new(&args.name, &template_name, pf_ports);
    pf.save(&target_dir).ok();

    // 9. optional remote – delegated to `creo add github/gitlab` for now.
    if args.git_remote {
        output::warn(
            "--git-remote: please run `creo add github` or `creo add gitlab` to publish (token-aware)",
        );
    }

    // 10. open editor if requested
    if args.editor {
        let _ = std::process::Command::new(&cfg.defaults.editor)
            .arg(&target_dir)
            .spawn();
    }

    println!();
    output::info(&format!("Starten mit: creo start {}", args.name));
    Ok(())
}

fn ordered_port_summary(ports: &std::collections::HashMap<String, u16>) -> String {
    let mut entries: Vec<(&String, &u16)> = ports.iter().collect();
    entries.sort_by_key(|(svc, _)| (*svc).clone());
    entries
        .into_iter()
        .map(|(svc, p)| format!("{svc}:{p}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_license(target_dir: &std::path::Path, name: &str, author: &str) -> Result<()> {
    let year = Utc::now().format("%Y");
    let body = match name.to_ascii_uppercase().as_str() {
        "MIT" => format!(
            "MIT License\n\nCopyright (c) {year} {author}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the \"Software\"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in\nall copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND.\n"
        ),
        "APACHE" | "APACHE-2.0" => format!(
            "Apache License 2.0\nCopyright {year} {author}\n\nLicensed under the Apache License, Version 2.0 (the \"License\");\nyou may not use this file except in compliance with the License.\nYou may obtain a copy of the License at\n\n    http://www.apache.org/licenses/LICENSE-2.0\n"
        ),
        "GPL" | "GPL-3.0" => format!(
            "GNU General Public License v3.0\nCopyright (C) {year} {author}\nSee https://www.gnu.org/licenses/gpl-3.0.txt for the full text.\n"
        ),
        other => format!("{other} License\nCopyright (c) {year} {author}\n"),
    };
    std::fs::write(target_dir.join("LICENSE"), body)?;
    Ok(())
}
