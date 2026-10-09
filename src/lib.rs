pub mod app;
pub mod cli;
pub mod error;
pub mod hashing;
pub mod model;
pub mod persistence;
pub mod report;
pub mod style;
pub mod util;
pub mod verify;

pub const BLOCK_READ_SIZE: usize = 1024 * 1024;
pub const DEFAULT_ALGORITHM: &str = "sha-256";
pub const VERSION: &str = "1.2-rust";
