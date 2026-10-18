use std::process::ExitCode;

use clap::Parser;
use xtask::Cli;

fn main() -> ExitCode {
    Cli::parse().run()
}
