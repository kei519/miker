use std::process::ExitCode;

use clap::Parser;

use crate::{commands::Command, common::Arch};

mod commands;
mod common;

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
        self.command.run()
    }
}
