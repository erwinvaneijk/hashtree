// SPDX-License-Identifier: EUPL-1.2
//! Encoding of digest values on output and decoding of digest values on input.
//!
//! Internally every digest is kept as a canonical lowercase hexadecimal string.
//! A digest is only rendered in another encoding when it is written out (to a
//! manifest or to a report), and digest values that are read back in are
//! normalised to hexadecimal again, regardless of the encoding they were
//! stored in.

use base64::alphabet;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, PAD};
use base64::engine::{DecodePaddingMode, Engine as _};

use crate::algorithm::Algorithm;
use crate::util::{normalize_hex, split_hex};

/// Digests shorter than this are always rendered as hexadecimal, because
/// base64 only starts to pay off for the longer digests.
pub const MIN_BASE64_DIGEST_BITS: usize = 256;

/// Engine used to produce base64 output: standard alphabet, with padding.
const BASE64_OUTPUT: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, PAD);

/// Padding is optional while reading, so both padded and unpadded digests work.
const LENIENT: GeneralPurposeConfig =
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);
const BASE64_STANDARD_INPUT: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, LENIENT);
const BASE64_URL_SAFE_INPUT: GeneralPurpose = GeneralPurpose::new(&alphabet::URL_SAFE, LENIENT);

/// The encoding used when digests are written out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DigestEncoding {
    #[default]
    Hex,
    Base64,
}

impl DigestEncoding {
    /// Select the encoding based on the `--base64` command line flag.
    pub fn from_base64_flag(base64: bool) -> Self {
        if base64 { Self::Base64 } else { Self::Hex }
    }

    /// Render a canonical hexadecimal digest in this encoding.
    ///
    /// Digests of fewer than [`MIN_BASE64_DIGEST_BITS`] bits stay hexadecimal,
    /// even when base64 output was requested.
    pub fn render(self, hex_digest: &str) -> String {
        match self {
            Self::Hex => hex_digest.to_string(),
            Self::Base64 => encode_base64(hex_digest).unwrap_or_else(|| hex_digest.to_string()),
        }
    }

    /// Render a digest for textual output, grouping hexadecimal digits in
    /// blocks of `group_size`. Base64 values are compact enough already and are
    /// therefore never grouped.
    pub fn render_grouped(self, hex_digest: &str, group_size: usize) -> String {
        match self {
            Self::Hex => split_hex(hex_digest, group_size),
            Self::Base64 => {
                encode_base64(hex_digest).unwrap_or_else(|| split_hex(hex_digest, group_size))
            }
        }
    }
}

/// Encode a hexadecimal digest as base64, unless it is too short or not
/// actually hexadecimal.
fn encode_base64(hex_digest: &str) -> Option<String> {
    if hex_digest.len() * 4 < MIN_BASE64_DIGEST_BITS {
        return None;
    }

    let bytes = hex::decode(hex_digest).ok()?;
    Some(BASE64_OUTPUT.encode(bytes))
}

/// Normalise a digest value that is either hexadecimal or base64 into the
/// canonical lowercase hexadecimal representation.
///
/// The digest length of `algorithm` disambiguates the two encodings: the
/// hexadecimal and the base64 form of a digest never have the same length.
/// Values that are neither are kept as their hexadecimal characters, which
/// preserves the lenient behaviour of older manifests.
pub fn decode_digest(algorithm: &Algorithm, value: &str) -> String {
    let compact: String = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();

    if is_hex(&compact) && has_hex_length_of(algorithm, compact.len()) {
        return compact.to_ascii_lowercase();
    }

    if let Some(hex_digest) = decode_base64(algorithm, &compact) {
        return hex_digest;
    }

    normalize_hex(value)
}

