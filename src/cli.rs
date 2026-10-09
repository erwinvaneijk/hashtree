use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "hashtree",
    about = "Creating a list of digests for files and/or directories"
)]
pub struct Cli {
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

    #[arg(short = 'n', long = "ignorecase")]
    pub ignore_case: bool,

    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    #[arg(short = 'f', long = "force")]
    pub force: bool,

    #[arg(long = "single")]
    pub single: bool,

    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    #[arg(short = 'i', long = "input")]
    pub input: Option<PathBuf>,

    #[arg(short = 's', long = "style", default_value = "json")]
    pub style: String,

    #[arg(value_name = "dir")]
    pub paths: Vec<PathBuf>,
}
