// SPDX-License-Identifier: EUPL-1.2
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub enum PersistenceStyle {
    Json,
    OldStyle,
    Xml,
}

impl PersistenceStyle {
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "json" => Ok(Self::Json),
            "old" => Ok(Self::OldStyle),
            "xml" => Ok(Self::Xml),
            _ => Err(AppError::UnsupportedStyleOption(value.to_string())),
        }
    }
}
