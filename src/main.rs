// SPDX-License-Identifier: EUPL-1.2
use clap::Parser;

use hashtree_rs::app;
use hashtree_rs::cli::Cli;

fn main() -> std::process::ExitCode {
    app::run(Cli::parse())
}
