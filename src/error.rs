use std::io;
use std::path::PathBuf;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),

    #[error("We do not support option {0}")]
    UnsupportedStyleOption(String),

    #[error("XML is not yet supported.")]
    XmlNotSupported,

    #[error("File {0} does not exist")]
    FileDoesNotExist(PathBuf),

    #[error("File {0} exists or not forced to be overwritten. Stop.")]
    OutputExists(PathBuf),

    #[error("The first line is not ok.")]
    InvalidOldStyleHeader,

    #[error("Cannot hash output file {path}: {source}")]
    HashOutputFile { path: PathBuf, source: io::Error },

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
