use std::process::{Command, ExitCode};

use anyhow::Result;
use clap::Parser;

use crate::{
    common::Arch,
    config::{self, BuildConfig, PackageMetadata},
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
    pub fn run(self) -> ExitCode {
        self.run_impl().unwrap_or_else(|e| {
            eprintln!("{e}");
            ExitCode::FAILURE
        })
    }

    pub fn run_impl(self) -> Result<ExitCode> {
        let mut exitcode = ExitCode::SUCCESS;

        let cargo_metadata = config::metadata()?;
        let build_config = BuildConfig::from_root(&cargo_metadata.workspace_root)?;

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

            let status = cmd.status()?;
            if !status.success() {
                exitcode = status
                    .code()
                    .map(|code| ExitCode::from(code.clamp(0, u8::MAX as _) as u8))
                    .unwrap_or(ExitCode::FAILURE);

                if !self.keep_going {
                    return Ok(exitcode);
                }
            }
        }

        Ok(exitcode)
    }
}
