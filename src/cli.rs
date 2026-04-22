//! CLI definition using `clap` derive API.
//!
//! All subcommands and their flags live here. Command modules under
//! `crate::commands` consume these structs and run the actual logic.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "creo",
    version,
    about = "Lean, fast CLI for managing dev projects",
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Disable emoji output (useful for SSH / minimal terminals).
    #[arg(long = "no-emoji", global = true)]
    pub no_emoji: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new project from a template.
    New(NewArgs),
    /// Add a feature (git, docker, ci, ...) to an existing project.
    Add(AddArgs),
    /// Fix common issues (ports, git, docker, env, ...).
    Fix(FixArgs),
    /// List all registered projects.
    List(ListArgs),
    /// Show detailed status of a single project.
    Status(StatusArgs),
    /// Open a project in the configured editor.
    Open(OpenArgs),
    /// Start a project's docker compose stack.
    Start(ProjectArgs),
    /// Stop a project's docker compose stack.
    Stop(ProjectArgs),
    /// Restart a project's docker compose stack.
    Restart(ProjectArgs),
    /// Tail logs of a project's containers.
    Logs(LogsArgs),
    /// Open an interactive shell in a running container.
    Shell(ShellArgs),
    /// Run an arbitrary command in the project context.
    Run(RunArgs),
    /// Run a system & project diagnosis.
    Doctor,
    /// Clean docker artifacts (images, volumes) for a project.
    Clean(CleanArgs),
    /// Archive a project (hide from `creo list`, stop containers).
    Archive(ProjectArgs),
    /// Read or modify global creo config.
    Config(ConfigArgs),
    /// Self-update creo from GitHub releases.
    Update,
    /// Rename a project (folder, registry, ports).
    Rename(RenameArgs),
    /// Move a project to a different path.
    Move(MoveArgs),
    /// Inspect creo's port reservations.
    Ports(PortsArgs),
    /// Clone an existing repository and register it with creo.
    Clone(CloneArgs),
    /// Manage tags on an existing project.
    Tag(TagArgs),
}

// ---------------------------------------------------------------------------
// Subcommand argument structs
// ---------------------------------------------------------------------------

#[derive(Debug, clap::Args)]
pub struct NewArgs {
    /// Name of the new project (also the directory name).
    pub name: String,
    /// Template to use. Defaults to "docker-only" if omitted.
    pub template: Option<String>,
    /// Override template via flag (alternative to positional).
    #[arg(short = 't', long = "template")]
    pub template_flag: Option<String>,
    /// Skip git init.
    #[arg(long)]
    pub no_git: bool,
    /// Skip docker generation.
    #[arg(long)]
    pub no_docker: bool,
    /// Create a remote on the configured git provider.
    #[arg(long)]
    pub git_remote: bool,
    /// Make the created remote private (only with --git-remote).
    #[arg(long)]
    pub private: bool,
    /// License to add (MIT, Apache, GPL, none).
    #[arg(long)]
    pub license: Option<String>,
    /// Tag to apply (repeatable).
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    /// Open the editor after creation.
    #[arg(long)]
    pub editor: bool,
    /// Manually set the base port (overrides auto-assignment).
    #[arg(long)]
    pub port: Option<u16>,
}

#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// Feature to add (git, docker, github, gitlab, env, lint, ci,
    /// precommit, license, readme, makefile, devcontainer, db, ssl).
    pub feature: String,
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct FixArgs {
    /// What to fix: ports | git | docker | env | deps | permissions | all
    pub problem: String,
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Filter by tag.
    #[arg(long)]
    pub tag: Option<String>,
    /// Show only running projects.
    #[arg(long)]
    pub running: bool,
    /// Include archived projects.
    #[arg(long)]
    pub archived: bool,
}

#[derive(Debug, clap::Args)]
pub struct PortsArgs {
    #[command(subcommand)]
    pub action: PortsAction,
}

#[derive(Debug, Subcommand)]
pub enum PortsAction {
    /// List all reserved ports + live status.
    List(PortsListArgs),
}

#[derive(Debug, clap::Args)]
pub struct PortsListArgs {
    /// Show only ports currently bound by a process.
    #[arg(long)]
    pub used: bool,
    /// Show only reserved ports that are currently free.
    #[arg(long)]
    pub free: bool,
}

#[derive(Debug, clap::Args)]
pub struct RenameArgs {
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, clap::Args)]
pub struct MoveArgs {
    pub project: String,
    pub new_path: String,
}

#[derive(Debug, clap::Args)]
pub struct CloneArgs {
    /// Git URL (https or ssh).
    pub url: String,
    /// Local directory + registry name. Defaults to the repo basename.
    pub name: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct TagArgs {
    #[command(subcommand)]
    pub action: TagAction,
}

#[derive(Debug, Subcommand)]
pub enum TagAction {
    /// Add a tag to a project.
    Add(TagMutateArgs),
    /// Remove a tag from a project.
    Remove(TagMutateArgs),
    /// List tags of a project (or all projects if omitted).
    List(TagListArgs),
}

#[derive(Debug, clap::Args)]
pub struct TagMutateArgs {
    pub project: String,
    pub tag: String,
}

#[derive(Debug, clap::Args)]
pub struct TagListArgs {
    /// Project name. If omitted, lists tags of every registered project.
    pub project: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct OpenArgs {
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
    /// Do not launch the editor, only print/cd into the path.
    #[arg(long)]
    pub no_editor: bool,
    /// Override editor for this invocation.
    #[arg(long)]
    pub editor: Option<String>,
    /// Internal: print only the path (used by the shell wrapper).
    #[arg(long, hide = true)]
    pub print_path: bool,
    /// Internal: only launch the editor (used by the shell wrapper).
    #[arg(long, hide = true)]
    pub editor_only: bool,
}

#[derive(Debug, clap::Args)]
pub struct ProjectArgs {
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct LogsArgs {
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
    /// Number of lines to show from the end of the logs.
    #[arg(long, default_value_t = 50)]
    pub tail: usize,
    /// Restrict logs to a single service.
    #[arg(long)]
    pub service: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ShellArgs {
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
    /// Service to enter; if omitted and multiple services exist, prompts.
    #[arg(long)]
    pub service: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// Project name.
    pub project: String,
    /// Command and arguments to run, e.g. `npm run build`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
    pub cmd: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct CleanArgs {
    /// Project name. If omitted, uses the project of the current directory.
    pub project: Option<String>,
    /// Remove docker images of the project.
    #[arg(long)]
    pub images: bool,
    /// Remove docker volumes of the project.
    #[arg(long)]
    pub volumes: bool,
    /// Remove both images and volumes.
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Set a config value, e.g. `creo config set editor vim`.
    Set { key: String, value: String },
    /// Get a config value.
    Get { key: String },
    /// List all config values.
    List,
    /// Reset (or create) the global config to defaults.
    /// Never touches registry.toml or ports.toml.
    Init {
        /// Overwrite an existing config without confirmation.
        #[arg(long)]
        force: bool,
    },
}
