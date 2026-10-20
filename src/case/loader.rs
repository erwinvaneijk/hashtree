// SPDX-License-Identifier: EUPL-1.2
//! Load and parse CASE JSON-LD files
//!
//! Converts CASE JSON-LD format back to hashtree's internal DirHasherResult
//! format for verification and comparison operations. Files in the CASE/UCO
//! structure written since hashtree 4.0 (and by jHashtools) are read strictly;
//! manifests written by earlier releases, which used other namespaces and kept
//! the hashes directly on the file, are still accepted.

use std::fs::File;
use std::path::Path;

use serde_json::Value;

use super::bundle::UCO_CONTEXT;
use crate::error::{AppError, AppResult};
use crate::model::{DigestResult, DirHasherResult};

/// Namespace of the `observable` prefix in manifests written before hashtree 4.0
const LEGACY_OBSERVABLE_NAMESPACE: &str = "https://ontology.caseontology.org/observable/";

/// Prefixes whose namespaces must match for the keys read here to mean what they say
const PREFIXES_READ: [&str; 3] = ["core", "observable", "types"];

fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidCaseFile(message.into())
}

/// Load a CASE JSON-LD file and convert to DirHasherResult
pub fn load_case(path: &Path) -> AppResult<DirHasherResult> {
    let file = File::open(path)?;
    let json: Value = serde_json::from_reader(file)?;

    // Validate JSON-LD structure
    if !json.is_object() {
        return Err(invalid("CASE file must be a JSON object"));
    }

    // Get @graph array
    let graph = json
        .get("@graph")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("CASE file must have @graph array"))?;

    let context = json.get("@context");
    let observable_namespace = context
        .and_then(|context| context.get("observable"))
        .and_then(Value::as_str);
    if observable_namespace == Some(LEGACY_OBSERVABLE_NAMESPACE) {
        return Ok(load_legacy_graph(graph));
    }

    check_context(context)?;

    let mut result = DirHasherResult::default();
    for node in graph {
        if has_type(node, "observable:File")
            && let Some((file_path, digest_result)) = read_file(node)?
        {
            result.content.insert(file_path, digest_result);
        }
    }

    Ok(result)
}

/// Check that the prefixes read here map to the CASE/UCO namespaces
fn check_context(context: Option<&Value>) -> AppResult<()> {
    let context = context
        .filter(|context| context.is_object())
        .ok_or_else(|| invalid("CASE file must have an @context object"))?;

    for prefix in PREFIXES_READ {
        let expected = UCO_CONTEXT
            .iter()
            .find(|(known_prefix, _)| *known_prefix == prefix)
            .map(|(_, namespace)| *namespace)
            .expect("every prefix read is part of the UCO context");
        if context.get(prefix).and_then(Value::as_str) != Some(expected) {
            return Err(invalid(format!(
                "The @context of the CASE file must map '{prefix}' to {expected}"
            )));
        }
    }
    Ok(())
}

/// Whether the node's @type is, or contains, the given type
fn has_type(node: &Value, wanted: &str) -> bool {
    match node.get("@type") {
        Some(Value::String(node_type)) => node_type == wanted,
        Some(Value::Array(node_types)) => node_types
            .iter()
            .any(|node_type| node_type.as_str() == Some(wanted)),
        _ => false,
    }
}

/// JSON-LD allows a single value instead of an array with one element
fn elements(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(values)) => values.iter().collect(),
        Some(value @ Value::Object(_)) => vec![value],
        _ => Vec::new(),
    }
}

/// Read a non-empty string field, or fail naming the field and its context
fn required_text<'a>(node: &'a Value, field: &str, context: &str) -> AppResult<&'a str> {
    node.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(format!("Missing {field} for {context}")))
}

/// Read the path and hashes of an observable:File; a file without hashes is skipped
fn read_file(file: &Value) -> AppResult<Option<(String, DigestResult)>> {
    let file_id = file
        .get("@id")
        .and_then(Value::as_str)
        .unwrap_or("(no @id)");
    let facets = elements(file.get("core:hasFacet"));

    let file_facet = facets
        .iter()
        .find(|facet| has_type(facet, "observable:FileFacet"))
        .ok_or_else(|| {
            invalid(format!(
                "observable:File {file_id} has no observable:FileFacet"
            ))
        })?;
    let file_path = required_text(file_facet, "observable:filePath", file_id)?;

    let mut digest_result = DigestResult::default();
    for facet in facets
        .iter()
        .filter(|facet| has_type(facet, "observable:ContentDataFacet"))
    {
        for hash in elements(facet.get("observable:hash")) {
            let hash_method = required_text(hash, "types:hashMethod", file_path)?;
            let hash_value = hash.get("types:hashValue");
            let hex_value = hash_value
                .and_then(|value| value.get("@value"))
                .or(hash_value)
                .and_then(Value::as_str)
                .ok_or_else(|| invalid(format!("Missing types:hashValue for {file_path}")))?;
            if hex::decode(hex_value).is_err() {
                return Err(invalid(format!(
                    "Invalid {hash_method} hash value for {file_path}"
                )));
            }
            digest_result.add_digest_name(hash_method, hex_value);
        }
    }

    if digest_result.digests.is_empty() {
        return Ok(None);
    }
    Ok(Some((file_path.to_string(), digest_result)))
}

