use std::collections::BTreeMap;

use chrono::Local;
use serde::de::Error as _;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::VERSION;
use crate::algorithm::Algorithm;
use crate::util::normalize_hex;

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
    pub fn add_digest(&mut self, algorithm: Algorithm, hex_digest: &str) {
        let hex_digest = normalize_hex(hex_digest);
        self.digests.insert(algorithm, hex_digest);
    }

    pub fn add_digest_name(&mut self, algorithm: &str, hex_digest: &str) {
        self.add_digest(Algorithm::from_name(algorithm), hex_digest);
    }

    pub fn matches(&self, other: &Self) -> bool {
        let common: Vec<&Algorithm> = self
            .digests
            .keys()
            .filter(|name| other.digests.contains_key(*name))
            .collect();

        if common.is_empty() {
            return false;
        }

        common
            .into_iter()
            .all(|algorithm| self.digests.get(algorithm) == other.digests.get(algorithm))
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
        let mut seq = serializer.serialize_seq(Some(self.digests.len()))?;
        for (algorithm, hex_digest) in &self.digests {
            seq.serialize_element(&format!("{algorithm}:{hex_digest}"))?;
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
    pub content: BTreeMap<String, DigestResult>,
}

#[derive(Debug, Clone)]
pub struct IndexedEntry {
    pub path: String,
    pub digests: DigestResult,
}

pub type IndexedResult = BTreeMap<String, IndexedEntry>;
