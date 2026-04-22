//! `creo add` – add a feature (git, docker, ci, ...) to an existing project.

use std::path::Path;

use anyhow::{bail, Context, Result};
use dialoguer::{Input, Select};

use crate::cli::AddArgs;
use crate::commands::common::resolve_project;
use crate::config::global::GlobalConfig;
use crate::config::ports::PortRegistry;
use crate::config::registry::Registry;
use crate::core::{git as creo_git, github, gitlab, ports as port_core};
use crate::utils::output;

pub async fn run(args: AddArgs) -> Result<()> {
    let (name, mut entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::PathBuf::from(&entry.path);
    let cfg = GlobalConfig::load()?;

    match args.feature.to_ascii_lowercase().as_str() {
        "git" => add_git(&dir, &mut entry)?,
        "docker" => add_docker(&dir, &name, &mut entry)?,
        "github" => add_github(&dir, &name, &mut entry, &cfg).await?,
        "gitlab" => add_gitlab(&dir, &name, &mut entry, &cfg).await?,
        "env" => add_env(&dir)?,
        "lint" => add_lint(&dir)?,
        "ci" => add_ci(&dir)?,
        "precommit" => add_precommit(&dir)?,
        "license" => add_license(&dir)?,
        "readme" => add_readme(&dir, &name)?,
        "makefile" => add_makefile(&dir)?,
        "devcontainer" => add_devcontainer(&dir, &name)?,
        "db" => add_db(&dir)?,
        "ssl" => add_ssl(&dir)?,
        other => bail!("unknown feature '{other}'. Try one of: git docker github gitlab env lint ci precommit license readme makefile devcontainer db ssl"),
    }

    // Persist updated registry entry.
    let mut registry = Registry::load()?;
    registry.insert(name, entry);
    registry.save()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Feature implementations
// ---------------------------------------------------------------------------

fn add_git(dir: &Path, entry: &mut crate::config::registry::ProjectEntry) -> Result<()> {
    if dir.join(".git").exists() {
        output::warn("git already initialized");
        return Ok(());
    }
    if !dir.join(".gitignore").exists() {
        std::fs::write(dir.join(".gitignore"), ".env\n.DS_Store\n")?;
    }
    creo_git::init_with_initial_commit(dir, "Initial commit (creo)")?;
    entry.git = true;
    output::success("git initialisiert");
    Ok(())
}

fn add_docker(
    dir: &Path,
    project: &str,
    entry: &mut crate::config::registry::ProjectEntry,
) -> Result<()> {
    if crate::core::docker::has_compose(dir) {
        output::warn("docker compose already present");
        return Ok(());
    }
    let cfg = GlobalConfig::load()?;
    let mut port_reg = PortRegistry::load()?;
    let assigned =
        port_core::assign_ports(project, &[("app".into(), 0)], &mut port_reg, cfg.ports.base_port);
    let app_port = *assigned.get("app").unwrap();
    port_reg.save()?;

    let dockerfile = "FROM alpine:3.20\nWORKDIR /app\nCOPY . .\nCMD [\"sh\",\"-c\",\"sleep infinity\"]\n";
    let compose = format!(
        "services:\n  app:\n    build: .\n    container_name: {project}-app\n    ports:\n      - \"{app_port}:8080\"\n    restart: unless-stopped\n"
    );
    std::fs::write(dir.join("Dockerfile"), dockerfile)?;
    std::fs::write(dir.join("docker-compose.yml"), compose)?;
    entry.docker = true;
    if !entry.ports.contains(&app_port) {
        entry.ports.push(app_port);
        entry.ports.sort();
    }
    output::success(&format!("Docker generiert (Port {app_port})"));
    Ok(())
}

async fn add_github(
    dir: &Path,
    project: &str,
    entry: &mut crate::config::registry::ProjectEntry,
    cfg: &GlobalConfig,
) -> Result<()> {
    if cfg.defaults.github_token.is_empty() {
        bail!("no github_token in config. Set it via: creo config set github_token <token>");
    }
    let url = github::create_repo(&cfg.defaults.github_token, project, false).await?;
    creo_git::set_remote_origin(dir, &url)?;
    entry.git_remote = url.clone();
    output::success(&format!("GitHub remote: {url}"));
    Ok(())
}

async fn add_gitlab(
    dir: &Path,
    project: &str,
    entry: &mut crate::config::registry::ProjectEntry,
    cfg: &GlobalConfig,
) -> Result<()> {
    if cfg.defaults.gitlab_token.is_empty() {
        bail!("no gitlab_token in config. Set it via: creo config set gitlab_token <token>");
    }
    let url = gitlab::create_project(
        &cfg.defaults.gitlab_url,
        &cfg.defaults.gitlab_token,
        project,
        false,
    )
    .await?;
    creo_git::set_remote_origin(dir, &url)?;
    entry.git_remote = url.clone();
    output::success(&format!("GitLab remote: {url}"));
    Ok(())
}

fn add_env(dir: &Path) -> Result<()> {
    let example = dir.join(".env.example");
    let real = dir.join(".env");
    if !example.exists() {
        std::fs::write(&example, "# example variables\n")?;
        output::success(".env.example angelegt");
    }
    if !real.exists() {
        std::fs::copy(&example, &real)?;
        output::success(".env angelegt");
    } else {
        output::warn(".env existiert bereits");
    }
    Ok(())
}

fn add_lint(dir: &Path) -> Result<()> {
    if dir.join("Cargo.toml").exists() {
        // Clippy is built-in; add a config file.
        std::fs::write(dir.join(".clippy.toml"), "# add lints here\n")?;
        output::success("Clippy config angelegt (run: cargo clippy)");
    } else if dir.join("package.json").exists() {
        std::fs::write(
            dir.join(".eslintrc.json"),
            "{\n  \"extends\": [\"eslint:recommended\"],\n  \"env\": { \"node\": true, \"browser\": true, \"es2022\": true }\n}\n",
        )?;
        output::success("ESLint config angelegt");
    } else if dir.join("requirements.txt").exists() || dir.join("pyproject.toml").exists() {
        std::fs::write(dir.join("ruff.toml"), "line-length = 100\n")?;
        output::success("Ruff config angelegt");
    } else {
        output::warn("could not detect language stack – no lint config added");
    }
    Ok(())
}

fn add_ci(dir: &Path) -> Result<()> {
    let path = dir.join(".github/workflows/ci.yml");
    if path.exists() {
        output::warn("CI workflow already present");
        return Ok(());
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    let body = "name: CI\non: [push, pull_request]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: echo 'add your build/test steps here'\n";
    std::fs::write(path, body)?;
    output::success(".github/workflows/ci.yml angelegt");
    Ok(())
}

fn add_precommit(dir: &Path) -> Result<()> {
    let path = dir.join(".pre-commit-config.yaml");
    if path.exists() {
        output::warn("pre-commit config already present");
        return Ok(());
    }
    std::fs::write(
        &path,
        "repos:\n  - repo: https://github.com/pre-commit/pre-commit-hooks\n    rev: v4.6.0\n    hooks:\n      - id: trailing-whitespace\n      - id: end-of-file-fixer\n      - id: check-yaml\n",
    )?;
    output::success(".pre-commit-config.yaml angelegt");
    Ok(())
}

fn add_license(dir: &Path) -> Result<()> {
    let options = ["MIT", "Apache-2.0", "GPL-3.0", "BSD-3-Clause"];
    let pick = Select::new()
        .with_prompt("Lizenz")
        .items(&options)
        .default(0)
        .interact()?;
    let author = creo_git::detect_author_name();
    let year = chrono::Utc::now().format("%Y");
    let body = format!("{} License\nCopyright (c) {year} {author}\n", options[pick]);
    std::fs::write(dir.join("LICENSE"), body)?;
    output::success(&format!("LICENSE ({}) angelegt", options[pick]));
    Ok(())
}

fn add_readme(dir: &Path, project: &str) -> Result<()> {
    let p = dir.join("README.md");
    if p.exists() {
        output::warn("README.md already present");
        return Ok(());
    }
    std::fs::write(&p, format!("# {project}\n\nGenerated by creo.\n"))?;
    output::success("README.md angelegt");
    Ok(())
}

fn add_makefile(dir: &Path) -> Result<()> {
    let p = dir.join("Makefile");
    if p.exists() {
        output::warn("Makefile already present");
        return Ok(());
    }
    let body = ".PHONY: build run test clean\n\nbuild:\n\t@echo 'add build steps'\n\nrun:\n\tdocker compose up -d\n\nstop:\n\tdocker compose down\n\ntest:\n\t@echo 'add tests'\n\nclean:\n\tdocker compose down -v\n";
    std::fs::write(&p, body)?;
    output::success("Makefile angelegt");
    Ok(())
}

fn add_devcontainer(dir: &Path, project: &str) -> Result<()> {
    let dc = dir.join(".devcontainer/devcontainer.json");
    if dc.exists() {
        output::warn("devcontainer already present");
        return Ok(());
    }
    std::fs::create_dir_all(dc.parent().unwrap())?;
    let body = format!(
        "{{\n  \"name\": \"{project}\",\n  \"dockerComposeFile\": \"../docker-compose.yml\",\n  \"service\": \"app\",\n  \"workspaceFolder\": \"/app\"\n}}\n"
    );
    std::fs::write(&dc, body)?;
    output::success(".devcontainer/devcontainer.json angelegt");
    Ok(())
}

fn add_db(dir: &Path) -> Result<()> {
    let dbs = ["postgres", "mysql", "mongo", "redis"];
    let pick = Select::new()
        .with_prompt("Datenbank")
        .items(&dbs)
        .default(0)
        .interact()?;
    let service = match dbs[pick] {
        "postgres" => {
            let name: String = Input::new()
                .with_prompt("Datenbank-Name")
                .default("app".into())
                .interact_text()?;
            format!(
                "  db:\n    image: postgres:16\n    environment:\n      POSTGRES_PASSWORD: postgres\n      POSTGRES_DB: {name}\n    ports:\n      - \"5432:5432\"\n    volumes:\n      - db-data:/var/lib/postgresql/data\n"
            )
        }
        "mysql" => "  db:\n    image: mysql:8\n    environment:\n      MYSQL_ROOT_PASSWORD: root\n    ports:\n      - \"3306:3306\"\n    volumes:\n      - db-data:/var/lib/mysql\n".into(),
        "mongo" => "  db:\n    image: mongo:7\n    ports:\n      - \"27017:27017\"\n    volumes:\n      - db-data:/data/db\n".into(),
        "redis" => "  db:\n    image: redis:7-alpine\n    ports:\n      - \"6379:6379\"\n".into(),
        _ => unreachable!(),
    };

    let compose = dir.join("docker-compose.yml");
    if !compose.exists() {
        bail!("no docker-compose.yml. Run `creo add docker` first.");
    }
    let mut existing = std::fs::read_to_string(&compose)?;
    if !existing.ends_with('\n') {
        existing.push('\n');
    }
    if !existing.contains("volumes:") {
        existing.push_str("\nvolumes:\n  db-data:\n");
    }
    // Insert the service before the `volumes:` block, or at the end.
    let new = if let Some(idx) = existing.find("\nvolumes:") {
        let (head, tail) = existing.split_at(idx);
        format!("{head}{service}{tail}")
    } else {
        format!("{existing}{service}")
    };
    std::fs::write(&compose, new)?;
    output::success(&format!("{} hinzugefügt zu docker-compose.yml", dbs[pick]));
    Ok(())
}

fn add_ssl(dir: &Path) -> Result<()> {
    let cert_dir = dir.join("certs");
    std::fs::create_dir_all(&cert_dir)?;
    let status = std::process::Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:4096",
            "-sha256",
            "-days",
            "365",
            "-nodes",
            "-keyout",
            cert_dir.join("key.pem").to_str().unwrap(),
            "-out",
            cert_dir.join("cert.pem").to_str().unwrap(),
            "-subj",
            "/CN=localhost",
        ])
        .status()
        .with_context(|| "failed to invoke `openssl`. Is it installed?")?;
    if !status.success() {
        bail!("openssl failed");
    }
    output::success("Self-signed Cert in certs/ angelegt");
    Ok(())
}
