// SPDX-License-Identifier: EUPL-1.2
//! Convert hashtree results to CASE format
//!
//! Transforms the internal DirHasherResult format into CASE ontology objects.

use std::time::SystemTime;

use crate::algorithm::Algorithm;
use crate::error::AppResult;
use crate::model::DirHasherResult;

use super::bundle::CaseBundle;
use super::ontology::{CaseAction, CaseFile, CaseHash, CasePerson, CaseTool, new_id};
use super::serializer::{serialize_action, serialize_file, serialize_person, serialize_tool};

/// Name of the algorithm in the UCO HashNameVocab. The vocabulary is open:
/// RIPEMD-160 is not part of it, so its name only follows the same pattern.
pub fn case_hash_method(algorithm: &Algorithm) -> String {
    match algorithm {
        Algorithm::Md5 => "MD5".to_string(),
        Algorithm::Sha1 => "SHA1".to_string(),
        Algorithm::Sha256 => "SHA256".to_string(),
        Algorithm::Sha384 => "SHA384".to_string(),
        Algorithm::Sha512 => "SHA512".to_string(),
        Algorithm::Sha3_256 => "SHA3-256".to_string(),
        Algorithm::Sha3_384 => "SHA3-384".to_string(),
        Algorithm::Sha3_512 => "SHA3-512".to_string(),
        Algorithm::Ripemd160 => "RIPEMD160".to_string(),
        Algorithm::Other(value) => value.to_ascii_uppercase(),
    }
}

