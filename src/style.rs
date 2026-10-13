// SPDX-License-Identifier: EUPL-1.2
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub enum PersistenceStyle {
    Json,
    OldStyle,
    Xml,
    Case,
}

impl PersistenceStyle {
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "json" => Ok(Self::Json),
            "old" => Ok(Self::OldStyle),
            "xml" => Ok(Self::Xml),
            "case" | "case-json-ld" => Ok(Self::Case),
            _ => Err(AppError::UnsupportedStyleOption(value.to_string())),
        }
    }
}
