//! `skap list` – list registered projects as a tabular summary.

use anyhow::Result;
use colored::Colorize;

use crate::cli::ListArgs;
use crate::config::registry::Registry;
use crate::core::docker::{self, Status};
use crate::utils::output;

pub async fn run(args: ListArgs) -> Result<()> {
    let registry = Registry::load()?;
    let mut rows: Vec<Row> = Vec::new();

    for (name, e) in &registry.projects {
        if e.archived && !args.archived {
            continue;
        }
        if let Some(tag) = &args.tag {
            if !e.tags.iter().any(|t| t == tag) {
                continue;
            }
        }
        let status = if e.docker {
            docker::status(std::path::Path::new(&e.path))
        } else {
            Status::Unknown
        };
        if args.running && status != Status::Up {
            continue;
        }
        rows.push(Row {
            name: name.clone(),
            template: e.template.clone(),
            docker: docker_label(status, e.docker, e.archived),
            git: git_label(e.git, !e.git_remote.is_empty()),
            ports: if e.ports.is_empty() {
                "─".to_string()
            } else {
                e.ports
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            },
            tags: e.tags.join(","),
        });
    }

    if rows.is_empty() {
        println!(
            "{}",
            "No projects registered yet. Try: skap new myapp".dimmed()
        );
        return Ok(());
    }

    let header = Row {
        name: "NAME".into(),
        template: "TEMPLATE".into(),
        docker: "DOCKER".into(),
        git: "GIT".into(),
        ports: "PORTS".into(),
        tags: "TAGS".into(),
    };

    let widths = compute_widths(std::iter::once(&header).chain(rows.iter()));
    print_row(&header, &widths, true);
    println!(
        "{}",
        "─".repeat(widths.iter().sum::<usize>() + widths.len() * 2)
    );
    for r in &rows {
        print_row(r, &widths, false);
    }
    Ok(())
}

struct Row {
    name: String,
    template: String,
    docker: String,
    git: String,
    ports: String,
    tags: String,
}

fn docker_label(s: Status, has_docker: bool, archived: bool) -> String {
    if !has_docker {
        return "─".into();
    }
    if archived {
        return output::status_symbol(Status::Down);
    }
    output::status_symbol(s)
}

fn git_label(has_git: bool, has_remote: bool) -> String {
    if !has_git {
        return "no".dimmed().to_string();
    }
    if has_remote {
        if output::emoji_enabled() {
            "✓ remote".green().to_string()
        } else {
            "yes+remote".green().to_string()
        }
    } else if output::emoji_enabled() {
        "✓ local".green().to_string()
    } else {
        "yes".green().to_string()
    }
}

fn compute_widths<'a, I: Iterator<Item = &'a Row>>(rows: I) -> [usize; 6] {
    let mut w = [0usize; 6];
    for r in rows {
        for (i, s) in [&r.name, &r.template, &r.docker, &r.git, &r.ports, &r.tags]
            .iter()
            .enumerate()
        {
            // Approximate display width by visible char count (ANSI color
            // codes from the `colored` crate are invisible and must not
            // count towards column width); emoji are wide but this keeps
            // the implementation dependency-free.
            let len = output::visible_width(s);
            if len > w[i] {
                w[i] = len;
            }
        }
    }
    w
}

fn print_row(r: &Row, w: &[usize; 6], header: bool) {
    let cells = [&r.name, &r.template, &r.docker, &r.git, &r.ports, &r.tags];
    let mut line = String::new();
    for (i, c) in cells.iter().enumerate() {
        let pad = w[i].saturating_sub(output::visible_width(c));
        line.push_str(c);
        for _ in 0..pad {
            line.push(' ');
        }
        line.push_str("  ");
    }
    if header {
        println!("{}", line.bold());
    } else {
        println!("{line}");
    }
}
