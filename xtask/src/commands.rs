use std::process::ExitCode;

use clap::Subcommand;

use crate::commands::{build::Build, check::Check};

mod build;
mod check;

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Build(Build),
    Check(Check),
}

impl Command {
    pub fn run(self) -> ExitCode {
        match self {
            Self::Build(build) => build.run(),
            Self::Check(check) => check.run(),
        }
    }
}
