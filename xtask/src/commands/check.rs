use std::process::Command;

use anyhow::Result;
use clap::Parser;

use crate::{
    Status,
    common::Arch,
    common::MessageFormat,
    config::{self, BuildConfig, PackageMetadata},
};

/// Check all packages in miker for proper targets derived from specified architecture.
///
/// This is valid for rust-analyzer check command. When you want to edit packages other than this
/// xtask, `cargo xtask check --message-format=json --keep-going --all-targets` works well.
/// Otherwise, set rust-analyzer.check.workspace to false and rust-analyzer.check.extraArgs to
/// `--package xtask`.
// TODO: Handle -j option.
// TODO: Should we Handle --lib, --bins, --tests, --benches and --examples option?
#[derive(Debug, Clone, Parser)]
pub struct Check {
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

impl Check {
    pub fn run(self) -> Result<Status> {
        let mut status = Status::Success;

        let cargo_metadata = config::metadata()?;
        let build_config = BuildConfig::from_root(&cargo_metadata.workspace_root)?;

        for package in cargo_metadata.workspace_packages() {
            let package_metadata = PackageMetadata::from_package(package)?;

            let mut cmd = Command::new("cargo");
            cmd.args([
                "check",
                &format!("--package={}", package.name),
                &format!("--message-format={}", self.message_format),
            ]);

            if let Some(option) = build_config
                .resolve(self.arch, package_metadata.target)?
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
