use std::{
    io::Result,
    process::{Command, ExitCode},
};

use clap::{Parser, ValueEnum};

use crate::common::Arch;

#[derive(Debug, Default, Clone, Copy, clap::ValueEnum)]
pub enum MessageFormat {
    #[default]
    Human,
    Short,
    Json,
    JsonDiagnosticShort,
    JsonDiagnosticRenderedAnsi,
    JsonRenderDiagnostic,
}

impl std::fmt::Display for MessageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value().unwrap().get_name().fmt(f)
    }
}

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
    pub fn run(self) -> ExitCode {
        match self.run_impl() {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Failed to check: {e}");
                1.into()
            }
        }
    }

    fn run_impl(self) -> Result<ExitCode> {
        let mut exitcode = ExitCode::SUCCESS;

        for package in ["bootloader", "xtask"] {
            let mut cmd = Command::new("cargo");
            cmd.args([
                "check",
                &format!("--package={package}"),
                &format!("--message-format={}", self.message_format),
            ]);

            if self.keep_going {
                cmd.arg("--keep-going");
            }
            if self.all_targets {
                cmd.arg("--all-targets");
            }
            if self.locked {
                cmd.arg("--locked");
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
