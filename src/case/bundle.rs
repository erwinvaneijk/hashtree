// SPDX-License-Identifier: EUPL-1.2
//! CASE Bundle construction
//!
//! A CASE Bundle wraps all investigative objects (files, hashes, tools, actions)
//! in a standardized JSON-LD structure.

use serde_json::{Map, Value, json};

/// Prefixes of the CASE/UCO 1.5.0 namespaces used in the JSON-LD output
pub const UCO_CONTEXT: [(&str, &str); 8] = [
    (
        "action",
        "https://ontology.unifiedcyberontology.org/uco/action/",
    ),
    (
        "case-investigation",
        "https://ontology.caseontology.org/case/investigation/",
    ),
    (
        "core",
        "https://ontology.unifiedcyberontology.org/uco/core/",
    ),
    (
        "identity",
        "https://ontology.unifiedcyberontology.org/uco/identity/",
    ),
    (
        "observable",
        "https://ontology.unifiedcyberontology.org/uco/observable/",
    ),
    (
        "tool",
        "https://ontology.unifiedcyberontology.org/uco/tool/",
    ),
    (
        "types",
        "https://ontology.unifiedcyberontology.org/uco/types/",
    ),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
];

/// CASE JSON-LD Bundle
#[derive(Debug, Clone)]
pub struct CaseBundle {
    /// JSON-LD @context defining namespace URIs
    pub context: Map<String, Value>,
    /// Array of CASE objects in @graph
    pub graph: Vec<Value>,
}

impl CaseBundle {
    /// Create a new CASE bundle with the CASE/UCO ontology context
    pub fn new() -> Self {
        let context = UCO_CONTEXT
            .iter()
            .map(|(prefix, namespace)| (prefix.to_string(), Value::String(namespace.to_string())))
            .collect();

        Self {
            context,
            graph: Vec::new(),
        }
    }

    /// Add an object to the bundle's graph
    pub fn add_object(&mut self, obj: Value) {
        self.graph.push(obj);
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> Value {
        json!({
            "@context": Value::Object(self.context.clone()),
            "@graph": self.graph
        })
    }
}

impl Default for CaseBundle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_bundle_with_ontology_context() {
        let bundle = CaseBundle::new();

        assert!(bundle.context.contains_key("observable"));
        assert!(bundle.context.contains_key("types"));
        assert!(bundle.context.contains_key("case-investigation"));
        assert!(bundle.context.contains_key("core"));
        assert_eq!(
            bundle.context.get("observable").and_then(Value::as_str),
            Some("https://ontology.unifiedcyberontology.org/uco/observable/")
        );
    }

    #[test]
    fn serializes_to_valid_json_ld() {
        let bundle = CaseBundle::new();
        let json = bundle.to_json();

        assert!(json.get("@context").is_some());
        assert!(json.get("@graph").is_some());
    }
}
