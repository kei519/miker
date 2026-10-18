use std::process::ExitCode;

use clap::Subcommand;

use crate::commands::check::Check;

mod check;

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Check(Check),
}

impl Command {
    pub fn run(self) -> ExitCode {
        match self {
            Self::Check(check) => check.run(),
        }
    }
}
