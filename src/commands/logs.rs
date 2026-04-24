//! `skap logs` – stream container logs.

use anyhow::Result;

use crate::cli::LogsArgs;
use crate::commands::common::resolve_project;
use crate::core::docker;

pub async fn run(args: LogsArgs) -> Result<()> {
    let (_name, entry) = resolve_project(args.project.as_deref())?;
    let dir = std::path::Path::new(&entry.path);
    let tail = args.tail.to_string();
    let mut argv: Vec<&str> = vec!["logs", "-f", "--tail", &tail];
    if let Some(svc) = &args.service {
        argv.push(svc);
    }
    docker::compose(dir, &argv)?;
    Ok(())
}
