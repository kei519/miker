use std::process::Command;

use anyhow::Result;
use clap::Parser;

use crate::{
    Status,
    common::Arch,
    config::{self, MikerConfig, PackageMetadata},
};

#[derive(Debug, Clone, Parser)]
pub struct Build {
    /// Do not abort the build as soon as there is an error.
    #[arg(long)]
    keep_going: bool,

    /// Assert that `Cargo.lock` will remain unchanged.
    #[arg(long)]
    locked: bool,

    #[arg(from_global)]
    arch: Arch,

    #[arg(long)]
    release: bool,
}

impl Build {
    pub fn run(self) -> Result<Status> {
        let mut status = Status::Success;

        let cargo_metadata = config::metadata()?;
        let build_config = MikerConfig::from_root(&cargo_metadata.workspace_root)?;

        for package in cargo_metadata.workspace_packages() {
            let package_metadata = PackageMetadata::from_package(package)?;

            let mut cmd = Command::new(env!("CARGO"));
            cmd.args(["build", &format!("--package={}", package.name)]);

            if let Some(option) = build_config
                .resolve(self.arch, package_metadata.target)?
                .option_for_cargo()
            {
                cmd.arg(option);
            }

            if self.keep_going {
                cmd.arg("--keep-going");
            }
            if self.locked {
                cmd.arg("--locked");
            }
            if self.release {
                cmd.arg("--release");
            }

            if !cmd.status()?.success() {
                status = Status::Failure;

                if !self.keep_going {
                    return Ok(status);
                }
            }
        }

        Ok(status)
    }
}
