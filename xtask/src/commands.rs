use anyhow::Result;
use clap::Subcommand;

use crate::{
    Status,
    commands::{build::Build, check::Check},
};

mod build;
mod check;

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Build(Build),
    Check(Check),
}

impl Command {
    pub fn run(self) -> Result<Status> {
        match self {
            Self::Build(build) => build.run(),
            Self::Check(check) => check.run(),
        }
    }
}
