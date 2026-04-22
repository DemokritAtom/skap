//! `creo open` – open a project in the editor / cd into it.
//!
//! Because a child process can't change the parent shell's cwd, we rely
//! on a small shell function the user adds to their rc file (printed by
//! the help text). When invoked with `--print-path` we only emit the
//! path; with `--editor-only` we only spawn the editor.

use anyhow::Result;

use crate::cli::OpenArgs;
use crate::commands::common::resolve_project;
use crate::config::global::GlobalConfig;
use crate::utils::output;

pub async fn run(args: OpenArgs) -> Result<()> {
    let (_name, entry) = resolve_project(args.project.as_deref())?;
    let cfg = GlobalConfig::load()?;

    if args.print_path {
        println!("{}", entry.path);
        return Ok(());
    }

    let editor = args.editor.clone().unwrap_or(cfg.defaults.editor.clone());
    let path = std::path::PathBuf::from(&entry.path);

    if args.editor_only {
        if !args.no_editor {
            let _ = std::process::Command::new(&editor).arg(&path).spawn();
        }
        return Ok(());
    }

    if !args.no_editor {
        let _ = std::process::Command::new(&editor).arg(&path).spawn();
    }
    output::info(&format!("cd {}", path.display()));
    output::step(
        "Tip: add the shell function from `creo --help` so `creo open` actually changes directory.",
    );
    Ok(())
}
