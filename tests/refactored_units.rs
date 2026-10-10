// SPDX-License-Identifier: EUPL-1.2
use std::fs;

use hashtree_rs::algorithm::Algorithm;
use hashtree_rs::hashing::hash_file;
use hashtree_rs::model::{DigestResult, DirHasherResult};
use hashtree_rs::verify::{index_result, missing, not_intersect};

#[test]
fn digest_result_matches_on_common_algorithm() {
    let mut left = DigestResult::default();
    left.add_digest(Algorithm::Sha1, "aaaaaaaa");
    left.add_digest(Algorithm::Md5, "bbbbbbbb");

    let mut right = DigestResult::default();
    right.add_digest(Algorithm::Sha1, "aaaaaaaa");

    assert!(left.matches(&right));
    assert!(right.matches(&left));
}

#[test]
fn digest_result_without_common_algorithm_does_not_match() {
    let mut left = DigestResult::default();
    left.add_digest(Algorithm::Sha1, "aaaaaaaa");

    let mut right = DigestResult::default();
    right.add_digest(Algorithm::Md5, "aaaaaaaa");

    assert!(!left.matches(&right));
}

#[test]
fn hash_file_computes_known_md5_and_sha256() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    let file_path = temp.path().join("sample.txt");
    fs::write(&file_path, b"abc").expect("fixture file should be written");

    let digests = hash_file(&file_path, &[Algorithm::Md5, Algorithm::Sha256])
        .expect("hashing should succeed");

    assert_eq!(
        digests.digests.get(&Algorithm::Md5).map(String::as_str),
        Some("900150983cd24fb0d6963f7d28e17f72")
    );
    assert_eq!(
        digests.digests.get(&Algorithm::Sha256).map(String::as_str),
        Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn verify_difference_helpers_return_expected_sets() {
    let mut left = DirHasherResult::default();
    let mut left_digest = DigestResult::default();
    left_digest.add_digest(Algorithm::Sha256, "aaaa");
    left.content.insert("a.txt".to_string(), left_digest);

    let mut right = DirHasherResult::default();
    let mut right_digest = DigestResult::default();
    right_digest.add_digest(Algorithm::Sha256, "bbbb");
    right.content.insert("a.txt".to_string(), right_digest);

    let mut right_extra = DigestResult::default();
    right_extra.add_digest(Algorithm::Sha256, "cccc");
    right.content.insert("b.txt".to_string(), right_extra);

    let left_index = index_result(&left, false);
    let right_index = index_result(&right, false);

    let diff = not_intersect(&left_index, &right_index);
    assert_eq!(diff.len(), 2);

    let left_missing = missing(&left_index, &right_index);
    assert_eq!(left_missing.len(), 1);

    let right_missing = missing(&right_index, &left_index);
    assert_eq!(right_missing.len(), 2);
}