/// Convert a DirHasherResult to a CASE Bundle. CASE types hash values as
/// xsd:hexBinary, so the digests are always written in hexadecimal.
pub fn convert_to_case(result: &DirHasherResult) -> AppResult<CaseBundle> {
    let mut bundle = CaseBundle::new();

    // Get current user for performer
    let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    // Create tool object
    let tool = CaseTool::new(
        new_id(),
        "hashtree",
        crate::VERSION.to_string(),
        "hash-calculator",
    );
    bundle.add_object(serialize_tool(&tool));

    // Create person object
    let person = CasePerson::new(new_id(), &username);
    bundle.add_object(serialize_person(&person));

    // Collect file IDs for the investigative action
    let mut file_ids = Vec::new();

    // Convert each file
    for (file_path, digest_result) in &result.content {
        let file_id = new_id();
        file_ids.push(file_id.clone());

        // Extract filename from path
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Create file object
        let mut case_file = CaseFile::new(&file_id, file_path.clone(), file_name);

        // Add hashes
        for (algorithm, hex_digest) in &digest_result.digests {
            let hash = CaseHash::new(case_hash_method(algorithm), hex_digest.clone());
            case_file.hashes.push(hash);
        }

        bundle.add_object(serialize_file(&case_file));
    }

    // Create investigative action
    let action = CaseAction::new(
        new_id(),
        person.id.clone(),
        SystemTime::now(),
        tool.id.clone(),
        file_ids,
    );
    bundle.add_object(serialize_action(&action));

    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DigestResult;

    #[test]
    fn convert_to_case_creates_bundle_with_objects() -> AppResult<()> {
        let mut result = DirHasherResult::default();
        let mut digests = DigestResult::default();
        digests.add_digest(Algorithm::Sha256, "abc123");
        result.content.insert("test.txt".to_string(), digests);

        let bundle = convert_to_case(&result)?;

        // Should have: tool, person, file, action
        assert_eq!(bundle.graph.len(), 4);

        Ok(())
    }

    #[test]
    fn convert_to_case_includes_tool_object() -> AppResult<()> {
        let result = DirHasherResult::default();
        let bundle = convert_to_case(&result)?;

        let tool_found = bundle
            .graph
            .iter()
            .any(|obj| obj.get("@type").and_then(|t| t.as_str()) == Some("tool:AnalyticTool"));

        assert!(tool_found, "Tool object should be present");
        Ok(())
    }

    #[test]
    fn convert_to_case_includes_person_object() -> AppResult<()> {
        let result = DirHasherResult::default();
        let bundle = convert_to_case(&result)?;

        let person_found = bundle
            .graph
            .iter()
            .any(|obj| obj.get("@type").and_then(|t| t.as_str()) == Some("identity:Person"));

        assert!(person_found, "Person object should be present");
        Ok(())
    }

    #[test]
    fn convert_to_case_includes_file_objects_for_each_input() -> AppResult<()> {
        let mut result = DirHasherResult::default();
        let mut digests = DigestResult::default();
        digests.add_digest(Algorithm::Sha256, "abc123");
        result
            .content
            .insert("file1.txt".to_string(), digests.clone());
        result.content.insert("file2.txt".to_string(), digests);

        let bundle = convert_to_case(&result)?;

        let file_count = bundle
            .graph
            .iter()
            .filter(|obj| obj.get("@type").and_then(|t| t.as_str()) == Some("observable:File"))
            .count();

        assert_eq!(file_count, 2, "Should have 2 file objects");
        Ok(())
    }

    #[test]
    fn convert_to_case_includes_action_object() -> AppResult<()> {
        let result = DirHasherResult::default();
        let bundle = convert_to_case(&result)?;

        let action_found = bundle.graph.iter().any(|obj| {
            obj.get("@type").and_then(|t| t.as_str())
                == Some("case-investigation:InvestigativeAction")
        });

        assert!(action_found, "Action object should be present");
        Ok(())
    }

    #[test]
    fn convert_to_case_action_refers_to_every_file() -> AppResult<()> {
        let mut result = DirHasherResult::default();
        let mut digests = DigestResult::default();
        digests.add_digest(Algorithm::Sha256, "abc123");
        result
            .content
            .insert("file1.txt".to_string(), digests.clone());
        result.content.insert("file2.txt".to_string(), digests);

        let bundle = convert_to_case(&result)?;

        let file_ids: Vec<&serde_json::Value> = bundle
            .graph
            .iter()
            .filter(|obj| obj.get("@type").and_then(|t| t.as_str()) == Some("observable:File"))
            .filter_map(|obj| obj.get("@id"))
            .collect();
        let action = bundle
            .graph
            .iter()
            .find(|obj| {
                obj.get("@type").and_then(|t| t.as_str())
                    == Some("case-investigation:InvestigativeAction")
            })
            .expect("action should be present");
        let result_ids: Vec<&serde_json::Value> = action["action:result"]
            .as_array()
            .expect("action:result should be an array")
            .iter()
            .filter_map(|reference| reference.get("@id"))
            .collect();

        assert_eq!(result_ids, file_ids);
        Ok(())
    }

    #[test]
    fn convert_to_case_writes_hex_digests_with_uco_hash_names() -> AppResult<()> {
        let hex_digest = "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e";

        let mut result = DirHasherResult::default();
        let mut digests = DigestResult::default();
        digests.add_digest(Algorithm::Sha256, hex_digest);
        result.content.insert("test.txt".to_string(), digests);

        let bundle = convert_to_case(&result)?;
        let file = bundle
            .graph
            .iter()
            .find(|obj| obj.get("@type").and_then(|t| t.as_str()) == Some("observable:File"))
            .expect("file should be present");
        let hash = &file["core:hasFacet"][1]["observable:hash"][0];

        assert_eq!(hash["types:hashMethod"], serde_json::json!("SHA256"));
        assert_eq!(
            hash["types:hashValue"]["@value"],
            serde_json::json!(hex_digest)
        );
        Ok(())
    }

    #[test]
    fn case_hash_method_uses_hash_name_vocabulary() {
        assert_eq!(case_hash_method(&Algorithm::Md5), "MD5");
        assert_eq!(case_hash_method(&Algorithm::Sha1), "SHA1");
        assert_eq!(case_hash_method(&Algorithm::Sha256), "SHA256");
        assert_eq!(case_hash_method(&Algorithm::Sha384), "SHA384");
        assert_eq!(case_hash_method(&Algorithm::Sha512), "SHA512");
        assert_eq!(case_hash_method(&Algorithm::Sha3_256), "SHA3-256");
        assert_eq!(case_hash_method(&Algorithm::Sha3_384), "SHA3-384");
        assert_eq!(case_hash_method(&Algorithm::Sha3_512), "SHA3-512");
        assert_eq!(case_hash_method(&Algorithm::Ripemd160), "RIPEMD160");
    }

    #[test]
    fn case_hash_methods_are_read_back_as_the_same_algorithm() {
        for algorithm in [
            Algorithm::Md5,
            Algorithm::Sha1,
            Algorithm::Sha256,
            Algorithm::Sha384,
            Algorithm::Sha512,
            Algorithm::Sha3_256,
            Algorithm::Sha3_384,
            Algorithm::Sha3_512,
            Algorithm::Ripemd160,
        ] {
            assert_eq!(
                Algorithm::from_name(&case_hash_method(&algorithm)),
                algorithm
            );
        }
    }
}
