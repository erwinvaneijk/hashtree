// SPDX-License-Identifier: EUPL-1.2
//! CASE (Cyber-investigation Analysis Standard Expression) ontology support
//!
//! Enables export of file hash digests in JSON-LD format compliant with the CASE ontology
//! specification. See <https://caseontology.org/> for more information.

pub mod bundle;
pub mod converter;
pub mod ontology;
pub mod serializer;

pub use bundle::CaseBundle;
pub use converter::convert_to_case;
pub use ontology::{CaseAction, CaseFile, CaseHash, CaseTool};
pub use serializer::serialize_case;
