// SPDX-License-Identifier: EUPL-1.2
//! CASE ontology data structures
//!
//! Defines the core CASE (Cyber-investigation Analysis Standard Expression) types
//! used to represent digital forensic information.

use std::time::SystemTime;

/// Create a new, unique IRI for a CASE object
pub fn new_id() -> String {
    format!("urn:uuid:{}", uuid::Uuid::new_v4())
}

/// Represents a hash in CASE format
#[derive(Debug, Clone)]
pub struct CaseHash {
    /// Hash algorithm name (e.g., "MD5", "SHA-256", "SHA-512")
    pub hash_method: String,
    /// Hexadecimal digest value (CASE types it as xsd:hexBinary)
    pub hash_value: String,
}

impl CaseHash {
    pub fn new(hash_method: impl Into<String>, hash_value: impl Into<String>) -> Self {
        Self {
            hash_method: hash_method.into(),
            hash_value: hash_value.into(),
        }
    }
}

/// Represents a file in CASE format
#[derive(Debug, Clone)]
pub struct CaseFile {
    /// Unique identifier for this file object
    pub id: String,
    /// Full path to the file
    pub file_path: String,
    /// File name (basename)
    pub file_name: String,
    /// Last modified time
    pub modified_time: Option<SystemTime>,
    /// Last accessed time
    pub accessed_time: Option<SystemTime>,
    /// Creation/birth time
    pub creation_time: Option<SystemTime>,
    /// Collection of hashes for this file
    pub hashes: Vec<CaseHash>,
}

impl CaseFile {
    pub fn new(
        id: impl Into<String>,
        file_path: impl Into<String>,
        file_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            file_path: file_path.into(),
            file_name: file_name.into(),
            modified_time: None,
            accessed_time: None,
            creation_time: None,
            hashes: Vec::new(),
        }
    }

    pub fn with_hash(mut self, hash: CaseHash) -> Self {
        self.hashes.push(hash);
        self
    }
}

/// Represents a tool in CASE format
#[derive(Debug, Clone)]
pub struct CaseTool {
    /// Unique identifier
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool version
    pub version: String,
    /// Tool type (e.g., "hash-calculator")
    pub tool_type: String,
}

impl CaseTool {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        tool_type: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            tool_type: tool_type.into(),
        }
    }
}

/// Represents an investigative action in CASE format
#[derive(Debug, Clone)]
pub struct CaseAction {
    /// Unique identifier
    pub id: String,
    /// ID of the person who performed the action
    pub performer_id: String,
    /// When the action started
    pub start_time: SystemTime,
    /// ID of the instrument (tool) used
    pub instrument_id: String,
    /// IDs of the results (the hashed files)
    pub result_ids: Vec<String>,
}

impl CaseAction {
    pub fn new(
        id: impl Into<String>,
        performer_id: impl Into<String>,
        start_time: SystemTime,
        instrument_id: impl Into<String>,
        result_ids: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            performer_id: performer_id.into(),
            start_time,
            instrument_id: instrument_id.into(),
            result_ids,
        }
    }
}

/// Represents a person (examiner/operator) in CASE format
#[derive(Debug, Clone)]
pub struct CasePerson {
    /// Unique identifier
    pub id: String,
    /// Full name
    pub name: String,
    /// Given name (optional)
    pub given_name: Option<String>,
}

impl CasePerson {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            given_name: None,
        }
    }

    pub fn with_given_name(mut self, given_name: impl Into<String>) -> Self {
        self.given_name = Some(given_name.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_creates_unique_uuid_iris() {
        let first = new_id();
        let second = new_id();

        assert!(first.starts_with("urn:uuid:"));
        assert_eq!(first.len(), "urn:uuid:".len() + 36);
        assert_ne!(first, second);
    }
}
