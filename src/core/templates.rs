//! Template engine.
//!
//! A template is a directory under `templates/<name>/` containing:
//!  * `template.toml`   – metadata (name, language, services with ports)
//!  * `*.tera`          – Tera files which are rendered to the target path
//!    with the `.tera` suffix stripped
//!  * any other files   – copied verbatim
//!
//! Built-in templates ship as part of the binary via `include_dir`-style
//! manual registration in [`builtin_templates`]. We embed each template
//! as `(relative_path, contents)` pairs at compile time using
//! `include_str!` so the binary stays a single file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use tera::{Context as TeraCtx, Tera};

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TemplateMeta {
    pub template: TemplateInfo,
    #[serde(default)]
    pub services: Vec<ServiceSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TemplateInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub language: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ServiceSpec {
    pub name: String,
    #[serde(default)]
    pub port_offset: u16,
    #[serde(default)]
    pub dockerfile: bool,
}

/// A loaded template ready to be rendered.
pub struct Template {
    pub meta: TemplateMeta,
    /// Files belonging to the template: `(relative_path, contents)`.
    pub files: Vec<(String, String)>,
}

impl Template {
    /// Load a built-in template by name.
    pub fn load_builtin(name: &str) -> Result<Self> {
        let bundle = builtin_templates()
            .into_iter()
            .find(|(n, _)| *n == name)
            .with_context(|| format!("unknown template: {name}"))?;
        let mut meta_raw: Option<&str> = None;
        let mut files: Vec<(String, String)> = Vec::new();
        for (path, contents) in bundle.1 {
            if path == "template.toml" {
                meta_raw = Some(contents);
            } else {
                files.push((path.to_string(), contents.to_string()));
            }
        }
        let meta_raw =
            meta_raw.with_context(|| format!("template {name} missing template.toml"))?;
        let meta: TemplateMeta = toml::from_str(meta_raw)
            .with_context(|| format!("failed to parse template.toml for {name}"))?;
        Ok(Self { meta, files })
    }

    /// Names of every built-in template.
    #[allow(dead_code)]
    pub fn builtin_names() -> Vec<&'static str> {
        builtin_templates().into_iter().map(|(n, _)| n).collect()
    }

    /// Render every file in the template into `target_dir`. Files ending
    /// with `.tera` are processed through Tera; the suffix is stripped
    /// from the output path. Other files are written verbatim.
    pub fn render_to(&self, target_dir: &Path, ctx: &TeraCtx) -> Result<()> {
        std::fs::create_dir_all(target_dir)
            .with_context(|| format!("failed to create {}", target_dir.display()))?;

        for (rel, contents) in &self.files {
            let (out_rel, rendered) = if let Some(stripped) = rel.strip_suffix(".tera") {
                let mut tera = Tera::default();
                tera.add_raw_template(rel, contents)
                    .with_context(|| format!("failed to register template file {rel}"))?;
                let rendered = tera
                    .render(rel, ctx)
                    .with_context(|| format!("failed to render {rel}"))?;
                (stripped.to_string(), rendered)
            } else {
                (rel.clone(), contents.clone())
            };

            let out_path = target_dir.join(&out_rel);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            std::fs::write(&out_path, rendered)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
        }
        Ok(())
    }
}

/// Build the Tera context from project + assigned ports.
pub fn make_context(
    project_name: &str,
    license: &str,
    author: &str,
    ports: &BTreeMap<String, u16>,
) -> TeraCtx {
    let mut ctx = TeraCtx::new();
    ctx.insert("project_name", project_name);
    ctx.insert("project_name_pascal", &to_pascal(project_name));
    ctx.insert("project_name_snake", &to_snake(project_name));
    ctx.insert("license", license);
    ctx.insert("author", author);
    ctx.insert("year", &chrono::Utc::now().format("%Y").to_string());
    for (svc, port) in ports {
        ctx.insert(format!("{svc}_port"), port);
    }
    ctx.insert("ports", ports);
    ctx
}

fn to_pascal(s: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for c in s.chars() {
        if c == '-' || c == '_' || c == ' ' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn to_snake(s: &str) -> String {
    s.replace(['-', ' '], "_").to_ascii_lowercase()
}

/// Optionally locate a template directory on disk (for development &
/// user-provided templates). Returns `Some(path)` if `templates/<name>/`
/// exists relative to the current working directory or `$CREO_TEMPLATES`.
#[allow(dead_code)]
pub fn find_template_dir(name: &str) -> Option<PathBuf> {
    if let Ok(env) = std::env::var("CREO_TEMPLATES") {
        let p = PathBuf::from(env).join(name);
        if p.is_dir() {
            return Some(p);
        }
    }
    let p = PathBuf::from("templates").join(name);
    if p.is_dir() {
        return Some(p);
    }
    None
}

// ---------------------------------------------------------------------------
// Built-in template bundles (compiled into the binary).
// ---------------------------------------------------------------------------

/// `(template_name, [(relative_path, contents), ...])`
fn builtin_templates() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    vec![
        (
            "docker-only",
            crate::core::templates_data::DOCKER_ONLY.to_vec(),
        ),
        ("react", crate::core::templates_data::REACT.to_vec()),
        ("next", crate::core::templates_data::NEXT.to_vec()),
        ("vue", crate::core::templates_data::VUE.to_vec()),
        ("svelte", crate::core::templates_data::SVELTE.to_vec()),
        ("express", crate::core::templates_data::EXPRESS.to_vec()),
        ("fastapi", crate::core::templates_data::FASTAPI.to_vec()),
        ("django", crate::core::templates_data::DJANGO.to_vec()),
        ("axum", crate::core::templates_data::AXUM.to_vec()),
        ("go-api", crate::core::templates_data::GO_API.to_vec()),
        ("rust-cli", crate::core::templates_data::RUST_CLI.to_vec()),
        ("go-cli", crate::core::templates_data::GO_CLI.to_vec()),
    ]
}
