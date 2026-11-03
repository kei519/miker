use anyhow::Result;
use clap::Subcommand;

use crate::{Status, build::BuildCommand, check::CheckCommand, clippy::ClippyCommand};

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Build(BuildCommand),
    Check(CheckCommand),
    Clippy(ClippyCommand),
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