fn is_hex(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn has_hex_length_of(algorithm: &Algorithm, length: usize) -> bool {
    match algorithm.digest_length_bytes() {
        Some(digest_bytes) => length == digest_bytes * 2,
        None => length.is_multiple_of(2),
    }
}

fn decode_base64(algorithm: &Algorithm, value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    let bytes = BASE64_STANDARD_INPUT
        .decode(value)
        .or_else(|_| BASE64_URL_SAFE_INPUT.decode(value))
        .ok()?;

    match algorithm.digest_length_bytes() {
        Some(digest_bytes) if bytes.len() != digest_bytes => None,
        _ => Some(hex::encode(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::{DigestEncoding, decode_digest};
    use crate::algorithm::Algorithm;

    const SHA256_HEX: &str = "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e";
    const SHA256_BASE64: &str = "pZGm1Av0IEBKARczz7exkNYsZb8LzaMrV7J32a2fFG4=";

    #[test]
    fn base64_is_used_for_digests_of_at_least_256_bits() {
        assert_eq!(DigestEncoding::Base64.render(SHA256_HEX), SHA256_BASE64);
    }

    #[test]
    fn short_digests_stay_hexadecimal_even_in_base64_mode() {
        let md5_hex = "900150983cd24fb0d6963f7d28e17f72";
        assert_eq!(DigestEncoding::Base64.render(md5_hex), md5_hex);
        assert_eq!(DigestEncoding::Hex.render(SHA256_HEX), SHA256_HEX);
    }

    #[test]
    fn grouping_applies_to_hexadecimal_output_only() {
        let md5_hex = "900150983cd24fb0d6963f7d28e17f72";
        assert_eq!(
            DigestEncoding::Hex.render_grouped(md5_hex, 4),
            "9001 5098 3cd2 4fb0 d696 3f7d 28e1 7f72"
        );
        assert_eq!(
            DigestEncoding::Base64.render_grouped(SHA256_HEX, 4),
            SHA256_BASE64
        );
    }

    #[test]
    fn decodes_both_encodings_into_canonical_hex() {
        assert_eq!(decode_digest(&Algorithm::Sha256, SHA256_HEX), SHA256_HEX);
        assert_eq!(decode_digest(&Algorithm::Sha256, SHA256_BASE64), SHA256_HEX);
        assert_eq!(
            decode_digest(&Algorithm::Sha256, SHA256_HEX.to_ascii_uppercase().as_str()),
            SHA256_HEX
        );
    }

    #[test]
    fn decodes_unpadded_and_url_safe_base64() {
        let unpadded = SHA256_BASE64.trim_end_matches('=');
        assert_eq!(decode_digest(&Algorithm::Sha256, unpadded), SHA256_HEX);

        let url_safe = SHA256_BASE64.replace('+', "-").replace('/', "_");
        assert_eq!(decode_digest(&Algorithm::Sha256, &url_safe), SHA256_HEX);
    }

    #[test]
    fn decodes_grouped_hexadecimal_from_old_style_manifests() {
        assert_eq!(
            decode_digest(&Algorithm::Md5, "9001 5098 3cd2 4fb0 d696 3f7d 28e1 7f72"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn values_that_are_neither_encoding_keep_their_hex_characters() {
        assert_eq!(decode_digest(&Algorithm::Sha256, "abc123"), "abc123");
        assert_eq!(decode_digest(&Algorithm::Sha256, ""), "");
    }

    #[test]
    fn roundtrips_every_supported_digest_length() {
        for (algorithm, digest_bytes) in [
            (Algorithm::Md5, 16),
            (Algorithm::Sha1, 20),
            (Algorithm::Sha256, 32),
            (Algorithm::Sha384, 48),
            (Algorithm::Sha512, 64),
            (Algorithm::Sha3_512, 64),
        ] {
            let hex_digest = "ab".repeat(digest_bytes);
            let rendered = DigestEncoding::Base64.render(&hex_digest);
            assert_eq!(
                decode_digest(&algorithm, &rendered),
                hex_digest,
                "{algorithm} should round trip"
            );
        }
    }
}
