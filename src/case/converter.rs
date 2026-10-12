// SPDX-License-Identifier: EUPL-1.2
//! Convert hashtree results to CASE format
//!
//! Transforms the internal DirHasherResult format into CASE ontology objects.

use std::time::SystemTime;

use crate::error::AppResult;
use crate::model::DirHasherResult;

use super::bundle::CaseBundle;
use super::ontology::{CaseAction, CaseFile, CaseHash, CasePerson, CaseTool};
use super::serializer::{
    serialize_action, serialize_bundle, serialize_file, serialize_person, serialize_tool,
};

/// Convert a DirHasherResult to a CASE Bundle
pub fn convert_to_case(result: &DirHasherResult) -> AppResult<CaseBundle> {
    let mut bundle = CaseBundle::new();

    // Get current user for performer
    let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    // Create tool object
    let tool = CaseTool::new(
        "case:tool-hashtree-1",
        "hashtree",
        crate::VERSION.to_string(),
        "hash-calculator",
    );
    bundle.add_object(serialize_tool(&tool));

    // Create person object
    let person = CasePerson::new("case:identity-operator", &username);
    bundle.add_object(serialize_person(&person));

    // Collect file IDs for bundle
    let mut file_ids = Vec::new();

    // Convert each file
    for (file_path, digest_result) in &result.content {
        let file_id = format!("case:file-{}", file_ids.len() + 1);
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
            let hash = CaseHash::new(algorithm.oldstyle_label(), hex_digest.clone());
            case_file.hashes.push(hash);
        }

        bundle.add_object(serialize_file(&case_file));
    }

    // Create investigative action
    let action = CaseAction::new(
        "case:action-hash-1",
        &username,
        SystemTime::now(),
        "case:tool-hashtree-1",
        "case:bundle-results",
    );
    bundle.add_object(serialize_action(&action));

    // Create results bundle
    bundle.add_object(serialize_bundle("case:bundle-results", &file_ids));

    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::Algorithm;
    use crate::model::{DigestResult, DirHasherResult};

    #[test]
    fn convert_to_case_creates_bundle_with_objects() -> AppResult<()> {
        let mut result = DirHasherResult::default();
        let mut digests = DigestResult::default();
        digests.add_digest(Algorithm::Sha256, "abc123");
        result.content.insert("test.txt".to_string(), digests);

        let bundle = convert_to_case(&result)?;

        // Should have: tool, person, file, action, bundle
        assert_eq!(bundle.graph.len(), 5);

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
            obj.get("@type").and_then(|t| t.as_str()) == Some("investigation:InvestigativeAction")
        });

        assert!(action_found, "Action object should be present");
        Ok(())
    }

    #[test]
    fn convert_to_case_includes_bundle_object() -> AppResult<()> {
        let result = DirHasherResult::default();
        let bundle = convert_to_case(&result)?;

        let bundle_found = bundle
            .graph
            .iter()
            .any(|obj| obj.get("@type").and_then(|t| t.as_str()) == Some("core:Bundle"));

        assert!(bundle_found, "Bundle object should be present");
        Ok(())
    }
}
