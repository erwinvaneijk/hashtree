// SPDX-License-Identifier: EUPL-1.2
//! CASE JSON-LD serialization
//!
//! Handles conversion of CASE objects to JSON-LD format for export.

use serde_json::{Map, Value, json};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use super::CaseBundle;
use super::ontology::{CaseAction, CaseFile, CaseHash, CasePerson, CaseTool};
use crate::error::AppResult;

/// Format a SystemTime as ISO 8601 with timezone
fn format_time(time: SystemTime) -> String {
    let datetime = chrono::DateTime::<chrono::Local>::from(time);
    datetime.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string()
}

/// Serialize a CaseHash to JSON-LD object
pub fn serialize_hash(hash: &CaseHash) -> Value {
    json!({
        "types:hashMethod": hash.hash_method,
        "types:hashValue": hash.hash_value
    })
}

/// Serialize a CaseFile to JSON-LD object
pub fn serialize_file(file: &CaseFile) -> Value {
    let mut obj = Map::new();

    obj.insert("@id".to_string(), Value::String(file.id.clone()));
    obj.insert(
        "@type".to_string(),
        Value::String("observable:File".to_string()),
    );
    obj.insert(
        "observable:filePath".to_string(),
        Value::String(file.file_path.clone()),
    );
    obj.insert(
        "observable:fileName".to_string(),
        Value::String(file.file_name.clone()),
    );

    if let Some(modified) = file.modified_time {
        obj.insert(
            "observable:modifiedTime".to_string(),
            Value::String(format_time(modified)),
        );
    }

    if let Some(accessed) = file.accessed_time {
        obj.insert(
            "observable:accessedTime".to_string(),
            Value::String(format_time(accessed)),
        );
    }

    if let Some(created) = file.creation_time {
        obj.insert(
            "observable:creationTime".to_string(),
            Value::String(format_time(created)),
        );
    }

    if !file.hashes.is_empty() {
        let hashes: Vec<Value> = file.hashes.iter().map(serialize_hash).collect();
        obj.insert("observable:hashes".to_string(), Value::Array(hashes));
    }

    Value::Object(obj)
}

/// Serialize a CaseTool to JSON-LD object
pub fn serialize_tool(tool: &CaseTool) -> Value {
    json!({
        "@id": tool.id,
        "@type": "tool:AnalyticTool",
        "tool:name": tool.name,
        "tool:version": tool.version,
        "tool:toolType": tool.tool_type
    })
}

/// Serialize a CaseAction to JSON-LD object
pub fn serialize_action(action: &CaseAction) -> Value {
    let mut obj = Map::new();

    obj.insert("@id".to_string(), Value::String(action.id.clone()));
    obj.insert(
        "@type".to_string(),
        Value::String("investigation:InvestigativeAction".to_string()),
    );
    obj.insert(
        "investigation:performer".to_string(),
        Value::String("case:identity-operator".to_string()),
    );
    obj.insert(
        "action:startTime".to_string(),
        Value::String(format_time(action.start_time)),
    );
    obj.insert(
        "action:instrument".to_string(),
        json!({"@id": action.instrument_id}),
    );
    obj.insert(
        "action:result".to_string(),
        json!({"@id": action.result_id}),
    );

    Value::Object(obj)
}

/// Serialize a CasePerson to JSON-LD object
pub fn serialize_person(person: &CasePerson) -> Value {
    let mut obj = Map::new();

    obj.insert("@id".to_string(), Value::String(person.id.clone()));
    obj.insert(
        "@type".to_string(),
        Value::String("identity:Person".to_string()),
    );
    obj.insert("core:name".to_string(), Value::String(person.name.clone()));

    if let Some(given_name) = &person.given_name {
        obj.insert(
            "identity:givenName".to_string(),
            Value::String(given_name.clone()),
        );
    }

    Value::Object(obj)
}

/// Serialize a CASE bundle with file references to JSON-LD object
pub fn serialize_bundle(bundle_id: &str, file_ids: &[String]) -> Value {
    let facets: Vec<Value> = file_ids.iter().map(|id| json!({"@id": id})).collect();

    json!({
        "@id": bundle_id,
        "@type": "core:Bundle",
        "core:hasFacet": facets
    })
}

/// Write a CASE bundle to a file in JSON-LD format
pub fn serialize_case(path: &Path, bundle: &CaseBundle, pretty: bool) -> AppResult<()> {
    let json_value = bundle.to_json();
    let mut file = File::create(path)?;

    let json_str = if pretty {
        serde_json::to_string_pretty(&json_value)?
    } else {
        serde_json::to_string(&json_value)?
    };

    file.write_all(json_str.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_hash_includes_algorithm_and_value() {
        let hash = CaseHash::new("SHA-256", "abc123");
        let json = serialize_hash(&hash);

        assert_eq!(
            json.get("types:hashMethod").and_then(Value::as_str),
            Some("SHA-256")
        );
        assert_eq!(
            json.get("types:hashValue").and_then(Value::as_str),
            Some("abc123")
        );
    }

    #[test]
    fn serialize_file_includes_required_fields() {
        let file = CaseFile::new("file-1", "/path/to/file.txt", "file.txt");
        let json = serialize_file(&file);

        assert_eq!(json.get("@id").and_then(Value::as_str), Some("file-1"));
        assert_eq!(
            json.get("@type").and_then(Value::as_str),
            Some("observable:File")
        );
        assert_eq!(
            json.get("observable:filePath").and_then(Value::as_str),
            Some("/path/to/file.txt")
        );
        assert_eq!(
            json.get("observable:fileName").and_then(Value::as_str),
            Some("file.txt")
        );
    }

    #[test]
    fn serialize_tool_includes_all_fields() {
        let tool = CaseTool::new("tool-1", "hashtree", "1.0.0", "hash-calculator");
        let json = serialize_tool(&tool);

        assert_eq!(json.get("@id").and_then(Value::as_str), Some("tool-1"));
        assert_eq!(
            json.get("@type").and_then(Value::as_str),
            Some("tool:AnalyticTool")
        );
        assert_eq!(
            json.get("tool:name").and_then(Value::as_str),
            Some("hashtree")
        );
        assert_eq!(
            json.get("tool:version").and_then(Value::as_str),
            Some("1.0.0")
        );
    }

    #[test]
    fn serialize_person_includes_name() {
        let person = CasePerson::new("person-1", "John Doe").with_given_name("John");
        let json = serialize_person(&person);

        assert_eq!(json.get("@id").and_then(Value::as_str), Some("person-1"));
        assert_eq!(
            json.get("@type").and_then(Value::as_str),
            Some("identity:Person")
        );
        assert_eq!(
            json.get("core:name").and_then(Value::as_str),
            Some("John Doe")
        );
        assert_eq!(
            json.get("identity:givenName").and_then(Value::as_str),
            Some("John")
        );
    }
}
