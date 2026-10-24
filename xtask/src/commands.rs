use anyhow::Result;
use clap::Subcommand;

use crate::{
    Status,
    commands::{build::Build, check::Check, clippy::Clippy},
};

mod build;
mod check;
mod clippy;

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Build(Build),
    Check(Check),
    Clippy(Clippy),
}

impl Command {
    pub fn run(self) -> Result<Status> {
        match self {
            Self::Build(build) => build.run(),
            Self::Check(check) => check.run(),
            Self::Clippy(clippy) => clippy.run(),
        }
    }
}
