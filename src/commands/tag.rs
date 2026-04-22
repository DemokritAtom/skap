//! `creo tag add|remove|list` – manage project tags after creation.

use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::{TagAction, TagArgs, TagListArgs, TagMutateArgs};
use crate::config::registry::Registry;
use crate::utils::output;

pub async fn run(args: TagArgs) -> Result<()> {
    match args.action {
        TagAction::Add(a) => add(a),
        TagAction::Remove(a) => remove(a),
        TagAction::List(a) => list(a),
    }
}

fn add(a: TagMutateArgs) -> Result<()> {
    let mut reg = Registry::load()?;
    let entry = reg
        .get_mut(&a.project)
        .with_context(|| format!("project '{}' is not registered", a.project))?;
    if entry.tags.iter().any(|t| t == &a.tag) {
        output::info(&format!("'{}' hat den Tag '{}' bereits", a.project, a.tag));
        return Ok(());
    }
    entry.tags.push(a.tag.clone());
    entry.tags.sort();
    reg.save()?;
    output::success(&format!("Tag '{}' an '{}' angehängt", a.tag, a.project));
    Ok(())
}

fn remove(a: TagMutateArgs) -> Result<()> {
    let mut reg = Registry::load()?;
    let entry = reg
        .get_mut(&a.project)
        .with_context(|| format!("project '{}' is not registered", a.project))?;
    let before = entry.tags.len();
    entry.tags.retain(|t| t != &a.tag);
    if entry.tags.len() == before {
        output::warn(&format!("'{}' trägt keinen Tag '{}'", a.project, a.tag));
        return Ok(());
    }
    reg.save()?;
    output::success(&format!("Tag '{}' von '{}' entfernt", a.tag, a.project));
    Ok(())
}

fn list(a: TagListArgs) -> Result<()> {
    let reg = Registry::load()?;
    if let Some(name) = a.project {
        let entry = reg
            .get(&name)
            .with_context(|| format!("project '{}' is not registered", name))?;
        if entry.tags.is_empty() {
            output::info(&format!("'{name}' hat keine Tags"));
        } else {
            println!("{}", entry.tags.join(", "));
        }
        return Ok(());
    }

    if reg.projects.is_empty() {
        output::info("Keine Projekte registriert.");
        return Ok(());
    }
    let width = reg.projects.keys().map(|n| n.len()).max().unwrap_or(0);
    for (name, e) in &reg.projects {
        let tags = if e.tags.is_empty() {
            "─".dimmed().to_string()
        } else {
            e.tags.join(", ")
        };
        println!("{:<width$}  {tags}", name, width = width);
    }
    Ok(())
}
