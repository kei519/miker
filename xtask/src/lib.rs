use std::process::ExitCode;

use clap::Parser;

use crate::{command::Command, common::Arch};

mod build;
mod check;
mod clippy;
mod command;
mod common;
mod config;
mod image;
mod run;

pub enum Status<T = ()> {
    Success(T),
    Failure,
}

impl Status {
    pub const fn success() -> Self {
        Status::Success(())
    }
}

impl<T> Status<T> {
    pub fn into_simple(self) -> Status {
        match self {
            Status::Success(_) => Status::Success(()),
            Status::Failure => Status::Failure,
        }
    }
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
                Status::Success(()) => 0,
                Status::Failure => 101,
            })
            .unwrap_or_else(|e| {
                eprintln!("{e:?}");
                1
            })
            .into()
    }
}
