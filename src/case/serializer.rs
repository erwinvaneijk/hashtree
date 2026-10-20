// SPDX-License-Identifier: EUPL-1.2
//! CASE JSON-LD serialization
//!
//! Handles conversion of CASE objects to JSON-LD format for export.

use chrono::SecondsFormat;
use serde_json::{Map, Value, json};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use super::CaseBundle;
use super::ontology::{CaseAction, CaseFile, CaseHash, CasePerson, CaseTool, new_id};
use crate::error::AppResult;

/// Create a JSON-LD literal of the given XSD datatype
fn typed_literal(datatype: &str, value: &str) -> Value {
    json!({"@type": datatype, "@value": value})
}

/// Format a SystemTime as an xsd:dateTime literal (ISO 8601 with timezone)
fn format_time(time: SystemTime) -> Value {
    let datetime = chrono::DateTime::<chrono::Local>::from(time);
    typed_literal(
        "xsd:dateTime",
        &datetime.to_rfc3339_opts(SecondsFormat::Millis, false),
    )
}

/// Serialize a CaseHash to a JSON-LD types:Hash object
pub fn serialize_hash(hash: &CaseHash) -> Value {
    json!({
        "@id": new_id(),
        "@type": "types:Hash",
        "types:hashMethod": hash.hash_method,
        "types:hashValue": typed_literal("xsd:hexBinary", &hash.hash_value)
    })
}

/// Serialize a CaseFile to a JSON-LD observable:File object with a FileFacet
/// and, when it has hashes, a ContentDataFacet
pub fn serialize_file(file: &CaseFile) -> Value {
    let mut file_facet = Map::new();

    file_facet.insert("@id".to_string(), Value::String(new_id()));
    file_facet.insert(
        "@type".to_string(),
        Value::String("observable:FileFacet".to_string()),
    );
    file_facet.insert(
        "observable:fileName".to_string(),
        Value::String(file.file_name.clone()),
    );
    file_facet.insert(
        "observable:filePath".to_string(),
        Value::String(file.file_path.clone()),
    );

    if let Some(modified) = file.modified_time {
        file_facet.insert("observable:modifiedTime".to_string(), format_time(modified));
    }

    if let Some(accessed) = file.accessed_time {
        file_facet.insert("observable:accessedTime".to_string(), format_time(accessed));
    }

    if let Some(created) = file.creation_time {
        file_facet.insert(
            "observable:observableCreatedTime".to_string(),
            format_time(created),
        );
    }

    let mut facets = vec![Value::Object(file_facet)];

    if !file.hashes.is_empty() {
        let hashes: Vec<Value> = file.hashes.iter().map(serialize_hash).collect();
        facets.push(json!({
            "@id": new_id(),
            "@type": "observable:ContentDataFacet",
            "observable:hash": hashes
        }));
    }

    json!({
        "@id": file.id,
        "@type": "observable:File",
        "core:hasFacet": facets
    })
}

/// Serialize a CaseTool to JSON-LD object
pub fn serialize_tool(tool: &CaseTool) -> Value {
    json!({
        "@id": tool.id,
        "@type": "tool:AnalyticTool",
        "core:name": tool.name,
        "tool:version": tool.version,
        "tool:toolType": tool.tool_type
    })
}

/// Serialize a CaseAction to JSON-LD object
pub fn serialize_action(action: &CaseAction) -> Value {
    let results: Vec<Value> = action
        .result_ids
        .iter()
        .map(|id| json!({"@id": id}))
        .collect();

    json!({
        "@id": action.id,
        "@type": "case-investigation:InvestigativeAction",
        "core:name": "Compute digests",
        "action:startTime": format_time(action.start_time),
        "action:performer": {"@id": action.performer_id},
        "action:instrument": {"@id": action.instrument_id},
        "action:result": results
    })
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
    fn serialize_hash_includes_algorithm_and_typed_value() {
        let hash = CaseHash::new("SHA256", "abc123");
        let json = serialize_hash(&hash);

        assert_eq!(
            json.get("@type").and_then(Value::as_str),
            Some("types:Hash")
        );
        assert!(
            json.get("@id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with("urn:uuid:"))
        );
        assert_eq!(
            json.get("types:hashMethod").and_then(Value::as_str),
            Some("SHA256")
        );
        assert_eq!(
            json.get("types:hashValue"),
            Some(&json!({"@type": "xsd:hexBinary", "@value": "abc123"}))
        );
    }

    #[test]
    fn serialize_file_puts_path_and_hashes_in_facets() {
        let file = CaseFile::new("file-1", "/path/to/file.txt", "file.txt")
            .with_hash(CaseHash::new("SHA256", "abc123"));
        let json = serialize_file(&file);

        assert_eq!(json.get("@id").and_then(Value::as_str), Some("file-1"));
        assert_eq!(
            json.get("@type").and_then(Value::as_str),
            Some("observable:File")
        );
        let facets = json
            .get("core:hasFacet")
            .and_then(Value::as_array)
            .expect("core:hasFacet should be an array");
        assert_eq!(facets.len(), 2);
        assert_eq!(
            facets[0].get("@type").and_then(Value::as_str),
            Some("observable:FileFacet")
        );
        assert_eq!(
            facets[0].get("observable:filePath").and_then(Value::as_str),
            Some("/path/to/file.txt")
        );
        assert_eq!(
            facets[0].get("observable:fileName").and_then(Value::as_str),
            Some("file.txt")
        );
        assert_eq!(
            facets[1].get("@type").and_then(Value::as_str),
            Some("observable:ContentDataFacet")
        );
        assert_eq!(
            facets[1]
                .get("observable:hash")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn serialize_file_without_hashes_has_only_a_file_facet() {
        let file = CaseFile::new("file-1", "/path/to/file.txt", "file.txt");
        let json = serialize_file(&file);

        assert_eq!(
            json.get("core:hasFacet")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn serialize_file_writes_times_as_xsd_date_time() {
        let mut file = CaseFile::new("file-1", "/path/to/file.txt", "file.txt");
        file.accessed_time = Some(SystemTime::UNIX_EPOCH);
        let json = serialize_file(&file);

        let accessed = &json["core:hasFacet"][0]["observable:accessedTime"];
        assert_eq!(
            accessed.get("@type").and_then(Value::as_str),
            Some("xsd:dateTime")
        );
        let value = accessed
            .get("@value")
            .and_then(Value::as_str)
            .expect("accessedTime should have a value");
        assert!(
            chrono::DateTime::parse_from_rfc3339(value).is_ok(),
            "{value} should be RFC 3339"
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
            json.get("core:name").and_then(Value::as_str),
            Some("hashtree")
        );
        assert_eq!(
            json.get("tool:version").and_then(Value::as_str),
            Some("1.0.0")
        );
    }

    #[test]
    fn serialize_action_refers_to_performer_tool_and_results() {
        let action = CaseAction::new(
            "action-1",
            "person-1",
            SystemTime::UNIX_EPOCH,
            "tool-1",
            vec!["file-1".to_string(), "file-2".to_string()],
        );
        let json = serialize_action(&action);

        assert_eq!(
            json.get("@type").and_then(Value::as_str),
            Some("case-investigation:InvestigativeAction")
        );
        assert_eq!(json["action:performer"], json!({"@id": "person-1"}));
        assert_eq!(json["action:instrument"], json!({"@id": "tool-1"}));
        assert_eq!(
            json["action:result"],
            json!([{"@id": "file-1"}, {"@id": "file-2"}])
        );
        assert_eq!(json["action:startTime"]["@type"], json!("xsd:dateTime"));
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
