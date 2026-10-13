// SPDX-License-Identifier: EUPL-1.2
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Algorithm {
    Md2,
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Sha3_256,
    Sha3_384,
    Sha3_512,
    Other(String),
}

impl Algorithm {
    pub fn from_name(value: &str) -> Self {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "md2" => Self::Md2,
            "md5" => Self::Md5,
            "sha" | "sha1" | "sha-1" => Self::Sha1,
            "sha256" | "sha-256" => Self::Sha256,
            "sha384" | "sha-384" => Self::Sha384,
            "sha512" | "sha-512" => Self::Sha512,
            "sha3" | "sha3-256" | "sha3256" => Self::Sha3_256,
            "sha3-384" | "sha3384" => Self::Sha3_384,
            "sha3-512" | "sha3512" => Self::Sha3_512,
            _ => Self::Other(normalized),
        }
    }

    pub fn from_oldstyle_prefix(prefix: &str) -> Option<Self> {
        match prefix.trim().to_ascii_uppercase().as_str() {
            "MD5" => Some(Self::Md5),
            "SHA" | "SHA-1" => Some(Self::Sha1),
            "SHA256" | "SHA-256" => Some(Self::Sha256),
            _ => None,
        }
    }

    pub fn oldstyle_label(&self) -> String {
        match self {
            Self::Md5 => "MD5".to_string(),
            Self::Sha1 => "SHA-1".to_string(),
            Self::Sha256 => "SHA-256".to_string(),
            Self::Md2 => "MD2".to_string(),
            Self::Sha384 => "SHA-384".to_string(),
            Self::Sha512 => "SHA-512".to_string(),
            Self::Sha3_256 => "SHA3-256".to_string(),
            Self::Sha3_384 => "SHA3-384".to_string(),
            Self::Sha3_512 => "SHA3-512".to_string(),
            Self::Other(value) => value.to_ascii_uppercase(),
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// Length of the digest in bytes, or `None` for an unknown algorithm.
    pub fn digest_length_bytes(&self) -> Option<usize> {
        match self {
            Self::Md2 | Self::Md5 => Some(16),
            Self::Sha1 => Some(20),
            Self::Sha256 | Self::Sha3_256 => Some(32),
            Self::Sha384 | Self::Sha3_384 => Some(48),
            Self::Sha512 | Self::Sha3_512 => Some(64),
            Self::Other(_) => None,
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Md2 => write!(f, "md2"),
            Self::Md5 => write!(f, "md5"),
            Self::Sha1 => write!(f, "sha-1"),
            Self::Sha256 => write!(f, "sha-256"),
            Self::Sha384 => write!(f, "sha-384"),
            Self::Sha512 => write!(f, "sha-512"),
            Self::Sha3_256 => write!(f, "sha3-256"),
            Self::Sha3_384 => write!(f, "sha3-384"),
            Self::Sha3_512 => write!(f, "sha3-512"),
            Self::Other(value) => write!(f, "{value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Algorithm;

    #[test]
    fn parses_aliases_and_formats_canonical_name() {
        assert_eq!(Algorithm::from_name("SHA1"), Algorithm::Sha1);
        assert_eq!(Algorithm::from_name("sha256"), Algorithm::Sha256);
        assert_eq!(Algorithm::Sha256.to_string(), "sha-256");
    }

    #[test]
    fn unknown_algorithms_become_other() {
        let algorithm = Algorithm::from_name("whirlpool");
        assert_eq!(algorithm.to_string(), "whirlpool");
        assert!(!algorithm.is_supported());
        assert_eq!(algorithm.digest_length_bytes(), None);
    }

    #[test]
    fn digest_lengths_match_the_algorithms() {
        assert_eq!(Algorithm::Md5.digest_length_bytes(), Some(16));
        assert_eq!(Algorithm::Sha1.digest_length_bytes(), Some(20));
        assert_eq!(Algorithm::Sha256.digest_length_bytes(), Some(32));
        assert_eq!(Algorithm::Sha3_384.digest_length_bytes(), Some(48));
        assert_eq!(Algorithm::Sha512.digest_length_bytes(), Some(64));
    }

    #[test]
    fn oldstyle_prefix_parsing_supports_legacy_sha_alias() {
        assert_eq!(
            Algorithm::from_oldstyle_prefix("SHA"),
            Some(Algorithm::Sha1)
        );
        assert_eq!(
            Algorithm::from_oldstyle_prefix("SHA-256"),
            Some(Algorithm::Sha256)
        );
    }
}
