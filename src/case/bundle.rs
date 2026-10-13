// SPDX-License-Identifier: EUPL-1.2
//! CASE Bundle construction
//!
//! A CASE Bundle wraps all investigative objects (files, hashes, tools, actions)
//! in a standardized JSON-LD structure.

use serde_json::{Map, Value, json};

/// CASE JSON-LD Bundle
#[derive(Debug, Clone)]
pub struct CaseBundle {
    /// JSON-LD @context defining namespace URIs
    pub context: Map<String, Value>,
    /// Array of CASE objects in @graph
    pub graph: Vec<Value>,
}

impl CaseBundle {
    /// Create a new CASE bundle with standard ontology context
    pub fn new() -> Self {
        let mut context = Map::new();

        // Standard RDF/CASE namespaces
        context.insert(
            "rdf".to_string(),
            Value::String("http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string()),
        );
        context.insert(
            "observable".to_string(),
            Value::String("https://ontology.caseontology.org/observable/".to_string()),
        );
        context.insert(
            "types".to_string(),
            Value::String("https://ontology.caseontology.org/types/".to_string()),
        );
        context.insert(
            "investigation".to_string(),
            Value::String("https://ontology.caseontology.org/investigation/".to_string()),
        );
        context.insert(
            "action".to_string(),
            Value::String("https://ontology.caseontology.org/action/".to_string()),
        );
        context.insert(
            "tool".to_string(),
            Value::String("https://ontology.caseontology.org/tool/".to_string()),
        );
        context.insert(
            "core".to_string(),
            Value::String("https://ontology.caseontology.org/core/".to_string()),
        );
        context.insert(
            "identity".to_string(),
            Value::String("https://ontology.caseontology.org/identity/".to_string()),
        );

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
        assert!(bundle.context.contains_key("investigation"));
        assert!(bundle.context.contains_key("core"));
    }

    #[test]
    fn serializes_to_valid_json_ld() {
        let bundle = CaseBundle::new();
        let json = bundle.to_json();

        assert!(json.get("@context").is_some());
        assert!(json.get("@graph").is_some());
    }
}
