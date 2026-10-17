// SPDX-License-Identifier: EUPL-1.2
//! Load and parse CASE JSON-LD files
//!
//! Converts CASE JSON-LD format back to hashtree's internal DirHasherResult
//! format for verification and comparison operations.

use std::fs::File;
use std::path::Path;

use crate::error::AppResult;
use crate::model::DirHasherResult;

/// Load a CASE JSON-LD file and convert to DirHasherResult
pub fn load_case(path: &Path) -> AppResult<DirHasherResult> {
    let file = File::open(path)?;
    let json: serde_json::Value = serde_json::from_reader(file)?;

    // Validate JSON-LD structure
    if !json.is_object() {
        return Err(crate::error::AppError::InvalidCaseFile(
            "CASE file must be a JSON object".to_string(),
        ));
    }

    // Get @graph array
    let graph = json
        .get("@graph")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            crate::error::AppError::InvalidCaseFile("CASE file must have @graph array".to_string())
        })?;

    let mut result = DirHasherResult::default();

    // Extract files from graph
    for obj in graph {
        if !obj.is_object() {
            continue;
        }

        // Look for observable:File objects
        let obj_type = obj.get("@type").and_then(|v| v.as_str()).unwrap_or("");

        if obj_type != "observable:File" {
            continue;
        }

        // Extract file metadata
        let file_path = obj
            .get("observable:filePath")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if file_path.is_empty() {
            continue;
        }

        // Extract hashes
        let mut digest_result = crate::model::DigestResult::default();

        if let Some(hashes) = obj.get("observable:hashes").and_then(|v| v.as_array()) {
            for hash_obj in hashes {
                if !hash_obj.is_object() {
                    continue;
                }

                let hash_method = hash_obj
                    .get("types:hashMethod")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let hash_value = hash_obj
                    .get("types:hashValue")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if !hash_method.is_empty() && !hash_value.is_empty() {
                    digest_result.add_digest_name(hash_method, hash_value);
                }
            }
        }

        if !digest_result.digests.is_empty() {
            result.content.insert(file_path, digest_result);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_case_from_valid_jsonld() -> AppResult<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");

        let json_content = r#"{
            "@context": {"observable": "https://ontology.caseontology.org/observable/", "types": "https://ontology.caseontology.org/types/"},
            "@graph": [
                {
                    "@type": "observable:File",
                    "observable:filePath": "/path/to/file.txt",
                    "observable:fileName": "file.txt",
                    "observable:hashes": [
                        {"types:hashMethod": "SHA-256", "types:hashValue": "abc123"},
                        {"types:hashMethod": "MD5", "types:hashValue": "def456"}
                    ]
                }
            ]
        }"#;

        std::fs::write(&file_path, json_content)?;

        let result = load_case(&file_path)?;

        assert_eq!(result.content.len(), 1);
        let digests = result
            .content
            .get("/path/to/file.txt")
            .expect("file should be present");
        assert_eq!(digests.digests.len(), 2);

        Ok(())
    }

    #[test]
    fn load_case_handles_missing_file() {
        let result = load_case(Path::new("/nonexistent/path.jsonld"));
        assert!(result.is_err());
    }

    #[test]
    fn load_case_handles_invalid_json() -> AppResult<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");

        std::fs::write(&file_path, "not valid json")?;

        let result = load_case(&file_path);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn load_case_handles_missing_graph() -> AppResult<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");

        let json_content = r#"{"@context": {}}"#;
        std::fs::write(&file_path, json_content)?;

        let result = load_case(&file_path);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn load_case_skips_non_file_objects() -> AppResult<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");

        let json_content = r#"{
            "@context": {"observable": "https://ontology.caseontology.org/observable/", "types": "https://ontology.caseontology.org/types/"},
            "@graph": [
                {"@type": "tool:AnalyticTool", "tool:name": "hashtree"},
                {"@type": "identity:Person", "core:name": "user"},
                {
                    "@type": "observable:File",
                    "observable:filePath": "/path/to/file.txt",
                    "observable:hashes": [
                        {"types:hashMethod": "SHA-256", "types:hashValue": "abc123"}
                    ]
                }
            ]
        }"#;

        std::fs::write(&file_path, json_content)?;

        let result = load_case(&file_path)?;
        assert_eq!(result.content.len(), 1);

        Ok(())
    }

    #[test]
    fn load_case_extracts_multiple_files() -> AppResult<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");

        let json_content = r#"{
            "@context": {"observable": "https://ontology.caseontology.org/observable/", "types": "https://ontology.caseontology.org/types/"},
            "@graph": [
                {
                    "@type": "observable:File",
                    "observable:filePath": "/path/to/file1.txt",
                    "observable:hashes": [{"types:hashMethod": "SHA-256", "types:hashValue": "hash1"}]
                },
                {
                    "@type": "observable:File",
                    "observable:filePath": "/path/to/file2.txt",
                    "observable:hashes": [{"types:hashMethod": "SHA-256", "types:hashValue": "hash2"}]
                }
            ]
        }"#;

        std::fs::write(&file_path, json_content)?;

        let result = load_case(&file_path)?;
        assert_eq!(result.content.len(), 2);

        Ok(())
    }

    #[test]
    fn load_case_handles_empty_file_path() -> AppResult<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");

        let json_content = r#"{
            "@context": {"observable": "https://ontology.caseontology.org/observable/", "types": "https://ontology.caseontology.org/types/"},
            "@graph": [
                {
                    "@type": "observable:File",
                    "observable:filePath": "",
                    "observable:hashes": [{"types:hashMethod": "SHA-256", "types:hashValue": "abc123"}]
                }
            ]
        }"#;

        std::fs::write(&file_path, json_content)?;

        let result = load_case(&file_path)?;
        assert_eq!(result.content.len(), 0);

        Ok(())
    }
}
