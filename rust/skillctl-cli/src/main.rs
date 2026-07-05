mod cli;
mod version;

use clap::{CommandFactory, Parser};
use std::process::ExitCode;

fn main() -> ExitCode {
    let command = cli::Cli::parse().command;
    match command {
        None => {
            cli::Cli::command()
                .print_help()
                .expect("help should render");
            println!();
            ExitCode::SUCCESS
        }
        Some(cli::Command::Version) => {
            print!("{}", version::metadata());
            ExitCode::SUCCESS
        }
        Some(command) => match skillctl_core::run(command.into()) {
            Ok(output) => {
                if !output.stdout.is_empty() {
                    print!("{}", output.stdout);
                }
                if !output.stderr.is_empty() {
                    eprint!("{}", output.stderr);
                }
                ExitCode::from(output.exit_code)
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(1)
            }
        },
    }
}
