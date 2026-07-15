use crate::version;
use clap::{Parser, Subcommand};
use skillctl_core::Request;

const HELP_FOOTER: &str = "\
Quick start:
  skillctl list
  skillctl update
  skillctl plan
  skillctl apply
  skillctl doctor

Paths:
  config:   ~/.skillctl/config.yaml
  skills:   ~/.skillctl/skills
  rendered: ~/.skillctl/rendered
  lockfile: <target>/.skillctl.lock.json

Exit codes:
  0  Success
  1  Operation, configuration, diagnostic, or plan error
  2  Invalid CLI usage

Examples:
  skillctl unlink example-skill
  skillctl unlink example-skill --target claude
  skillctl version

Notes:
  - Run skillctl update to refresh configured Git sources.
  - Runtime target directories should point at ~/.skillctl/rendered via symlinks.";

#[derive(Debug, Parser)]
#[command(
    name = "skillctl",
    version = version::CLAP_LONG_VERSION,
    long_version = version::CLAP_LONG_VERSION,
    about = concat!(
        "skillctl v",
        env!("CARGO_PKG_VERSION"),
        " - Materialize Agent Skills into runtime-specific skill directories"
    ),
    after_help = HELP_FOOTER,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "List configured canonical skill IDs")]
    List,
    #[command(about = "Refresh configured Git skill sources")]
    Update,
    #[command(about = "Preview target changes without mutating files")]
    Plan,
    #[command(about = "Render configured skills and update target symlinks")]
    Apply,
    #[command(about = "Check source, target, lockfile, and symlink health")]
    Doctor,
    #[command(about = "Remove stale lockfile-managed target symlinks")]
    Prune,
    #[command(about = "Remove one lockfile-managed skill symlink")]
    Unlink {
        #[arg(help = "Configured skill ID to unlink")]
        skill: String,
        #[arg(long, help = "Limit unlinking to one configured target")]
        target: Option<String>,
    },
    #[command(about = "Show CLI version metadata")]
    Version,
}

impl From<Command> for Request {
    fn from(command: Command) -> Self {
        match command {
            Command::Plan => Request::Plan,
            Command::Update => Request::Update,
            Command::Apply => Request::Apply,
            Command::Doctor => Request::Doctor,
            Command::List => Request::List,
            Command::Prune => Request::Prune,
            Command::Unlink { skill, target } => Request::Unlink { skill, target },
            Command::Version => unreachable!("version is handled by the CLI crate"),
        }
    }
}
