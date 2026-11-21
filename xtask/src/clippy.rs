use std::process::Command;

use anyhow::{Result, anyhow};
use clap::Parser;

use crate::{
    Status,
    common::Arch,
    common::MessageFormat,
    config::{self, MikerConfig, PackageMetadata},
};

#[derive(Debug, Clone, Parser)]
pub struct ClippyCommand {
    /// Error format.
    #[arg(long, value_enum, default_value_t = Default::default())]
    message_format: MessageFormat,

    /// Do not abort the build as soon as there is an error.
    #[arg(long)]
    keep_going: bool,

    /// Check all targets.
    ///
    /// This is equivalent to specifying --lib --bins --tests --benches --examples.
    #[arg(long)]
    all_targets: bool,

    /// Assert that `Cargo.lock` will remain unchanged.
    #[arg(long)]
    locked: bool,

    #[arg(from_global)]
    arch: Arch,
}

impl ClippyCommand {
    pub fn run(self) -> Result<Status> {
        let mut status = Status::success();

        let cargo_metadata = config::metadata()?;
        let mut build_config = MikerConfig::from_root(&cargo_metadata.workspace_root)?;
        let arch_config = build_config
            .take_arch_config(self.arch)
            .ok_or_else(|| anyhow!("Config for {} is not found", self.arch))?;

        for package in cargo_metadata.workspace_packages() {
            // Skip cargo-xtask itself
            if package.name == env!("CARGO_PKG_NAME") {
                continue;
            }

            let package_metadata = PackageMetadata::from_package(package)?;

            let mut cmd = Command::new("cargo");
            cmd.args([
                "clippy",
                &format!("--package={}", package.name),
                &format!("--message-format={}", self.message_format),
            ]);

            if let Some(option) = arch_config
                .target(&package_metadata.target)?
                .option_for_cargo()
            {
                cmd.arg(option);
            }

            if self.keep_going {
                cmd.arg("--keep-going");
            }
            if self.all_targets {
                cmd.arg("--all-targets");
            }
            if self.locked {
                cmd.arg("--locked");
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
