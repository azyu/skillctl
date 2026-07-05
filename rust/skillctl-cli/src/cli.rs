use crate::version;
use clap::{Parser, Subcommand};
use skillctl_core::Request;

const QUICK_START: &str = "\
Quick start:
  skillctl list
  skillctl plan
  skillctl apply
  skillctl doctor

Notes:
  - Edit canonical skills under ~/.skillctl/skills.
  - Runtime target directories should point at ~/.skillctl/rendered via symlinks.";
#[derive(Debug, Parser)]
#[command(
    name = "skillctl",
    version = version::CLAP_LONG_VERSION,
    long_version = version::CLAP_LONG_VERSION,
    about = "Materialize Agent Skills into runtime-specific skill directories",
    after_help = QUICK_START,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Preview target changes without mutating files")]
    Plan,
    #[command(about = "Render configured skills and update target symlinks")]
    Apply,
    #[command(about = "Check source, target, lockfile, and symlink health")]
    Doctor,
    #[command(about = "List configured canonical skill IDs")]
    List,
    #[command(about = "Remove stale lockfile-managed target symlinks")]
    Prune,
    #[command(about = "Show CLI version metadata")]
    Version,
    #[command(about = "Remove one lockfile-managed skill symlink")]
    Unlink {
        skill: String,
        #[arg(long)]
        target: Option<String>,
    },
}

impl From<Command> for Request {
    fn from(command: Command) -> Self {
        match command {
            Command::Plan => Request::Plan,
            Command::Apply => Request::Apply,
            Command::Doctor => Request::Doctor,
            Command::List => Request::List,
            Command::Prune => Request::Prune,
            Command::Unlink { skill, target } => Request::Unlink { skill, target },
            Command::Version => unreachable!("version is handled by the CLI crate"),
        }
    }
}
