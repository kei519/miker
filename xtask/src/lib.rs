use std::process::ExitCode;

use clap::Parser;

use crate::{command::Command, common::Arch};

mod build;
mod check;
mod clippy;
mod command;
mod common;
mod config;

pub enum Status {
    Success,
    Failure,
}

/// Provide automations for miker.
#[derive(Debug, Parser)]
#[command(bin_name = "cargo xtask")]
pub struct Cli {
    /// Target architecture.
    #[arg(long, value_enum, global = true)]
    #[cfg_attr(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        arg(default_value_t = Arch::host())
    )]
    arch: Arch,

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> ExitCode {
        self.command
            .run()
            .map(|status| match status {
                Status::Success => 0,
                Status::Failure => 101,
            })
            .unwrap_or_else(|e| {
                eprintln!("{e:#}");
                1
            })
            .into()
    }
}