/// Read the graph of a manifest written before hashtree 4.0
fn load_legacy_graph(graph: &[Value]) -> DirHasherResult {
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
        let mut digest_result = DigestResult::default();

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

    result
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

    const UCO_TEST_CONTEXT: &str = r#""@context": {
        "core": "https://ontology.unifiedcyberontology.org/uco/core/",
        "observable": "https://ontology.unifiedcyberontology.org/uco/observable/",
        "types": "https://ontology.unifiedcyberontology.org/uco/types/"
    }"#;

    const SHA256_OF_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn load_json(json_content: &str) -> AppResult<DirHasherResult> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.jsonld");
        std::fs::write(&file_path, json_content)?;
        load_case(&file_path)
    }

    fn uco_file(facets: &str) -> String {
        format!(
            r#"{{{UCO_TEST_CONTEXT}, "@graph": [{{"@type": "observable:File", "core:hasFacet": [{facets}]}}]}}"#
        )
    }

    #[test]
    fn load_case_reads_uco_facets() -> AppResult<()> {
        let json_content = format!(
            r#"{{{UCO_TEST_CONTEXT}, "@graph": [
                {{"@type": "tool:AnalyticTool", "core:name": "jHashtools"}},
                {{
                    "@id": "urn:uuid:00000000-0000-4000-8000-000000000001",
                    "@type": "observable:File",
                    "core:hasFacet": [
                        {{"@type": "observable:FileFacet", "observable:filePath": "dir/abc.txt"}},
                        {{
                            "@type": "observable:ContentDataFacet",
                            "observable:hash": [{{
                                "@type": "types:Hash",
                                "types:hashMethod": "SHA256",
                                "types:hashValue": {{"@type": "xsd:hexBinary", "@value": "{SHA256_OF_ABC}"}}
                            }}]
                        }}
                    ]
                }}
            ]}}"#
        );

        let result = load_json(&json_content)?;

        assert_eq!(result.content.len(), 1);
        let digests = result
            .content
            .get("dir/abc.txt")
            .expect("file should be present");
        assert_eq!(
            digests.digests.get(&crate::algorithm::Algorithm::Sha256),
            Some(&SHA256_OF_ABC.to_string())
        );
        Ok(())
    }

    #[test]
    fn load_case_accepts_single_values_and_plain_hash_values() -> AppResult<()> {
        let json_content = format!(
            r#"{{{UCO_TEST_CONTEXT}, "@graph": [
                {{
                    "@type": ["observable:File"],
                    "core:hasFacet": [
                        {{"@type": "observable:FileFacet", "observable:filePath": "dir/abc.txt"}},
                        {{
                            "@type": "observable:ContentDataFacet",
                            "observable:hash": {{"types:hashMethod": "sha256", "types:hashValue": "{SHA256_OF_ABC}"}}
                        }}
                    ]
                }},
                {{
                    "@type": "observable:File",
                    "core:hasFacet": {{"@type": "observable:FileFacet", "observable:filePath": "no-hashes.txt"}}
                }}
            ]}}"#
        );

        let result = load_json(&json_content)?;

        assert_eq!(result.content.len(), 1);
        assert!(result.content.contains_key("dir/abc.txt"));
        Ok(())
    }

    #[test]
    fn load_case_rejects_malformed_uco_documents() {
        let file_facet = r#"{"@type": "observable:FileFacet", "observable:filePath": "a.txt"}"#;
        let hashes = |hash: &str| {
            format!(
                r#"{file_facet}, {{"@type": "observable:ContentDataFacet", "observable:hash": [{hash}]}}"#
            )
        };
        let cases = [
            (
                r#"{"@graph": []}"#.to_string(),
                "must have an @context object",
            ),
            (
                r#"{"@context": {"observable": "https://example.org/"}, "@graph": []}"#.to_string(),
                "must map 'core' to",
            ),
            (uco_file(""), "has no observable:FileFacet"),
            (
                uco_file(r#"{"@type": "observable:FileFacet"}"#),
                "Missing observable:filePath",
            ),
            (
                uco_file(&hashes(r#"{"types:hashValue": "00"}"#)),
                "Missing types:hashMethod for a.txt",
            ),
            (
                uco_file(&hashes(r#"{"types:hashMethod": "MD5"}"#)),
                "Missing types:hashValue for a.txt",
            ),
            (
                uco_file(&hashes(
                    r#"{"types:hashMethod": "MD5", "types:hashValue": "zz"}"#,
                )),
                "Invalid MD5 hash value for a.txt",
            ),
        ];

        for (json_content, expected_message) in cases {
            let error = load_json(&json_content).expect_err(&json_content);
            assert!(
                matches!(&error, AppError::InvalidCaseFile(message) if message.contains(expected_message)),
                "{json_content}: {error}"
            );
        }
    }
}
