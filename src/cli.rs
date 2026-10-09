use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "hashtree",
    about = "Creating a list of digests for files and/or directories"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Create a digest manifest from files/directories
    Create(CreateArgs),
    /// Verify files/directories against a digest manifest
    Verify(VerifyArgs),
}

#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(short = 'f', long = "force")]
    pub force: bool,

    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct VerifyArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(short = 'n', long = "ignorecase")]
    pub ignore_case: bool,

    #[arg(short = 'i', long = "input")]
    pub input: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct CommonArgs {
    #[command(flatten)]
    pub algorithm_selection: AlgorithmSelection,

    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    #[arg(long = "single")]
    pub single: bool,

    #[arg(short = 's', long = "style", default_value = "json")]
    pub style: String,

    #[arg(value_name = "dir")]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Args)]
pub struct AlgorithmSelection {
    #[arg(short = '1', long = "sha-1")]
    pub sha1: bool,

    #[arg(short = '2', long = "sha-256")]
    pub sha256: bool,

    #[arg(long = "sha-384")]
    pub sha384: bool,

    #[arg(long = "sha-512")]
    pub sha512: bool,

    #[arg(long = "md5")]
    pub md5: bool,

    #[arg(long = "md2")]
    pub md2: bool,

    #[arg(short = 'a', long = "all")]
    pub all: bool,
}
