// SPDX-License-Identifier: EUPL-1.2
use std::io;
use std::path::PathBuf;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Verification failed: there are differences.")]
    VerificationFailed,

    #[error("--jobs must be greater than zero")]
    InvalidJobCount,

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

    #[error("{0}")]
    InvalidCaseFile(String),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Cannot hash output file {path}: {source}")]
    HashOutputFile { path: PathBuf, source: io::Error },

    #[error("Cannot create thread pool: {0}")]
    ThreadPool(String),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl AppError {
    /// Process exit code for this error; every kind of failure has its own code.
    ///
    /// Code 0 is success and code 2 is reserved for command-line usage errors
    /// reported by clap, so neither is returned here.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::VerificationFailed => 1,
            Self::InvalidJobCount => 3,
            Self::UnsupportedStyleOption(_) => 4,
            Self::XmlNotSupported => 5,
            Self::FileDoesNotExist(_) => 6,
            Self::OutputExists(_) => 7,
            Self::InvalidOldStyleHeader => 8,
            Self::InvalidCaseFile(_) => 9,
            Self::Json(_) => 10,
            Self::HashOutputFile { .. } => 11,
            Self::ThreadPool(_) => 12,
            Self::Io(_) => 13,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_error_kind_has_a_distinct_failure_exit_code() {
        let json_error = serde_json::from_str::<serde_json::Value>("{")
            .expect_err("truncated JSON should not parse");
        let errors = [
            AppError::VerificationFailed,
            AppError::InvalidJobCount,
            AppError::UnsupportedStyleOption("yaml".to_string()),
            AppError::XmlNotSupported,
            AppError::FileDoesNotExist(PathBuf::from("missing")),
            AppError::OutputExists(PathBuf::from("out.json")),
            AppError::InvalidOldStyleHeader,
            AppError::InvalidCaseFile("bad".to_string()),
            AppError::Json(json_error),
            AppError::HashOutputFile {
                path: PathBuf::from("out.json"),
                source: io::Error::other("denied"),
            },
            AppError::ThreadPool("no threads".to_string()),
            AppError::Io(io::Error::other("disk")),
        ];

        let codes: HashSet<u8> = errors.iter().map(AppError::exit_code).collect();

        assert_eq!(codes.len(), errors.len(), "exit codes must be unique");
        assert!(!codes.contains(&0), "0 is reserved for success");
        assert!(!codes.contains(&2), "2 is reserved for usage errors");
    }

    #[test]
    fn verification_failure_exits_with_one() {
        assert_eq!(AppError::VerificationFailed.exit_code(), 1);
    }
}
