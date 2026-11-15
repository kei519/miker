use anyhow::Result;
use clap::Subcommand;

use crate::{
    Status, build::BuildCommand, check::CheckCommand, clippy::ClippyCommand, image::ImageCommand,
    run::RunCommand,
};

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Build(BuildCommand),
    Check(CheckCommand),
    Clippy(ClippyCommand),
    Image(ImageCommand),
    Run(RunCommand),
}

impl Command {
    pub fn run(self) -> Result<Status> {
        match self {
            Self::Build(build) => build.run(),
            Self::Check(check) => check.run(),
            Self::Clippy(clippy) => clippy.run(),
            Self::Image(image) => image.run(),
            Self::Run(run) => run.run(),
        }
    }
}
