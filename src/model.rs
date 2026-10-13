// SPDX-License-Identifier: EUPL-1.2
use std::collections::{BTreeMap, HashMap};

use chrono::Local;
use serde::de::Error as _;
use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::VERSION;
use crate::algorithm::Algorithm;
use crate::encoding::{DigestEncoding, decode_digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructionInfo {
    username: String,
    version_information: String,
    construction_date: serde_json::Value,
    operating_system: String,
}

impl Default for ConstructionInfo {
    fn default() -> Self {
        let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let operating_system = format!("{}:unknown", std::env::consts::OS);
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string();

        Self {
            username,
            version_information: VERSION.to_string(),
            construction_date: serde_json::Value::String(timestamp),
            operating_system,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DigestResult {
    pub digests: BTreeMap<Algorithm, String>,
}

impl DigestResult {
    /// Add a digest value that is either hexadecimal or base64 encoded; it is
    /// stored as canonical lowercase hexadecimal.
    pub fn add_digest(&mut self, algorithm: Algorithm, digest: &str) {
        let hex_digest = decode_digest(&algorithm, digest);
        self.digests.insert(algorithm, hex_digest);
    }

    pub fn add_digest_name(&mut self, algorithm: &str, digest: &str) {
        self.add_digest(Algorithm::from_name(algorithm), digest);
    }

    pub fn matches(&self, other: &Self) -> bool {
        let mut found_common = false;

        for (algorithm, digest) in &self.digests {
            if let Some(other_digest) = other.digests.get(algorithm) {
                found_common = true;
                if digest != other_digest {
                    return false;
                }
            }
        }

        found_common
    }

    pub fn equals_subset_of(&self, other: &Self) -> bool {
        self.digests
            .iter()
            .all(|(algorithm, hex)| other.digests.get(algorithm) == Some(hex))
    }

    pub fn algorithms(&self) -> Vec<String> {
        self.digests.keys().map(ToString::to_string).collect()
    }

    pub fn algorithms_enum(&self) -> Vec<Algorithm> {
        self.digests.keys().cloned().collect()
    }
}

impl Serialize for DigestResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        EncodedDigestResult::new(self, DigestEncoding::Hex).serialize(serializer)
    }
}

/// Serializable view on a [`DigestResult`] that renders its digests in a
/// specific encoding.
pub struct EncodedDigestResult<'a> {
    digests: &'a DigestResult,
    encoding: DigestEncoding,
}

impl<'a> EncodedDigestResult<'a> {
    pub fn new(digests: &'a DigestResult, encoding: DigestEncoding) -> Self {
        Self { digests, encoding }
    }
}

impl Serialize for EncodedDigestResult<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let digests = &self.digests.digests;
        let mut seq = serializer.serialize_seq(Some(digests.len()))?;
        for (algorithm, hex_digest) in digests {
            let digest = self.encoding.render(hex_digest);
            seq.serialize_element(&format!("{algorithm}:{digest}"))?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DigestResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Vec::<String>::deserialize(deserializer)?;
        let mut result = Self::default();
        for digest in raw {
            let (algorithm, value) = digest
                .split_once(':')
                .ok_or_else(|| D::Error::custom(format!("Invalid digest format: {digest}")))?;
            result.add_digest_name(algorithm, value);
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirHasherResult {
    #[serde(rename = "constructionInfo")]
    pub construction_info: ConstructionInfo,
    pub content: HashMap<String, DigestResult>,
}

/// Serializable view on a [`DirHasherResult`] that renders its digests in a
/// specific encoding.
pub struct EncodedDirHasherResult<'a> {
    result: &'a DirHasherResult,
    encoding: DigestEncoding,
}

impl<'a> EncodedDirHasherResult<'a> {
    pub fn new(result: &'a DirHasherResult, encoding: DigestEncoding) -> Self {
        Self { result, encoding }
    }
}

impl Serialize for EncodedDirHasherResult<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("DirHasherResult", 2)?;
        state.serialize_field("constructionInfo", &self.result.construction_info)?;
        state.serialize_field(
            "content",
            &EncodedContent {
                content: &self.result.content,
                encoding: self.encoding,
            },
        )?;
        state.end()
    }
}

struct EncodedContent<'a> {
    content: &'a HashMap<String, DigestResult>,
    encoding: DigestEncoding,
}

impl Serialize for EncodedContent<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.content.len()))?;
        for (path, digests) in self.content {
            map.serialize_entry(path, &EncodedDigestResult::new(digests, self.encoding))?;
        }
        map.end()
    }
}

#[derive(Debug, Clone)]
pub struct IndexedEntry {
    pub path: String,
    pub digests: DigestResult,
}

pub type IndexedResult = HashMap<String, IndexedEntry>;
