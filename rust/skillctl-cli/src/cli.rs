use clap::{Parser, Subcommand};
use skillctl_core::Request;

#[derive(Debug, Parser)]
#[command(name = "skillctl")]
#[command(about = "Materialize Agent Skills into runtime-specific skill directories")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Plan,
    Apply,
    Doctor,
    List,
    Prune,
    Unlink {
        skill: String,
        #[arg(long)]
        target: Option<String>,
    },
}

impl From<Cli> for Request {
    fn from(cli: Cli) -> Self {
        match cli.command {
            Command::Plan => Request::Plan,
            Command::Apply => Request::Apply,
            Command::Doctor => Request::Doctor,
            Command::List => Request::List,
            Command::Prune => Request::Prune,
            Command::Unlink { skill, target } => Request::Unlink { skill, target },
        }
    }
}
